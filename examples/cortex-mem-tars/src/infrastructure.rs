use anyhow::{Context, Result};
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// Infrastructure for cortex-mem-tars using V2 architecture
pub struct Infrastructure {
    operations: Arc<MemoryOperations>,
    data_dir: String,
}

impl Infrastructure {
    /// Create new infrastructure with data directory
    pub async fn new(data_dir: &str) -> Result<Self> {
        log::info!("Initializing infrastructure with data directory: {}", data_dir);

        // Initialize MemoryOperations from data directory
        let operations = MemoryOperations::from_data_dir(data_dir)
            .await
            .context("Failed to initialize MemoryOperations")?;

        log::info!("Infrastructure initialized successfully");

        Ok(Self {
            operations: Arc::new(operations),
            data_dir: data_dir.to_string(),
        })
    }

    /// Get MemoryOperations
    pub fn operations(&self) -> &Arc<MemoryOperations> {
        &self.operations
    }

    /// Get data directory
    pub fn data_dir(&self) -> &str {
        &self.data_dir
    }
}
