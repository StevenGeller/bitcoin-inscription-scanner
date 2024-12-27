use crate::config::Config;
use super::error::{NodeError, Result};
use bitcoin::{Block, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::str::FromStr;

pub struct NodeClient {
    client: Client,
    semaphore: Arc<Semaphore>,
}

impl NodeClient {
    pub fn new(config: &Config) -> Result<Self> {
        let auth = Auth::UserPass(
            config.node.rpc_user.clone(),
            config.node.rpc_password.clone(),
        );
        
        let client = Client::new(&config.node.rpc_url, auth)
            .map_err(|e| NodeError::ConnectionError(e.to_string()))?;
        
        Ok(Self {
            client,
            semaphore: Arc::new(Semaphore::new(config.node.max_concurrent_requests)),
        })
    }

    pub async fn get_block(&self, hash: &BlockHash) -> Result<Block> {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            NodeError::ConnectionError(format!("Failed to acquire semaphore: {}", e))
        })?;

        let rpc_hash = bitcoincore_rpc::bitcoin::BlockHash::from_str(&hash.to_string())
            .map_err(|e| NodeError::ConnectionError(format!("Failed to convert hash: {}", e)))?;

        // Get block as hex string
        let block_hex = hex::decode(
            self.client
                .get_block_hex(&rpc_hash)
                .map_err(|e| NodeError::RpcError(e))?
        ).map_err(|e| NodeError::ConnectionError(format!("Failed to decode hex: {}", e)))?;
        bitcoin::consensus::encode::deserialize(&block_hex)
            .map_err(|e| NodeError::ConnectionError(format!("Failed to deserialize block: {}", e)))
    }

    pub async fn get_block_count(&self) -> Result<u64> {
        self.client
            .get_block_count()
            .map_err(|e| NodeError::RpcError(e))
    }

    pub async fn get_block_hash(&self, height: u64) -> Result<BlockHash> {
        let rpc_hash = self.client
            .get_block_hash(height)
            .map_err(|e| NodeError::RpcError(e))?;

        BlockHash::from_str(&rpc_hash.to_string())
            .map_err(|e| NodeError::ConnectionError(format!("Failed to convert hash: {}", e)))
    }

    #[allow(dead_code)]
    pub async fn get_best_block_hash(&self) -> Result<BlockHash> {
        let rpc_hash = self.client
            .get_best_block_hash()
            .map_err(|e| NodeError::RpcError(e))?;

        BlockHash::from_str(&rpc_hash.to_string())
            .map_err(|e| NodeError::ConnectionError(format!("Failed to convert hash: {}", e)))
    }
}
