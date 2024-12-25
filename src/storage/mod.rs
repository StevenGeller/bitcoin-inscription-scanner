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
}