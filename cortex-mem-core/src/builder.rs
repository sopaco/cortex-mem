/// 统一初始化API模块
/// 提供Builder模式的一站式初始化接口
use crate::{
    Result,
    automation::{AutoIndexer, AutomationConfig, AutomationManager, IndexerConfig},
    embedding::{EmbeddingClient, EmbeddingConfig},
    events::EventBus,
    filesystem::CortexFilesystem,
    llm::LLMClient,
    memory_event_coordinator::{CoordinatorConfig, MemoryEventCoordinator},
    session::{SessionConfig, SessionManager},
    vector_store::{QdrantVectorStore, VectorStore},
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// 🎯 一站式初始化cortex-mem，包含自动化功能
pub struct CortexMemBuilder {
    data_dir: PathBuf,
    embedding_config: Option<EmbeddingConfig>,
    qdrant_config: Option<crate::config::QdrantConfig>,
    llm_client: Option<Arc<dyn LLMClient>>,
    session_config: SessionConfig,
    /// 事件协调器配置
    coordinator_config: Option<CoordinatorConfig>,
}

impl CortexMemBuilder {
    /// 创建新的构建器
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            embedding_config: None,
            qdrant_config: None,
            llm_client: None,
            session_config: SessionConfig::default(),
            coordinator_config: None,
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

    /// 配置会话管理
    pub fn with_session_config(mut self, config: SessionConfig) -> Self {
        self.session_config = config;
        self
    }

    /// 配置事件协调器
    pub fn with_coordinator_config(mut self, config: CoordinatorConfig) -> Self {
        self.coordinator_config = Some(config);
        self
    }

    /// 🎯 构建完整的cortex-mem实例
    pub async fn build(self) -> Result<CortexMem> {
        info!("Building Cortex Memory with incremental update support");

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
        // 同时保留具体类型（供 MemoryEventCoordinator 使用）和 trait object（供 VectorStore 接口使用）
        let (qdrant_store_typed, vector_store): (Option<Arc<QdrantVectorStore>>, Option<Arc<dyn VectorStore>>) =
            if let Some(ref cfg) = self.qdrant_config {
                match QdrantVectorStore::new(cfg).await {
                    Ok(store) => {
                        info!("Qdrant vector store connected: {}", cfg.url);
                        let typed = Arc::new(store);
                        let dyn_store: Arc<dyn VectorStore> = typed.clone();
                        (Some(typed), Some(dyn_store))
                    }
                    Err(e) => {
                        warn!("Failed to connect to Qdrant, vector search disabled: {}", e);
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

        // 4. 创建事件总线
        let (event_bus, event_rx) = EventBus::new();
        let event_bus = Arc::new(event_bus);

        // 5. 创建 MemoryEventCoordinator（如果配置了所有必需组件）
        let (coordinator_handle, memory_event_tx) =
            if let (Some(llm), Some(emb), Some(qdrant_store)) =
                (&self.llm_client, &embedding, &qdrant_store_typed)
            {
                let qdrant_store = qdrant_store.clone();

                let config = self.coordinator_config.unwrap_or_default();
                let (coordinator, tx, rx) = MemoryEventCoordinator::new_with_config(
                    filesystem.clone(),
                    llm.clone(),
                    emb.clone(),
                    qdrant_store,
                    config,
                );

                // 启动事件协调器
                let handle = tokio::spawn(coordinator.start(rx));
                info!("✅ MemoryEventCoordinator started for incremental updates");

                (Some(handle), Some(tx))
            } else {
                warn!("MemoryEventCoordinator disabled: missing LLM, embedding, or vector store");
                (None, None)
            };

        // 6. 创建SessionManager（带 memory_event_tx）
        // Clone the sender so we can keep one for CortexMem's public getter.
        let memory_event_tx_for_session = memory_event_tx.clone();
        let session_manager = if let Some(tx) = memory_event_tx_for_session {
            // 使用 MemoryEventCoordinator 的事件通道
            if let Some(ref llm) = self.llm_client {
                SessionManager::with_llm_and_events(
                    filesystem.clone(),
                    self.session_config,
                    llm.clone(),
                    event_bus.as_ref().clone(),
                )
                .with_memory_event_tx(tx)
            } else {
                SessionManager::with_event_bus(
                    filesystem.clone(),
                    self.session_config,
                    event_bus.as_ref().clone(),
                )
                .with_memory_event_tx(tx)
            }
        } else {
            // 回退到旧的事件总线机制
            if let Some(ref llm) = self.llm_client {
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
            }
        };

        // 7. 启动 AutomationManager（监听 MessageAdded 事件触发实时 L2 向量索引）
        let (automation_handle, automation_tx_handle) = if let (Some(emb), Some(qdrant_store)) = (&embedding, &qdrant_store_typed) {
            let indexer = Arc::new(AutoIndexer::new(
                filesystem.clone(),
                emb.clone(),
                qdrant_store.clone(),
                IndexerConfig::default(),
            ));
            let automation_manager = if let Some(ref tx) = memory_event_tx {
                AutomationManager::with_memory_events(
                    indexer,
                    AutomationConfig {
                        auto_index: true,
                        index_on_message: true,
                        ..AutomationConfig::default()
                    },
                    tx.clone(),
                )
            } else {
                AutomationManager::new(
                    indexer,
                    AutomationConfig {
                        auto_index: true,
                        index_on_message: true,
                        ..AutomationConfig::default()
                    },
                )
            };
            let tx_handle = automation_manager.memory_event_tx_handle();
            let handle = tokio::spawn(async move {
                if let Err(e) = automation_manager.start(event_rx).await {
                    tracing::error!("AutomationManager failed: {}", e);
                }
            });
            info!("✅ AutomationManager started (real-time L2 indexing on MessageAdded)");
            (Some(handle), Some(tx_handle))
        } else {
            // No embedding/qdrant → drop the event_rx; AutomationManager is not needed
            drop(event_rx);
            warn!("AutomationManager not started: Qdrant or Embedding not configured");
            (None, None)
        };

        info!("✅ CortexMem initialized successfully");

        Ok(CortexMem {
            filesystem,
            session_manager: Arc::new(RwLock::new(session_manager)),
            embedding,
            vector_store,
            llm_client: self.llm_client,
            event_bus,
            qdrant_store_typed,
            memory_event_tx,
            coordinator_handle,
            automation_handle,
            automation_tx_handle,
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
    #[allow(dead_code)]
    event_bus: Arc<EventBus>,
    /// Typed Qdrant store (for consumers that need Arc<QdrantVectorStore>)
    qdrant_store_typed: Option<Arc<QdrantVectorStore>>,
    /// Memory event sender (for VectorSearchEngine / AutomationManager wiring)
    memory_event_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::memory_events::MemoryEvent>>,
    /// MemoryEventCoordinator 的后台任务句柄
    coordinator_handle: Option<tokio::task::JoinHandle<()>>,
    /// AutomationManager 的后台任务句柄
    automation_handle: Option<tokio::task::JoinHandle<()>>,
    /// AutomationManager 的 memory_event_tx 句柄（用于 tenant 切换时更新 coordinator sender）
    automation_tx_handle: Option<Arc<tokio::sync::RwLock<Option<tokio::sync::mpsc::UnboundedSender<crate::memory_events::MemoryEvent>>>>>,
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

    /// 获取具体类型的 Qdrant 存储（供需要 Arc<QdrantVectorStore> 的消费者使用）
    pub fn qdrant_store(&self) -> Option<Arc<QdrantVectorStore>> {
        self.qdrant_store_typed.clone()
    }

    /// 获取 memory event sender（用于 VectorSearchEngine / AutomationManager 接入遗忘机制）
    pub fn memory_event_tx(
        &self,
    ) -> Option<tokio::sync::mpsc::UnboundedSender<crate::memory_events::MemoryEvent>> {
        self.memory_event_tx.clone()
    }

    /// 获取 AutomationManager 的 tx 句柄（用于 tenant 切换时替换 coordinator sender）
    pub fn automation_tx_handle(
        &self,
    ) -> Option<Arc<tokio::sync::RwLock<Option<tokio::sync::mpsc::UnboundedSender<crate::memory_events::MemoryEvent>>>>> {
        self.automation_tx_handle.clone()
    }

    /// 优雅关闭
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down CortexMem...");

        // 停止 AutomationManager
        if let Some(handle) = self.automation_handle {
            handle.abort();
            info!("AutomationManager stopped");
        }

        // 停止 MemoryEventCoordinator
        if let Some(handle) = self.coordinator_handle {
            handle.abort();
            info!("MemoryEventCoordinator stopped");
        }

        Ok(())
    }
}