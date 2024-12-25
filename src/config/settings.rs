use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
    pub storage: StorageConfig,
    pub processing: ProcessingConfig,
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub rpc_url: String,
    pub rpc_user: String,
    pub rpc_password: String,
    pub max_concurrent_requests: usize,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub image_dir: PathBuf,
    pub text_log: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct ProcessingConfig {
    pub parallel_blocks: usize,
    pub batch_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                rpc_url: "http://127.0.0.1:8332".to_string(),
                rpc_user: "user".to_string(),
                rpc_password: "password".to_string(),
                max_concurrent_requests: 16,
            },
            storage: StorageConfig {
                image_dir: PathBuf::from("./data/images"),
                text_log: PathBuf::from("./data/inscriptions.log"),
            },
            processing: ProcessingConfig {
                parallel_blocks: 8,
                batch_size: 1000,
            },
        }
    }
}