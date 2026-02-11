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
    /// NOTE: This creates a NON-TENANT instance for global infrastructure
    /// Each bot should create its own tenant-isolated MemoryOperations
    pub async fn new(config: Config) -> Result<Self> {
        log::info!("正在初始化基础设施...");

        // Get data directory from config (which handles priorities correctly)
        let data_dir = config.cortex.data_dir();
        log::info!("使用数据目录: {}", data_dir);

        // Initialize MemoryOperations WITHOUT tenant isolation
        // This is only for infrastructure-level operations
        // Each bot will create its own tenant-isolated operations
        let operations = MemoryOperations::from_data_dir(&data_dir)
            .await
            .context("Failed to initialize MemoryOperations")?;

        log::info!("基础设施初始化成功（非租户模式，仅用于基础设施）");

        Ok(Self {
            operations: Arc::new(operations),
            config,
        })
    }

    /// Get memory operations (V2 API)
    /// WARNING: This returns NON-TENANT operations
    /// For bot-specific operations, create tenant-isolated operations
    pub fn operations(&self) -> &Arc<MemoryOperations> {
        &self.operations
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}
