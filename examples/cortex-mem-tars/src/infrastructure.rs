use anyhow::{Context, Result};
use cortex_mem_config::Config;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// Infrastructure manager - manages memory operations and configuration
pub struct Infrastructure {
    operations: Arc<MemoryOperations>,
    config: Config,
}

impl Infrastructure {
    /// Create new infrastructure from configuration
    pub async fn new(config: Config) -> Result<Self> {
        log::info!("正在初始化基础设施...");

        // Get data directory from config or use default
        let data_dir = std::env::var("CORTEX_DATA_DIR")
            .unwrap_or_else(|_| {
                directories::ProjectDirs::from("com", "cortex-mem", "tars")
                    .map(|dirs| dirs.data_dir().to_string_lossy().to_string())
                    .unwrap_or_else(|| "./.cortex".to_string())
            });

        // Initialize MemoryOperations from data directory
        let operations = MemoryOperations::from_data_dir(&data_dir)
            .await
            .context("Failed to initialize MemoryOperations")?;

        log::info!("基础设施初始化成功");

        Ok(Self {
            operations: Arc::new(operations),
            config,
        })
    }

    /// Get memory operations (V2 API)
    pub fn operations(&self) -> &Arc<MemoryOperations> {
        &self.operations
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}
