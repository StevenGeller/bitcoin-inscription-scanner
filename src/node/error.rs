use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("RPC error: {0}")]
    RpcError(#[from] bitcoincore_rpc::Error),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;
