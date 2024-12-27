mod image;
mod text;

use crate::parser::Inscription;
use std::path::PathBuf;
use thiserror::Error;
use serde_json;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Image error: {0}")]
    ImageError(String),
    
    #[error("Text error: {0}")]
    TextError(String),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Hash error: {0}")]
    HashError(#[from] bitcoin::hashes::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct Storage {
    image_storage: image::ImageStorage,
    text_storage: text::TextStorage,
}

impl Storage {
    pub fn new(image_dir: PathBuf, text_log: PathBuf) -> Result<Self> {
        Ok(Self {
            image_storage: image::ImageStorage::new(image_dir)?,
            text_storage: text::TextStorage::new(text_log)?,
        })
    }

    #[allow(dead_code)]
pub async fn store_inscription(&self, inscription: &Inscription) -> Result<()> {
    match &inscription.content {
        crate::parser::InscriptionType::Image { mime_type, data } => {
            self.image_storage.store(inscription.txid, mime_type, data)
        }
        crate::parser::InscriptionType::Text(text) => {
            self.text_storage.store(inscription.txid, text)
        }
        crate::parser::InscriptionType::Unknown(_) => Ok(()),
    }
}

pub async fn store_text(&self, text: String) -> Result<()> {
    // Generate a unique identifier using timestamp and text hash
    use std::time::{SystemTime, UNIX_EPOCH};
    use bitcoin::hashes::{sha256, Hash, HashEngine};
    
    let _timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let mut engine = sha256::Hash::engine();
    engine.input(text.as_bytes());
    let hash = sha256::Hash::from_engine(engine);
    
    // Create a unique txid-like identifier
    let hash_bytes = hash.to_byte_array();
    let pseudo_txid = bitcoin::Txid::from_slice(&hash_bytes)
        .map_err(|e| StorageError::HashError(e))?;
    
    self.text_storage.store(pseudo_txid, &text)
}
}
