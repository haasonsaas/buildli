pub mod cli;
pub mod config;
pub mod embeddings;
pub mod indexer;
pub mod query;
pub mod server;
pub mod utils;
pub mod vector;

pub use config::{Config, ConfigManager};

#[derive(Debug, thiserror::Error)]
pub enum BuildliError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Indexing error: {0}")]
    Indexing(String),
    
    #[error("Query error: {0}")]
    Query(String),
    
    #[error("Embedding error: {0}")]
    Embedding(String),
    
    #[error("Vector store error: {0}")]
    VectorStore(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BuildliError>;