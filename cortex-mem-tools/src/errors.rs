use thiserror::Error;

/// Common error types for memory tools
#[derive(Debug, Error)]
pub enum MemoryToolsError {
    /// Invalid input provided
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Runtime error during operation
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Memory not found
    #[error("Memory not found: {0}")]
    MemoryNotFound(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Core memory error
    #[error("Core memory error: {0}")]
    Core(#[from] anyhow::Error),
}

impl From<cortex_mem_core::error::MemoryError> for MemoryToolsError {
    fn from(err: cortex_mem_core::error::MemoryError) -> Self {
        MemoryToolsError::Core(anyhow::anyhow!("Core error: {}", err))
    }
}

/// Result type for memory tools operations
pub type MemoryToolsResult<T> = Result<T, MemoryToolsError>;
