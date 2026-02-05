use anyhow::{Context, Result};
use cortex_mem_config::Config;
use cortex_mem_core::memory::MemoryManager;
use cortex_mem_rig::llm::OpenAILLMClient;
use cortex_mem_rig::vector_store::qdrant::QdrantVectorStore;
use std::sync::Arc;

/// 基础设施管理器，负责初始化和管理 LLM 客户端、向量存储和记忆管理器
pub struct Infrastructure {
    pub memory_manager: Arc<MemoryManager>,
    pub config: Config,
}

impl Infrastructure {
    /// 创建新的基础设施
    pub async fn new(config: Config) -> Result<Self> {
        log::info!("正在初始化基础设施...");

        // 初始化 LLM 客户端
        let llm_client = OpenAILLMClient::new(&config.llm, &config.embedding)
            .context("无法初始化 LLM 客户端")?;
        log::info!("LLM 客户端初始化成功");

        // 初始化向量存储
        let vector_store = QdrantVectorStore::new(&config.qdrant)
            .await
            .context("无法连接到 Qdrant 向量存储")?;
        log::info!("Qdrant 向量存储连接成功");

        // 初始化记忆管理器
        let memory_config = config.memory.clone();
        let memory_manager = Arc::new(MemoryManager::new(
            Box::new(vector_store),
            Box::new(llm_client),
            memory_config,
        ));
        log::info!("记忆管理器初始化成功");

        log::info!("基础设施初始化完成");

        Ok(Self {
            memory_manager,
            config,
        })
    }

    /// 获取记忆管理器
    pub fn memory_manager(&self) -> &Arc<MemoryManager> {
        &self.memory_manager
    }

    /// 获取配置
    pub fn config(&self) -> &Config {
        &self.config
    }
}