/// 统一初始化API模块
/// 提供Builder模式的一站式初始化接口
use crate::{
    Result,
    automation::{AutoExtractor, AutoIndexer, AutomationConfig, AutomationManager, IndexerConfig},
    embedding::{EmbeddingClient, EmbeddingConfig},
    events::EventBus,
    filesystem::CortexFilesystem,
    llm::LLMClient,
    session::{SessionConfig, SessionManager},
    vector_store::{QdrantVectorStore, VectorStore},
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// 🎯 一站式初始化cortex-mem，包含自动化功能
pub struct CortexMemBuilder {
    data_dir: PathBuf,
    embedding_config: Option<EmbeddingConfig>,
    qdrant_config: Option<crate::config::QdrantConfig>,
    llm_client: Option<Arc<dyn LLMClient>>,
    automation_config: AutomationConfig,
    session_config: SessionConfig,
}

impl CortexMemBuilder {
    /// 创建新的构建器
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            embedding_config: None,
            qdrant_config: None,
            llm_client: None,
            automation_config: AutomationConfig::default(),
            session_config: SessionConfig::default(),
        }
    }

    /// 配置Embedding服务
    pub fn with_embedding(mut self, config: EmbeddingConfig) -> Self {
        self.embedding_config = Some(config);
        self
    }

    /// 配置Qdrant向量数据库
    pub fn with_qdrant(mut self, config: crate::config::QdrantConfig) -> Self {
        self.qdrant_config = Some(config);
        self
    }

    /// 配置LLM客户端
    pub fn with_llm(mut self, llm_client: Arc<dyn LLMClient>) -> Self {
        self.llm_client = Some(llm_client);
        self
    }

    /// 🆕 配置自动化行为
    pub fn with_automation(mut self, config: AutomationConfig) -> Self {
        self.automation_config = config;
        self
    }

    /// 配置会话管理
    pub fn with_session_config(mut self, config: SessionConfig) -> Self {
        self.session_config = config;
        self
    }

    /// 🎯 构建完整的cortex-mem实例
    pub async fn build(self) -> Result<CortexMem> {
        info!(
            "Building Cortex Memory with automation enabled: {}",
            self.automation_config.auto_index || self.automation_config.auto_extract
        );

        // 1. 初始化文件系统
        let filesystem = Arc::new(CortexFilesystem::new(
            self.data_dir.to_string_lossy().as_ref(),
        ));
        filesystem.initialize().await?;
        info!("Filesystem initialized at: {:?}", self.data_dir);

        // 2. 初始化Embedding客户端（可选）
        let embedding = if let Some(cfg) = self.embedding_config {
            match EmbeddingClient::new(cfg) {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    warn!("Failed to create embedding client: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 3. 初始化Qdrant向量存储（可选）
        let vector_store: Option<Arc<dyn VectorStore>> = if let Some(ref cfg) = self.qdrant_config {
            match QdrantVectorStore::new(cfg).await {
                Ok(store) => {
                    info!("Qdrant vector store connected: {}", cfg.url);
                    Some(Arc::new(store))
                }
                Err(e) => {
                    warn!("Failed to connect to Qdrant, vector search disabled: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 4. 创建事件总线
        let (event_bus, event_rx) = EventBus::new();
        let event_bus = Arc::new(event_bus);

        // 5. 创建SessionManager（带事件总线）
        let session_manager = if let Some(ref llm) = self.llm_client {
            SessionManager::with_llm_and_events(
                filesystem.clone(),
                self.session_config,
                llm.clone(),
                event_bus.as_ref().clone(),
            )
        } else {
            SessionManager::with_event_bus(
                filesystem.clone(),
                self.session_config,
                event_bus.as_ref().clone(),
            )
        };

        // 6. 创建AutomationManager（如果配置了）
        let automation_handle = if self.automation_config.auto_index
            || self.automation_config.auto_extract
        {
            // 需要同时有embedding和qdrant_config才能创建AutoIndexer
            if let (Some(emb), Some(cfg)) = (&embedding, &self.qdrant_config) {
                // 🔧 移除ref
                // 创建AutoIndexer
                let indexer_config = IndexerConfig {
                    auto_index: true,
                    batch_size: 10,
                    async_index: false,
                };

                // 重新创建QdrantVectorStore用于AutoIndexer
                let qdrant_store = QdrantVectorStore::new(cfg).await?;
                let indexer = Arc::new(AutoIndexer::new(
                    filesystem.clone(),
                    emb.clone(),
                    Arc::new(qdrant_store),
                    indexer_config,
                ));

                // 创建AutoExtractor（如果有LLM）
                let extractor = if let (Some(llm), true) =
                    (&self.llm_client, self.automation_config.auto_extract)
                {
                    Some(Arc::new(AutoExtractor::new(
                        filesystem.clone(),
                        llm.clone(),
                        Default::default(),
                    )))
                } else {
                    None
                };

                // 启动AutomationManager
                let manager = AutomationManager::new(indexer, extractor, self.automation_config);

                // 在后台启动
                info!("Starting AutomationManager in background");
                let handle = tokio::spawn(async move {
                    if let Err(e) = manager.start(event_rx).await {
                        error!("AutomationManager failed: {}", e);
                    }
                });

                Some(handle)
            } else {
                warn!("Automation disabled: missing embedding or qdrant configuration");
                None
            }
        } else {
            None
        };

        Ok(CortexMem {
            filesystem,
            session_manager: Arc::new(RwLock::new(session_manager)),
            embedding,
            vector_store,
            llm_client: self.llm_client,
            event_bus,
            automation_handle,
        })
    }
}

/// CortexMem实例 - 统一封装所有功能
pub struct CortexMem {
    pub filesystem: Arc<CortexFilesystem>,
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub embedding: Option<Arc<EmbeddingClient>>,
    pub vector_store: Option<Arc<dyn VectorStore>>,
    pub llm_client: Option<Arc<dyn LLMClient>>,
    event_bus: Arc<EventBus>,
    automation_handle: Option<tokio::task::JoinHandle<()>>,
}

impl CortexMem {
    /// 获取SessionManager
    pub fn session_manager(&self) -> Arc<RwLock<SessionManager>> {
        self.session_manager.clone()
    }

    /// 获取文件系统
    pub fn filesystem(&self) -> Arc<CortexFilesystem> {
        self.filesystem.clone()
    }

    /// 获取Embedding客户端
    pub fn embedding(&self) -> Option<Arc<EmbeddingClient>> {
        self.embedding.clone()
    }

    /// 获取向量存储
    pub fn vector_store(&self) -> Option<Arc<dyn VectorStore>> {
        self.vector_store.clone()
    }

    /// 获取LLM客户端
    pub fn llm_client(&self) -> Option<Arc<dyn LLMClient>> {
        self.llm_client.clone()
    }

    /// 优雅关闭
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down CortexMem...");

        if let Some(handle) = self.automation_handle {
            handle.abort();
            info!("Automation manager stopped");
        }

        Ok(())
    }
}
