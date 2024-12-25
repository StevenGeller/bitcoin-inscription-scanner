// main.rs
//
// Bitcoin Inscription Scanner - Core Application Entry Point
//
// This module serves as the main entry point for the Bitcoin inscription scanner.
// It orchestrates the interaction between different components and implements the
// core scanning logic.
//
// Architecture Overview:
// - Uses a modular design with clear separation of concerns
// - Implements concurrent block processing for performance
// - Provides both live and mock scanning modes for testing
// - Handles graceful error recovery and logging
//
// Key Components:
// - Block Processing Pipeline: Fetches -> Parses -> Stores
// - Mock Mode: Generates test inscriptions without requiring a Bitcoin node
// - Error Handling: Comprehensive error handling with detailed logging
//
// Performance Considerations:
// - Batch processing to optimize node communication
// - Parallel inscription parsing using rayon
// - Efficient memory management for large blocks
//
// Threading Model:
// - Main async runtime using tokio
// - Parallel block processing using rayon
// - Connection pooling for RPC calls

mod config;
mod node;
mod parser;
mod storage;
mod utils;

use clap::Parser;
use std::path::PathBuf;
use tokio;
use log::{info, error, warn};
use bitcoin::{Block, Transaction, TxOut, blockdata::script::Builder};
use bitcoin::block::{Header, Version};
use bitcoin::hash_types::TxMerkleNode;
use bitcoin::hashes::Hash;
use bitcoin::pow::CompactTarget;
use bitcoin::blockdata::opcodes::all::{OP_IF, OP_ENDIF};
use bitcoin::opcodes::{OP_0, OP_FALSE};
use bitcoin::script::PushBytesBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to configuration file (default: config.toml)
    #[clap(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Start scanning from this block height
    /// If not specified, starts from genesis block
    #[clap(long)]
    start_block: Option<u64>,

    /// Resume scanning from last processed block
    /// Requires previous scan data in storage
    #[clap(long)]
    resume: bool,

    /// Enable verbose logging for debugging
    /// Includes detailed inscription parsing information
    #[clap(short, long)]
    verbose: bool,

    /// Run in mock mode without Bitcoin node
    /// Generates test inscriptions for development
    #[clap(long)]
    mock: bool,
}

/// Creates a mock block containing a test inscription
/// 
/// This function generates a valid Bitcoin block structure with a single
/// transaction containing an inscription. Used for testing the scanner
/// without requiring a Bitcoin node connection.
///
/// Parameters:
/// - height: Block height, used to generate unique content
///
/// Returns:
/// - Block: A complete Bitcoin block with test inscription
///
/// Technical Details:
/// - Creates valid script following ordinal inscription format
/// - Uses standard OP_FALSE OP_IF pattern
/// - Includes proper MIME type and content
/// - Sets valid block header fields
fn create_mock_inscription_block(height: u64) -> Block {
    // Create inscription script following ordinal protocol
    // Format: OP_FALSE OP_IF <content-type> OP_0 <content> OP_ENDIF
    let mut content_type = PushBytesBuf::new();
    content_type.extend_from_slice(b"text/plain;charset=utf-8").unwrap();

    let mut content = PushBytesBuf::new();
    content.extend_from_slice(format!("Hello from block {}!", height).as_bytes()).unwrap();

    // Build complete inscription script
    let script = Builder::new()
        .push_opcode(OP_FALSE)  // Standard inscription marker
        .push_opcode(OP_IF)     // Start conditional
        .push_slice(&content_type)
        .push_opcode(OP_0)      // Content type separator
        .push_slice(&content)
        .push_opcode(OP_ENDIF)  // End conditional
        .into_script();

    // Create transaction with inscription output
    let tx = Transaction {
        version: 2,
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![],
        output: vec![TxOut {
            value: 0,            // Inscriptions typically use zero-value outputs
            script_pubkey: script,
        }],
    };

    // Generate deterministic block header
    let zeros = [0u8; 32];
    let prev_blockhash = bitcoin::BlockHash::from_slice(&zeros).unwrap();
    let merkle_root = TxMerkleNode::from_slice(&zeros).unwrap();

    Block {
        header: Header {
            version: Version::ONE,
            prev_blockhash,
            merkle_root,
            time: height as u32,  // Use height as timestamp for deterministic testing
            bits: CompactTarget::from_consensus(0x1d00ffff),
            nonce: 0,
        },
        txdata: vec![tx],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments and initialize logging
    let args = Args::parse();
    env_logger::Builder::from_default_env()
        .filter_level(if args.verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info })
        .init();

    info!("Starting Bitcoin Inscription Scanner");

    // Load and validate configuration
    info!("Loading configuration from {}", args.config.display());
    let config = config::load_config(args.config)?;

    // Initialize system components
    let node_client = if args.mock {
        info!("Running in mock mode");
        None
    } else {
        info!("Connecting to Bitcoin node at {}", config.node.rpc_url);
        match node::NodeClient::new(&config) {
            Ok(client) => Some(client),
            Err(e) => {
                error!("Failed to connect to Bitcoin node: {}", e);
                error!("Please check your Bitcoin node is running and the credentials are correct");
                error!("RPC URL: {}", config.node.rpc_url);
                error!("You can use --mock flag to run with mock data for testing");
                return Err(e.into());
            }
        }
    };

    // Initialize parser with batch size from config
    let parser = parser::ParallelParser::new(config.processing.batch_size);
    
    info!("Initializing storage");
    let storage = storage::Storage::new(
        config.storage.image_dir.clone(),
        config.storage.text_log.clone(),
    )?;

    // Determine scanning start position
    let start_block = if args.resume {
        warn!("Resume functionality not yet implemented, starting from block 0");
        0
    } else {
        args.start_block.unwrap_or(0)
    };

    // Get target end block (latest block or mock range)
    let latest_block = if let Some(client) = &node_client {
        info!("Checking Bitcoin node connection...");
        match client.get_block_count().await {
            Ok(count) => count,
            Err(e) => {
                error!("Failed to get latest block height: {}", e);
                error!("Please check your Bitcoin node is running and accessible");
                return Err(e.into());
            }
        }
    } else {
        // In mock mode, process 10 blocks for testing
        start_block + 10
    };

    info!("Starting scan from block {} to {}", start_block, latest_block);

    // Main scanning loop - processes blocks in batches
    let mut current_block = start_block;
    while current_block < latest_block {
        // Calculate batch size for this iteration
        let end_block = std::cmp::min(
            current_block + config.processing.batch_size as u64,
            latest_block,
        );

        info!("Processing blocks {} to {}", current_block, end_block);

        // Fetch blocks - either from node or generate mock blocks
        let blocks = if let Some(client) = &node_client {
            let mut blocks = Vec::new();
            for height in current_block..end_block {
                match client.get_block_hash(height).await {
                    Ok(hash) => {
                        match client.get_block(&hash).await {
                            Ok(block) => blocks.push(block),
                            Err(e) => {
                                error!("Failed to fetch block {}: {}", height, e);
                                continue;  // Skip failed blocks but continue processing
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get hash for block {}: {}", height, e);
                        continue;
                    }
                }
            }
            blocks
        } else {
            // Generate mock blocks for testing
            (current_block..end_block)
                .map(create_mock_inscription_block)
                .collect()
        };

        // Process blocks in parallel using rayon
        let inscriptions = parser.process_blocks(blocks);
        info!("Found {} inscriptions in blocks {} to {}", 
            inscriptions.len(), current_block, end_block);

        // Store discovered inscriptions
        for inscription in inscriptions {
            if let Err(e) = storage.store_inscription(&inscription).await {
                error!("Failed to store inscription {}: {}", inscription.txid, e);
            }
        }

        info!("Completed blocks {} to {}", current_block, end_block);
        current_block = end_block;
    }

    info!("Scanning completed");
    Ok(())
}