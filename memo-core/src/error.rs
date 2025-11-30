use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Vector store error: {0}")]
    VectorStore(#[from] qdrant_client::QdrantError),
    
    #[error("LLM error: {0}")]
    LLM(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Memory not found: {id}")]
    NotFound { id: String },
    
    #[error("Invalid memory action: {action}")]
    InvalidAction { action: String },
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Embedding error: {0}")]
    Embedding(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, MemoryError>;

impl MemoryError {
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }
    
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::Validation(msg.into())
    }
    
    pub fn embedding<S: Into<String>>(msg: S) -> Self {
        Self::Embedding(msg.into())
    }
    
    pub fn parse<S: Into<String>>(msg: S) -> Self {
        Self::Parse(msg.into())
    }
}