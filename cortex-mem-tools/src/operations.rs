use crate::{errors::*, types::*};
use cortex_mem_core::{
    CortexFilesystem,
    FilesystemOperations,
    MemoryIndexManager,
    SessionConfig,
    SessionManager,
    automation::{
        AbstractConfig, AutoIndexer, AutomationConfig, AutomationManager, IndexerConfig,
        LayerGenerationConfig, LayerGenerator, OverviewConfig, SyncConfig, SyncManager,
    },
    embedding::{EmbeddingClient, EmbeddingConfig},
    events::EventBus,
    layers::manager::LayerManager,
    llm::LLMClient,
    search::VectorSearchEngine,
    vector_store::{QdrantVectorStore, VectorStore},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// High-level memory operations
///
/// All operations require:
/// - LLM client for layer generation
/// - Vector search engine for semantic search
/// - Embedding client for vectorization
pub struct MemoryOperations {
    pub(crate) filesystem: Arc<CortexFilesystem>,
    pub(crate) session_manager: Arc<RwLock<SessionManager>>,
    pub(crate) layer_manager: Arc<LayerManager>,
    pub(crate) vector_engine: Arc<VectorSearchEngine>,
    pub(crate) layer_generator: Option<Arc<LayerGenerator>>,
    pub(crate) auto_indexer: Option<Arc<AutoIndexer>>,

    // 保存组件引用以便退出时索引使用
    pub(crate) embedding_client: Arc<EmbeddingClient>,
    pub(crate) vector_store: Arc<QdrantVectorStore>,
    pub(crate) llm_client: Arc<dyn LLMClient>,

    pub(crate) default_user_id: String,
    pub(crate) default_agent_id: String,

    /// 事件发送器，用于异步触发层级生成
    pub(crate) memory_event_tx:
        Option<tokio::sync::mpsc::UnboundedSender<cortex_mem_core::memory_events::MemoryEvent>>,

    /// 事件协调器引用，用于同步等待后台处理完成
    pub(crate) event_coordinator: Option<Arc<cortex_mem_core::MemoryEventCoordinator>>,
}

impl MemoryOperations {
    /// Get the underlying filesystem
    pub fn filesystem(&self) -> &Arc<CortexFilesystem> {
        &self.filesystem
    }

    /// Get the vector search engine
    pub fn vector_engine(&self) -> &Arc<VectorSearchEngine> {
        &self.vector_engine
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &Arc<RwLock<SessionManager>> {
        &self.session_manager
    }

    /// Get the layer generator (for manual layer generation on exit)
    pub fn layer_generator(&self) -> Option<&Arc<LayerGenerator>> {
        self.layer_generator.as_ref()
    }

    /// Get the auto indexer (for manual indexing on exit)
    pub fn auto_indexer(&self) -> Option<&Arc<AutoIndexer>> {
        self.auto_indexer.as_ref()
    }

    /// Get the default user ID
    pub fn default_user_id(&self) -> &str {
        &self.default_user_id
    }

    /// Get the default agent ID
    pub fn default_agent_id(&self) -> &str {
        &self.default_agent_id
    }

    /// Get the memory event sender (for triggering processing)
    pub fn memory_event_tx(
        &self,
    ) -> Option<&tokio::sync::mpsc::UnboundedSender<cortex_mem_core::memory_events::MemoryEvent>>
    {
        self.memory_event_tx.as_ref()
    }

    /// Get the vector store (for admin operations like prune, reindex)
    pub fn vector_store(&self) -> &Arc<QdrantVectorStore> {
        &self.vector_store
    }

    /// Create from data directory with tenant isolation, LLM support, and vector search
    ///
    /// This is the primary constructor that requires all dependencies.
    pub async fn new(
        data_dir: &str,
        tenant_id: impl Into<String>,
        llm_client: Arc<dyn LLMClient>,
        qdrant_url: &str,
        qdrant_collection: &str,
        qdrant_api_key: Option<&str>,
        embedding_api_base_url: &str,
        embedding_api_key: &str,
        embedding_model_name: &str,
        embedding_dim: Option<usize>,
        user_id: Option<String>,
        enable_intent_analysis: bool,
    ) -> Result<Self> {
        let tenant_id = tenant_id.into();
        let filesystem = Arc::new(CortexFilesystem::with_tenant(data_dir, &tenant_id));
        filesystem.initialize().await?;

        // 创建EventBus用于自动化
        let (event_bus, event_rx_main) = EventBus::new();

        // Initialize Qdrant first (needed for MemoryEventCoordinator)
        tracing::info!("Initializing Qdrant vector store: {}", qdrant_url);
        let qdrant_config = cortex_mem_core::QdrantConfig {
            url: qdrant_url.to_string(),
            collection_name: qdrant_collection.to_string(),
            embedding_dim,
            timeout_secs: 30,
            api_key: qdrant_api_key
                .map(|s| s.to_string())
                .or_else(|| std::env::var("QDRANT_API_KEY").ok()),
            tenant_id: Some(tenant_id.clone()), // 设置租户ID
        };
        let vector_store = Arc::new(QdrantVectorStore::new(&qdrant_config).await?);
        tracing::info!(
            "Qdrant connected successfully, collection: {}",
            qdrant_config.get_collection_name()
        );

        // Initialize Embedding client (needed for MemoryEventCoordinator)
        tracing::info!(
            "Initializing Embedding client with model: {}",
            embedding_model_name
        );
        let embedding_config = EmbeddingConfig {
            api_base_url: embedding_api_base_url.to_string(),
            api_key: embedding_api_key.to_string(),
            model_name: embedding_model_name.to_string(),
            batch_size: 10,
            timeout_secs: 30,
            ..EmbeddingConfig::default()
        };
        let embedding_client = Arc::new(EmbeddingClient::new(embedding_config)?);
        tracing::info!("Embedding client initialized");

        // 🔧 Fix: ensure Qdrant collection exists even when embedding_dim is not in config.
        // When embedding_dim is None, QdrantVectorStore::new skips ensure_collection.
        // We probe the real dimension by running a test embedding and create the collection.
        if embedding_dim.is_none() {
            tracing::info!("embedding_dim not configured, probing from embedding service...");
            match embedding_client.embed("probe").await {
                Ok(probe_vec) => {
                    let probed_dim = probe_vec.len();
                    tracing::info!("Probed embedding dimension: {}", probed_dim);
                    if let Err(e) = vector_store.ensure_collection_with_dim(probed_dim).await {
                        tracing::warn!("Failed to ensure collection with probed dim {}: {}", probed_dim, e);
                    } else {
                        tracing::info!("Collection ensured with probed dimension {}", probed_dim);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to probe embedding dimension, collection may not exist: {}", e
                    );
                }
            }
        }

        // Create MemoryEventCoordinator BEFORE SessionManager
        let (coordinator, memory_event_tx, event_rx) = cortex_mem_core::MemoryEventCoordinator::new(
            filesystem.clone(),
            llm_client.clone(),
            embedding_client.clone(),
            vector_store.clone(),
        );

        // 保存 coordinator 克隆用于后台任务等待
        let coordinator_clone = coordinator.clone();

        // Start the coordinator event loop in background
        tokio::spawn(coordinator.start(event_rx));
        tracing::info!("MemoryEventCoordinator started for incremental updates");

        let config = SessionConfig::default();
        // Create SessionManager with memory_event_tx for integration
        let session_manager = SessionManager::with_llm_and_events(
            filesystem.clone(),
            config,
            llm_client.clone(),
            event_bus.clone(),
        )
        .with_memory_event_tx(memory_event_tx.clone());
        let session_manager = Arc::new(RwLock::new(session_manager));

        // LLM-enabled LayerManager for high-quality L0/L1 generation
        let layer_manager = Arc::new(LayerManager::new(filesystem.clone(), llm_client.clone()));

        // Create shared MemoryIndexManager (used by VectorSearchEngine for archived filtering
        // and by MemoryCleanupService for forgetting curve evictions)
        let index_manager = Arc::new(MemoryIndexManager::new(filesystem.clone()));

        // Create vector search engine with LLM support for query rewriting.
        // Wire up:
        //   - memory_event_tx  → search hits emit MemoryAccessed events (forgetting mechanism)
        //   - index_manager    → archived memories are filtered from search results
        let vector_engine = Arc::new(
            VectorSearchEngine::with_llm(
                vector_store.clone(),
                embedding_client.clone(),
                filesystem.clone(),
                llm_client.clone(),
            )
            .with_memory_event_tx(memory_event_tx.clone())
            .with_index_manager(index_manager.clone())
            .with_intent_analysis(enable_intent_analysis),
        );
        tracing::info!("Vector search engine created with LLM, event tracking, and archived filter");

        // 使用传入的user_id。
        // 注意：不要回退到 tenant_id —— tenant_id 是用于隔离 Qdrant collection 的部署标识
        // （如 "local-XeStation_zed_agent"），不应作为用户身份存入 cortex://user/{id}/ 目录。
        // 若未显式传入 user_id，则使用稳定的默认值 "default"，确保记忆归属于一致的用户维度。
        let actual_user_id = user_id.unwrap_or_else(|| "default".to_string());

        // 创建 AutoIndexer 用于 L2 消息实时索引
        let indexer_config = IndexerConfig {
            auto_index: true,
            batch_size: 10,
            async_index: true,
        };
        let auto_indexer = Arc::new(AutoIndexer::new(
            filesystem.clone(),
            embedding_client.clone(),
            vector_store.clone(),
            indexer_config,
        ));

        // 创建 AutomationManager：仅负责 L2 消息实时索引
        // L0/L1 生成、记忆提取、向量同步均由 MemoryEventCoordinator 处理
        //
        // 使用 `with_memory_events` 路径，将 L2 索引请求路由到 MemoryEventCoordinator
        // 而不是直接调用 AutoIndexer，实现统一调度和可观测性。
        let automation_config = AutomationConfig {
            auto_index: true,
            index_on_message: true, // 消息添加时实时索引 L2
            index_batch_delay: 1,
            max_concurrent_tasks: 3,
        };
        let automation_manager = AutomationManager::with_memory_events(
            auto_indexer.clone(),
            automation_config,
            memory_event_tx.clone(),
        );

        // 启动 AutomationManager（直接消费 EventBus 事件，无需分裂转发）
        let tenant_id_for_automation = tenant_id.clone();
        tokio::spawn(async move {
            tracing::info!(
                "AutomationManager started for tenant {} (L2 message indexing)",
                tenant_id_for_automation
            );
            if let Err(e) = automation_manager.start(event_rx_main).await {
                tracing::error!("AutomationManager stopped with error: {}", e);
            }
        });

        // 创建 LayerGenerator（供 ensure_all_layers / ensure_session_layers 手动调用）
        let layer_gen_config = LayerGenerationConfig {
            batch_size: 10,
            delay_ms: 1000,
            auto_generate_on_startup: false,
            abstract_config: AbstractConfig {
                max_tokens: 400,
                max_chars: 2000,
                target_sentences: 2,
            },
            overview_config: OverviewConfig {
                max_tokens: 1500,
                max_chars: 6000,
            },
        };
        let layer_generator = Arc::new(LayerGenerator::new(
            filesystem.clone(),
            llm_client.clone(),
            layer_gen_config,
        ));

        // Auto-sync existing content to vector database (in background)
        let sync_manager = SyncManager::new(
            filesystem.clone(),
            embedding_client.clone(),
            vector_store.clone(),
            llm_client.clone(),
            SyncConfig::default(),
        );

        // Spawn background sync task
        let _fs_clone = filesystem.clone();
        tokio::spawn(async move {
            tracing::info!("Starting background sync to vector database...");
            match sync_manager.sync_all().await {
                Ok(stats) => {
                    tracing::info!(
                        "Auto-sync completed: {} files indexed, {} files skipped",
                        stats.indexed_files,
                        stats.skipped_files
                    );
                }
                Err(e) => {
                    tracing::warn!("Auto-sync failed: {}", e);
                }
            }
        });

        // Build VectorSyncManager for MemoryCleanupService.
        // The embedding client is required by the constructor but only used for Add/Update
        // operations; Delete calls (the only ones cleanup makes) don't touch it.
        let vector_sync_for_cleanup = Arc::new(
            cortex_mem_core::vector_sync_manager::VectorSyncManager::new(
                filesystem.clone(),
                embedding_client.clone(),
                vector_store.clone(),
            ),
        );

        // Launch background MemoryCleanupService (Ebbinghaus forgetting curve eviction).
        // Runs every 24 hours; removes archived memories whose strength has decayed below
        // the delete threshold and syncs deletions to Qdrant.
        {
            use cortex_mem_core::{
                memory_cleanup::{MemoryCleanupConfig, MemoryCleanupService},
                memory_index::MemoryScope,
            };

            let cleanup_svc = MemoryCleanupService::new(
                index_manager.clone(),
                MemoryCleanupConfig::default(),
                Some(vector_sync_for_cleanup),
            );
            let cleanup_user_id = actual_user_id.clone();

            tokio::spawn(async move {
                // Give the rest of the system time to finish initialising before
                // the first cleanup sweep.
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                tracing::info!("MemoryCleanupService background task started");

                let interval = std::time::Duration::from_secs(24 * 60 * 60);
                loop {
                    // Clean both User-scoped memories (preferences, entities, …)
                    // and Agent-scoped memories (cases, …).
                    match cleanup_svc
                        .run_cleanup(&MemoryScope::User, &cleanup_user_id)
                        .await
                    {
                        Ok(stats) => tracing::info!(
                            "MemoryCleanup[User]: scanned={}, archived={}, deleted={}",
                            stats.total_scanned, stats.archived, stats.deleted
                        ),
                        Err(e) => tracing::warn!("MemoryCleanup[User] failed: {}", e),
                    }
                    match cleanup_svc
                        .run_cleanup(&MemoryScope::Agent, &cleanup_user_id)
                        .await
                    {
                        Ok(stats) => tracing::info!(
                            "MemoryCleanup[Agent]: scanned={}, archived={}, deleted={}",
                            stats.total_scanned, stats.archived, stats.deleted
                        ),
                        Err(e) => tracing::warn!("MemoryCleanup[Agent] failed: {}", e),
                    }
                    tokio::time::sleep(interval).await;
                }
            });
        }

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            vector_engine,
            layer_generator: Some(layer_generator),
            auto_indexer: Some(auto_indexer),

            embedding_client,
            vector_store,
            llm_client,

            default_user_id: actual_user_id,
            default_agent_id: tenant_id.clone(),

            memory_event_tx: Some(memory_event_tx),
            event_coordinator: Some(coordinator_clone),
        })
    }

    /// Add a message to a session
    pub async fn add_message(&self, thread_id: &str, role: &str, content: &str) -> Result<String> {
        let thread_id = if thread_id.is_empty() {
            "default"
        } else {
            thread_id
        };

        let sm = self.session_manager.read().await;

        if !sm.session_exists(thread_id).await? {
            drop(sm);
            let sm = self.session_manager.write().await;
            // 🔧 使用create_session_with_ids创建session，传入默认的user_id和agent_id
            sm.create_session_with_ids(
                thread_id,
                Some(self.default_user_id.clone()),
                Some(self.default_agent_id.clone()),
            )
            .await?;
            drop(sm);
        } else {
            // 🔧 Session存在，检查并更新user_id/agent_id（兼容旧session）
            if let Ok(metadata) = sm.load_session(thread_id).await {
                let needs_update = metadata.user_id.is_none() || metadata.agent_id.is_none();

                if needs_update {
                    drop(sm);
                    let sm = self.session_manager.write().await;

                    // 重新加载并更新
                    if let Ok(mut metadata) = sm.load_session(thread_id).await {
                        if metadata.user_id.is_none() {
                            metadata.user_id = Some(self.default_user_id.clone());
                        }
                        if metadata.agent_id.is_none() {
                            metadata.agent_id = Some(self.default_agent_id.clone());
                        }
                        let _ = sm.update_session(&metadata).await;
                        tracing::info!("Updated session {} with user_id and agent_id", thread_id);
                    }
                    drop(sm);
                }
            }
        }

        let sm = self.session_manager.read().await;

        // 🔧 使用SessionManager::add_message()替代message_storage().save_message()
        // 这样可以自动触发MessageAdded事件，从而触发自动索引
        let message_role = match role {
            "user" => cortex_mem_core::MessageRole::User,
            "assistant" => cortex_mem_core::MessageRole::Assistant,
            "system" => cortex_mem_core::MessageRole::System,
            _ => cortex_mem_core::MessageRole::User,
        };

        let message = sm
            .add_message(thread_id, message_role, content.to_string())
            .await?;
        let message_uri = format!(
            "cortex://session/{}/timeline/{}/{}/{}_{}.md",
            thread_id,
            message.timestamp.format("%Y-%m"),
            message.timestamp.format("%d"),
            message.timestamp.format("%H_%M_%S"),
            &message.id[..8]
        );

        tracing::info!(
            "Added message to session {}, URI: {}",
            thread_id,
            message_uri
        );
        Ok(message_uri)
    }

    /// List sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let entries = self.filesystem.list("cortex://session").await?;

        let mut session_infos = Vec::new();
        for entry in entries {
            if entry.is_directory {
                let thread_id = entry.name;
                if let Ok(metadata) = self
                    .session_manager
                    .read()
                    .await
                    .load_session(&thread_id)
                    .await
                {
                    let status_str = match metadata.status {
                        cortex_mem_core::session::manager::SessionStatus::Active => "active",
                        cortex_mem_core::session::manager::SessionStatus::Closed => "closed",
                        cortex_mem_core::session::manager::SessionStatus::Archived => "archived",
                    };

                    session_infos.push(SessionInfo {
                        thread_id: metadata.thread_id,
                        status: status_str.to_string(),
                        message_count: 0,
                        created_at: metadata.created_at,
                        updated_at: metadata.updated_at,
                    });
                }
            }
        }

        Ok(session_infos)
    }

    /// Get session by thread_id
    pub async fn get_session(&self, thread_id: &str) -> Result<SessionInfo> {
        let sm = self.session_manager.read().await;
        let metadata = sm.load_session(thread_id).await?;

        let status_str = match metadata.status {
            cortex_mem_core::session::manager::SessionStatus::Active => "active",
            cortex_mem_core::session::manager::SessionStatus::Closed => "closed",
            cortex_mem_core::session::manager::SessionStatus::Archived => "archived",
        };

        Ok(SessionInfo {
            thread_id: metadata.thread_id,
            status: status_str.to_string(),
            message_count: 0,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
        })
    }

    /// Close session (fire-and-forget).
    ///
    /// Sends a `SessionClosed` event to `MemoryEventCoordinator` via channel.
    /// Memory extraction and L0/L1 generation happen **asynchronously** in the
    /// background; this method returns before they complete.
    ///
    /// Use `close_session_sync` in exit/shutdown flows where you need to wait.
    pub async fn close_session(&self, thread_id: &str) -> Result<()> {
        let mut sm = self.session_manager.write().await;
        sm.close_session(thread_id).await?;
        tracing::info!("Closed session: {}", thread_id);
        Ok(())
    }

    /// Close session and synchronously wait for the full processing pipeline.
    ///
    /// Blocks until:
    /// 1. Session metadata → marked closed (EventBus `SessionClosed` published)
    /// 2. LLM memory extraction from session timeline
    /// 3. user/agent memory files written
    /// 4. L0/L1 layer files generated for all affected directories
    /// 5. Session timeline synced to vector store
    ///
    /// Suitable for shutdown/exit flows. After this returns you can call
    /// `index_all_files` knowing all L0/L1 files are already on disk.
    pub async fn close_session_sync(&self, thread_id: &str) -> Result<()> {
        // 1. Mark session as closed (metadata + legacy EventBus event)
        let metadata = {
            let mut sm = self.session_manager.write().await;
            sm.close_session_metadata_only(thread_id).await?
        };

        let user_id = metadata.user_id.as_deref().unwrap_or("default");
        let agent_id = metadata.agent_id.as_deref().unwrap_or("default");

        tracing::info!(
            "Session {} marked closed, starting synchronous processing (user={}, agent={})...",
            thread_id, user_id, agent_id
        );

        // 2. Run the full processing pipeline synchronously via coordinator
        if let Some(ref coordinator) = self.event_coordinator {
            coordinator
                .process_session_closed(thread_id, user_id, agent_id)
                .await?;
            tracing::info!(
                "Session {} processing complete (memory extraction + L0/L1 generated)",
                thread_id
            );
        } else {
            tracing::warn!(
                "MemoryEventCoordinator not initialized; session {} processing skipped",
                thread_id
            );
        }

        Ok(())
    }

    /// Read file from filesystem
    pub async fn read_file(&self, uri: &str) -> Result<String> {
        let content = self.filesystem.read(uri).await?;
        Ok(content)
    }

    /// List files in directory
    pub async fn list_files(&self, uri: &str) -> Result<Vec<String>> {
        let entries = self.filesystem.list(uri).await?;
        let uris = entries.into_iter().map(|e| e.uri).collect();
        Ok(uris)
    }

    /// Delete file or directory
    pub async fn delete(&self, uri: &str) -> Result<()> {
        // First delete from vector database
        // We need to delete all 3 layers: L0, L1, L2
        let l0_id =
            cortex_mem_core::uri_to_vector_id(uri, cortex_mem_core::ContextLayer::L0Abstract);
        let l1_id =
            cortex_mem_core::uri_to_vector_id(uri, cortex_mem_core::ContextLayer::L1Overview);
        let l2_id = cortex_mem_core::uri_to_vector_id(uri, cortex_mem_core::ContextLayer::L2Detail);

        // Delete from vector store (ignore errors as vectors might not exist)
        let _ = self.vector_store.delete(&l0_id).await;
        let _ = self.vector_store.delete(&l1_id).await;
        let _ = self.vector_store.delete(&l2_id).await;

        tracing::info!(
            "Deleted vectors for URI: {} (L0: {}, L1: {}, L2: {})",
            uri,
            l0_id,
            l1_id,
            l2_id
        );

        // Then delete from filesystem
        self.filesystem.delete(uri).await?;
        tracing::info!("Deleted file: {}", uri);
        Ok(())
    }

    /// Check if file/directory exists
    pub async fn exists(&self, uri: &str) -> Result<bool> {
        let exists = self
            .filesystem
            .exists(uri)
            .await
            .map_err(ToolsError::Core)?;
        Ok(exists)
    }

    /// Generate all missing L0/L1 layer files (for calling during exit)
    ///
    /// This method scans all directories, finds those missing .abstract.md or .overview.md,
    /// and generates them in batches. Suitable for calling during application exit.
    pub async fn ensure_all_layers(&self) -> Result<cortex_mem_core::automation::GenerationStats> {
        if let Some(ref generator) = self.layer_generator {
            tracing::info!("Starting scan and generation of missing L0/L1 layer files...");
            match generator.ensure_all_layers().await {
                Ok(stats) => {
                    tracing::info!(
                        "L0/L1 layer generation completed: total={}, generated={}, failed={}",
                        stats.total,
                        stats.generated,
                        stats.failed
                    );
                    Ok(stats)
                }
                Err(e) => {
                    tracing::error!("L0/L1 layer generation failed: {}", e);
                    Err(e.into())
                }
            }
        } else {
            tracing::warn!("LayerGenerator not configured, skipping layer generation");
            Ok(cortex_mem_core::automation::GenerationStats::default())
        }
    }

    /// Generate L0/L1 layer files for a specific session
    /// # Arguments
    /// * `session_id` - Session ID
    ///
    /// # Returns
    /// Returns generation statistics
    pub async fn ensure_session_layers(
        &self,
        session_id: &str,
    ) -> Result<cortex_mem_core::automation::GenerationStats> {
        if let Some(ref generator) = self.layer_generator {
            let timeline_uri = format!("cortex://session/{}/timeline", session_id);
            tracing::info!("Generating L0/L1 layer files for session {}", session_id);

            match generator.ensure_timeline_layers(&timeline_uri).await {
                Ok(stats) => {
                    tracing::info!(
                        "Session {} L0/L1 layer generation completed: total={}, generated={}, failed={}",
                        session_id,
                        stats.total,
                        stats.generated,
                        stats.failed
                    );
                    Ok(stats)
                }
                Err(e) => {
                    tracing::error!("Session {} L0/L1 layer generation failed: {}", session_id, e);
                    Err(e.into())
                }
            }
        } else {
            tracing::warn!("LayerGenerator not configured, skipping layer generation");
            Ok(cortex_mem_core::automation::GenerationStats::default())
        }
    }

    /// Index all files to vector database (for calling during exit)
    /// This method scans all files, including newly generated .abstract.md and .overview.md,
    /// and indexes them to the vector database. Suitable for calling during application exit.
    pub async fn index_all_files(&self) -> Result<cortex_mem_core::automation::SyncStats> {
        tracing::info!("Starting to index all files to vector database...");

        use cortex_mem_core::automation::{SyncConfig, SyncManager};

        // Create SyncManager
        let sync_manager = SyncManager::new(
            self.filesystem.clone(),
            self.embedding_client.clone(),
            self.vector_store.clone(),
            self.llm_client.clone(), // Not optional
            SyncConfig::default(),
        );

        match sync_manager.sync_all().await {
            Ok(stats) => {
                tracing::info!(
                    "Indexing completed: {} total files, {} indexed, {} skipped, {} errors",
                    stats.total_files,
                    stats.indexed_files,
                    stats.skipped_files,
                    stats.error_files
                );
                Ok(stats)
            }
            Err(e) => {
                tracing::error!("Indexing failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Index files to vector database for a specific session
    ///
    /// # Arguments
    /// * `session_id` - Session ID
    ///
    /// # Returns
    /// Returns indexing statistics
    pub async fn index_session_files(
        &self,
        session_id: &str,
    ) -> Result<cortex_mem_core::automation::SyncStats> {
        tracing::info!("Starting to index files to vector database for session {}...", session_id);

        use cortex_mem_core::automation::{SyncConfig, SyncManager};

        // Create SyncManager
        let sync_manager = SyncManager::new(
            self.filesystem.clone(),
            self.embedding_client.clone(),
            self.vector_store.clone(),
            self.llm_client.clone(),
            SyncConfig::default(),
        );

        // Limit scan scope to specific session
        let session_uri = format!("cortex://session/{}", session_id);

        match sync_manager.sync_specific_path(&session_uri).await {
            Ok(stats) => {
                tracing::info!(
                    "Session {} indexing completed: {} total files, {} indexed, {} skipped, {} errors",
                    session_id,
                    stats.total_files,
                    stats.indexed_files,
                    stats.skipped_files,
                    stats.error_files
                );
                Ok(stats)
            }
            Err(e) => {
                tracing::error!("Session {} indexing failed: {}", session_id, e);
                Err(e.into())
            }
        }
    }

    /// 等待所有后台异步任务完成（用于长时间服务等待）
    ///
    /// 使用真正的事件通知机制等待，而非固定超时。
    /// 在退出流程中建议优先使用 `close_session_sync`。
    pub async fn wait_for_background_tasks(&self, max_wait_secs: u64) -> bool {
        use std::time::Duration;

        if let Some(ref coordinator) = self.event_coordinator {
            // 使用真正的事件通知机制
            coordinator
                .wait_for_completion(Duration::from_secs(max_wait_secs))
                .await
        } else {
            // Fallback: if no coordinator, use simple wait
            warn!("MemoryEventCoordinator not initialized, using simple wait");
            tokio::time::sleep(Duration::from_secs(max_wait_secs.min(5))).await;
            true
        }
    }

    /// Wait for MemoryEventCoordinator to drain all pending events (deprecated-style polling).
    ///
    /// Prefer `close_session_sync` for exit flows — it blocks until the pipeline
    /// completes without polling. This method is retained for legacy call-sites
    /// where fire-and-forget + explicit wait is still needed.
    pub async fn flush_and_wait(&self, check_interval_secs: Option<u64>) -> bool {
        let interval = std::time::Duration::from_secs(check_interval_secs.unwrap_or(1));

        if let Some(ref coordinator) = self.event_coordinator {
            coordinator.flush_and_wait(interval).await
        } else {
            warn!("MemoryEventCoordinator not initialized, skipping wait");
            true
        }
    }

    // ==================== Long-Running Service APIs ====================
    // 以下 API 专为长期后台运行的服务（如 ZeroClaw）设计

    /// 手动触发记忆处理（供外部调度使用）
    ///
    /// 适用于长期后台运行的服务，可以在任意时间点手动触发记忆处理，
    /// 而不需要依赖 SessionClosed 事件。
    ///
    /// # Arguments
    /// * `scope` - 处理范围：User, Agent, Session, 或 Resources
    /// * `owner_id` - 所有者 ID（如 user_id, agent_id, session_id）
    ///
    /// # Returns
    /// 返回处理结果，包含更新的文件数和索引数
    pub async fn trigger_processing(
        &self,
        scope: &str,
        owner_id: &str,
    ) -> crate::Result<ProcessingResult> {
        let memory_scope = match scope.to_lowercase().as_str() {
            "user" => cortex_mem_core::MemoryScope::User,
            "agent" => cortex_mem_core::MemoryScope::Agent,
            "session" => cortex_mem_core::MemoryScope::Session,
            "resources" => cortex_mem_core::MemoryScope::Resources,
            _ => {
                return Err(crate::ToolsError::ValidationError(format!(
                    "Invalid scope: {}. Valid values: user, agent, session, resources",
                    scope
                )));
            }
        };

        tracing::info!("Manual trigger processing for {:?}/{}", memory_scope, owner_id);

        // Force update layers
        if let Some(ref coordinator) = self.event_coordinator {
            coordinator
                .force_full_update(&memory_scope, owner_id)
                .await?;
        } else {
            warn!("MemoryEventCoordinator not initialized, cannot trigger processing");
            return Ok(ProcessingResult::default());
        }

        // 索引文件（根据 scope 和 owner_id 确定路径）
        let sync_stats = match memory_scope {
            cortex_mem_core::MemoryScope::Session => {
                self.index_session_files(owner_id).await?
            }
            _ => {
                // 其他 scope 使用全局索引
                self.index_all_files().await?
            }
        };

        Ok(ProcessingResult {
            scope: scope.to_string(),
            owner_id: owner_id.to_string(),
            layers_updated: sync_stats.indexed_files,
            vectors_indexed: sync_stats.indexed_files,
        })
    }

    /// 获取待处理队列状态
    ///
    /// 返回当前的事件统计信息，用于监控和调度。
    pub async fn pending_status(&self) -> PendingStatus {
        if let Some(ref coordinator) = self.event_coordinator {
            let stats = coordinator.get_stats().await;
            PendingStatus {
                memory_created: stats.memory_created,
                memory_updated: stats.memory_updated,
                memory_deleted: stats.memory_deleted,
                layers_updated: stats.layers_updated,
                sessions_closed: stats.sessions_closed,
            }
        } else {
            PendingStatus::default()
        }
    }
}

/// 处理结果
#[derive(Debug, Clone, Default)]
pub struct ProcessingResult {
    /// 处理范围
    pub scope: String,
    /// 所有者 ID
    pub owner_id: String,
    /// 更新的层级文件数
    pub layers_updated: usize,
    /// 索引的向量数
    pub vectors_indexed: usize,
}

/// 待处理状态（事件统计）
#[derive(Debug, Clone, Default)]
pub struct PendingStatus {
    /// 创建的记忆数
    pub memory_created: u64,
    /// 更新的记忆数
    pub memory_updated: u64,
    /// 删除的记忆数
    pub memory_deleted: u64,
    /// 更新的层级数
    pub layers_updated: u64,
    /// 关闭的会话数
    pub sessions_closed: u64,
}
