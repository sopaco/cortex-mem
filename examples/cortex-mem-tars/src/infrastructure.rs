use anyhow::{Context, Result};
use cortex_mem_config::Config;
use cortex_mem_core::{CortexMem, CortexMemBuilder, AutomationConfig, EmbeddingConfig, QdrantConfig};
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// Infrastructure manager - manages memory operations and configuration
pub struct Infrastructure {
    operations: Arc<MemoryOperations>,
    cortex: Arc<CortexMem>,  // 🆕 统一自动索引实例
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
        log::info!("Qdrant 配置: url={}, collection={}, embedding_dim={:?}", 
            config.qdrant.url, config.qdrant.collection_name, config.qdrant.embedding_dim);

        // Get LLM configuration
        let llm_config = cortex_mem_core::llm::LLMConfig {
            api_base_url: config.llm.api_base_url.clone(),
            api_key: config.llm.api_key.clone(),
            model_efficient: config.llm.model_efficient.clone(),
            temperature: 0.1,
            max_tokens: 4096,
        };
        let llm_client: Arc<dyn cortex_mem_core::llm::LLMClient> = 
            Arc::new(cortex_mem_core::llm::LLMClientImpl::new(llm_config)?);

        // 🆕 使用CortexMemBuilder初始化（包含自动索引）
        log::info!("正在初始化CortexMem（包含自动索引和自动提取）...");
        let cortex = CortexMemBuilder::new(&data_dir)
            .with_embedding(EmbeddingConfig {
                api_base_url: config.embedding.api_base_url.clone(),
                api_key: config.embedding.api_key.clone(),
                model_name: config.embedding.model_name.clone(),
                batch_size: 10,
                timeout_secs: 30,
            })
            .with_qdrant(QdrantConfig {
                url: config.qdrant.url.clone(),
                collection_name: config.qdrant.collection_name.clone(),
                embedding_dim: config.qdrant.embedding_dim,
                timeout_secs: 60,
            })
            .with_llm(llm_client.clone())
            .with_automation(AutomationConfig {
                auto_index: true,       // ✅ 自动索引
                auto_extract: true,     // ✅ 自动提取
                index_on_message: false, // 批处理模式
                index_on_close: true,   // 会话关闭时索引
                index_batch_delay: 2,
            })
            .build()
            .await
            .context("Failed to build CortexMem")?;

        log::info!("✅ CortexMem初始化成功（自动索引和自动提取已启用）");

        // 为了保持向后兼容，仍然创建MemoryOperations（使用global租户）
        let operations = MemoryOperations::new(
            &data_dir,
            "global",  // Use "global" as tenant ID for infrastructure
            llm_client,
            &config.qdrant.url,
            &config.qdrant.collection_name,
            &config.embedding.api_base_url,
            &config.embedding.api_key,
            &config.embedding.model_name,
            config.qdrant.embedding_dim,
        )
        .await
        .context("Failed to initialize MemoryOperations")?;

        log::info!("基础设施初始化成功（global 租户模式，仅用于基础设施）");

        Ok(Self {
            operations: Arc::new(operations),
            cortex: Arc::new(cortex),  // 🆕 保存CortexMem实例
            config,
        })
    }

    /// Get memory operations (V2 API)
    /// WARNING: This returns GLOBAL tenant operations
    /// For bot-specific operations, create tenant-isolated operations
    pub fn operations(&self) -> &Arc<MemoryOperations> {
        &self.operations
    }
    
    /// 🆕 Get CortexMem instance (for unified automation)
    pub fn cortex(&self) -> &Arc<CortexMem> {
        &self.cortex
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}
