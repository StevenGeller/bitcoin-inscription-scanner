// inscription.rs
//
// Bitcoin Inscription Detection and Parsing
//
// This module implements the core inscription detection and parsing logic.
// It follows the ordinal inscription protocol specification and handles
// both text and image inscriptions.
//
// Protocol Details:
// - Inscriptions use OP_FALSE OP_IF ... OP_ENDIF pattern
// - Content type and content are separated by OP_0
// - Supports standard MIME types for content identification
//
// Performance Considerations:
// - Efficient script parsing using iterators
// - Minimal memory allocations
// - Early exit on non-inscription scripts
//
// Error Handling:
// - Robust parsing of malformed scripts
// - Graceful handling of invalid UTF-8
// - Detailed logging for debugging

use bitcoin::{Script, Transaction};
use bitcoin::blockdata::script::Instruction;
use bitcoin::blockdata::opcodes::all;
use bitcoin::opcodes::{OP_0, OP_FALSE};
use serde::{Serialize, Deserialize};
use std::iter::Peekable;
use std::str::FromStr;
use log::debug;

/// Represents different types of inscription content
/// 
/// This enum handles the various content types that can be
/// found in Bitcoin inscriptions, with specialized handling
/// for text and images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InscriptionType {
    /// Plain text inscriptions with UTF-8 encoding
    Text(String),
    
    /// Image inscriptions with MIME type and raw data
    Image { 
        mime_type: String,
        data: Vec<u8> 
    },
    
    /// Unknown content types preserved as raw bytes
    Unknown(Vec<u8>),
}

/// Represents a complete inscription found in a transaction
///
/// Contains both the transaction identifier and the parsed
/// inscription content. This structure is serializable for
/// storage and can be recreated from stored data.
#[derive(Debug, Clone)]
pub struct Inscription {
    /// Transaction ID where the inscription was found
    pub txid: bitcoin::Txid,
    
    /// Parsed inscription content
    pub content: InscriptionType,
}

// Custom serialization implementation to handle Bitcoin types
impl Serialize for Inscription {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Inscription", 2)?;
        
        // Convert Txid to string for compatibility
        state.serialize_field("txid", &self.txid.to_string())?;
        state.serialize_field("content", &self.content)?;
        state.end()
    }
}

// Custom deserialization implementation to handle Bitcoin types
impl<'de> Deserialize<'de> for Inscription {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor, MapAccess};
        use std::fmt;

        struct InscriptionVisitor;

        impl<'de> Visitor<'de> for InscriptionVisitor {
            type Value = Inscription;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Inscription")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Inscription, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut txid = None;
                let mut content = None;

                // Parse fields from map
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "txid" => {
                            let txid_str = map.next_value::<String>()?;
                            txid = Some(bitcoin::Txid::from_str(&txid_str)
                                .map_err(de::Error::custom)?);
                        }
                        "content" => {
                            content = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(&key, &["txid", "content"]));
                        }
                    }
                }

                // Ensure all required fields are present
                let txid = txid.ok_or_else(|| de::Error::missing_field("txid"))?;
                let content = content.ok_or_else(|| de::Error::missing_field("content"))?;

                Ok(Inscription { txid, content })
            }
        }

        deserializer.deserialize_map(InscriptionVisitor)
    }
}

/// Core inscription detection and parsing logic
pub struct InscriptionParser;

impl InscriptionParser {
    /// Creates a new inscription parser
    pub fn new() -> Self {
        Self
    }

    /// Parses a transaction looking for inscriptions
    ///
    /// Examines each transaction output for inscription patterns
    /// and returns the first valid inscription found.
    ///
    /// Parameters:
    /// - tx: The Bitcoin transaction to examine
    ///
    /// Returns:
    /// - Option<Inscription>: The first inscription found, if any
    pub fn parse_transaction(&self, tx: &Transaction) -> Option<Inscription> {
        debug!("Parsing transaction: {}", tx.txid());
        
        // First check inputs for coinbase inscriptions
        for (i, input) in tx.input.iter().enumerate() {
            debug!("Checking input {} of transaction {}", i, tx.txid());
            
            // Check if this is a coinbase input
            if input.previous_output.is_null() {
                debug!("Found coinbase input in tx: {}", tx.txid());
                debug!("Coinbase script: {:?}", input.script_sig);
                
                // Log raw script bytes for debugging
                if let Ok(bytes) = String::from_utf8(input.script_sig.as_bytes().to_vec()) {
                    debug!("Raw script bytes as UTF-8: {}", bytes);
                }
                
                if let Some(text) = self.extract_text_from_script(&input.script_sig) {
                    debug!("Found text in coinbase: {}", text);
                    return Some(Inscription {
                        txid: tx.txid(),
                        content: InscriptionType::Text(text),
                    });
                } else {
                    debug!("No text found in coinbase script");
                }
            }
        }

        // Then check outputs for ordinal inscriptions
        for (i, output) in tx.output.iter().enumerate() {
            debug!("Checking output {} of transaction {}", i, tx.txid());
            debug!("Script: {:?}", output.script_pubkey);
            if let Some(content) = self.parse_script(&output.script_pubkey) {
                debug!("Found inscription in transaction {} output {}", tx.txid(), i);
                return Some(Inscription {
                    txid: tx.txid(),
                    content,
                });
            }
        }
        None
    }

    /// Extracts meaningful text from a script
    fn extract_text_from_script(&self, script: &Script) -> Option<String> {
        let mut found_text = None;
        let mut push_count = 0;
        
        for instruction in script.instructions() {
            if let Ok(Instruction::PushBytes(data)) = instruction {
                push_count += 1;
                // The text is in the third push operation (OP_PUSHBYTES_69)
                if push_count == 3 {
                    debug!("Found third push data: {:?}", data.as_bytes());
                    // Convert hex to ASCII
                    let hex_str = hex::encode(data.as_bytes());
                    debug!("Hex string: {}", hex_str);
                    if let Ok(decoded) = hex::decode(&hex_str) {
                        if let Ok(text) = String::from_utf8(decoded) {
                            debug!("Decoded text: {}", text);
                            found_text = Some(text);
                        }
                    }
                }
            }
        }
        
        found_text
    }

    /// Parses a Bitcoin script looking for inscription patterns
    ///
    /// Implements the core inscription detection logic:
    /// - Looks for OP_FALSE/OP_0 OP_IF sequence
    /// - Handles both explicit and implicit zero representations
    /// - Validates complete inscription structure
    ///
    /// Parameters:
    /// - script: The Bitcoin script to parse
    ///
    /// Returns:
    /// - Option<InscriptionType>: The parsed inscription content, if found
    fn parse_script(&self, script: &Script) -> Option<InscriptionType> {
        let mut instructions = script.instructions().peekable();
        
        // Check for OP_FALSE/OP_0 OP_IF sequence
        match (instructions.next()?, instructions.next()?) {
            (Ok(first), Ok(Instruction::Op(op2))) => {
                debug!("Found first instruction: {:?} and second: {:?}", first, op2);
                
                // Check if it's either OP_FALSE or OP_0 (PushBytes([]))
                let is_false = match first {
                    Instruction::Op(op1) => op1 == OP_FALSE || op1 == OP_0,
                    Instruction::PushBytes(data) => data.as_bytes().is_empty(),
                };

                if is_false && op2 == all::OP_IF {
                    debug!("Found inscription start sequence");
                    self.parse_inscription_content(&mut instructions)
                } else {
                    debug!("Not an inscription sequence");
                    None
                }
            }
            other => {
                debug!("Invalid instruction sequence: {:?}", other);
                None
            }
        }
    }

    /// Parses the content portion of an inscription
    ///
    /// Handles the data between OP_IF and OP_ENDIF:
    /// - Extracts content type and content
    /// - Validates separators and structure
    /// - Handles different content encodings
    ///
    /// Parameters:
    /// - instructions: Iterator over remaining script instructions
    ///
    /// Returns:
    /// - Option<InscriptionType>: The parsed content if valid
    fn parse_inscription_content<'a, I>(&self, instructions: &mut Peekable<I>) -> Option<InscriptionType>
    where
        I: Iterator<Item = Result<Instruction<'a>, bitcoin::blockdata::script::Error>>
    {
        let mut content_type = Vec::new();
        let mut content = Vec::new();
        let mut reading_content_type = true;

        while let Some(Ok(instruction)) = instructions.next() {
            match instruction {
                Instruction::Op(all::OP_ENDIF) => {
                    debug!("Found OP_ENDIF, ending inscription");
                    break;
                }
                Instruction::PushBytes(data) => {
                    debug!("Found PushBytes: {:?}", data.as_bytes());
                    if reading_content_type {
                        content_type.extend_from_slice(data.as_bytes());
                        if let Some(Ok(instruction)) = instructions.peek() {
                            let is_zero = match instruction {
                                Instruction::Op(op) => *op == OP_0 || *op == OP_FALSE,
                                Instruction::PushBytes(data) => data.as_bytes().is_empty(),
                            };
                            if is_zero {
                                debug!("Found OP_0/OP_FALSE, switching to content");
                                reading_content_type = false;
                                instructions.next();
                            }
                        }
                    } else {
                        content.extend_from_slice(data.as_bytes());
                    }
                }
                op => {
                    debug!("Skipping instruction: {:?}", op);
                    continue;
                }
            }
        }

        debug!("Content type: {:?}", String::from_utf8_lossy(&content_type));
        debug!("Content: {:?}", String::from_utf8_lossy(&content));

        self.classify_inscription(content_type, content)
    }

    /// Classifies inscription content based on MIME type
    ///
    /// Determines the appropriate InscriptionType based on:
    /// - MIME type parsing
    /// - Content validation
    /// - Encoding detection
    ///
    /// Parameters:
    /// - content_type: Raw MIME type bytes
    /// - content: Raw content bytes
    ///
    /// Returns:
    /// - Option<InscriptionType>: The classified content
    fn classify_inscription(&self, content_type: Vec<u8>, content: Vec<u8>) -> Option<InscriptionType> {
        let content_type = String::from_utf8(content_type).ok()?;
        
        match content_type.as_str() {
            "text/plain;charset=utf-8" => {
                String::from_utf8(content)
                    .ok()
                    .map(InscriptionType::Text)
            }
            mime if mime.starts_with("image/") => {
                Some(InscriptionType::Image {
                    mime_type: content_type,
                    data: content,
                })
            }
            _ => Some(InscriptionType::Unknown(content))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::blockdata::script::Builder;
    use serde_json;

    #[test]
    fn test_coinbase_text_extraction() {
        let parser = InscriptionParser::new();

        // Create a transaction with a coinbase input containing the genesis block text
        let script = Builder::new()
            .push_slice(b"The Times 03/Jan/2009 Chancellor on brink of second bailout for banks")
            .into_script();

        let tx = Transaction {
            version: 1,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: bitcoin::OutPoint::null(),
                script_sig: script,
                sequence: bitcoin::Sequence::MAX,
                witness: bitcoin::Witness::default(),
            }],
            output: vec![],
        };

        let inscription = parser.parse_transaction(&tx).unwrap();
        if let InscriptionType::Text(text) = inscription.content {
            assert_eq!(text, "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks");
        } else {
            panic!("Expected text inscription from coinbase");
        }
    }

    #[test]
    fn test_inscription_parsing() {
        let parser = InscriptionParser::new();

        // Test with OP_FALSE
        let script = Builder::new()
            .push_opcode(OP_FALSE)
            .push_opcode(all::OP_IF)
            .push_slice(b"text/plain;charset=utf-8")
            .push_opcode(OP_0)
            .push_slice(b"Hello, Bitcoin!")
            .push_opcode(all::OP_ENDIF)
            .into_script();

        let tx = Transaction {
            version: 1,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![bitcoin::TxOut {
                value: 0,
                script_pubkey: script,
            }],
        };

        let inscription = parser.parse_transaction(&tx).unwrap();
        match inscription.content {
            InscriptionType::Text(text) => assert_eq!(text, "Hello, Bitcoin!"),
            _ => panic!("Expected text inscription"),
        }

        // Test with OP_0
        let script = Builder::new()
            .push_opcode(OP_0)
            .push_opcode(all::OP_IF)
            .push_slice(b"text/plain;charset=utf-8")
            .push_opcode(OP_0)
            .push_slice(b"Hello, Bitcoin!")
            .push_opcode(all::OP_ENDIF)
            .into_script();

        let tx = Transaction {
            version: 1,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![bitcoin::TxOut {
                value: 0,
                script_pubkey: script,
            }],
        };

        let inscription = parser.parse_transaction(&tx).unwrap();
        
        // Test content
        if let InscriptionType::Text(text) = &inscription.content {
            assert_eq!(text, "Hello, Bitcoin!");
        } else {
            panic!("Expected text inscription");
        }

        // Test serialization/deserialization
        let json = serde_json::to_string(&inscription).unwrap();
        let deserialized: Inscription = serde_json::from_str(&json).unwrap();
        
        // Compare txids
        assert_eq!(deserialized.txid, inscription.txid);
        
        // Compare contents
        if let (InscriptionType::Text(original), InscriptionType::Text(deserialized)) = (&inscription.content, &deserialized.content) {
            assert_eq!(original, deserialized);
        } else {
            panic!("Expected text inscriptions");
        }
    }
}
