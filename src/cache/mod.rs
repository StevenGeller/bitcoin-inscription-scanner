mod db;
mod bloom;

pub use db::CacheDb;
pub use bloom::BloomCache;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database error: {0}")]
    DbError(#[from] rocksdb::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
}

pub type Result<T> = std::result::Result<T, CacheError>;