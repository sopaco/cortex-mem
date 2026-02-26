use crate::{errors::*, types::*};
use cortex_mem_core::{
    CortexFilesystem,
    FilesystemOperations,
    SessionConfig,
    SessionManager,
    automation::{
        AbstractConfig, AutoExtractConfig, AutoExtractor, AutoIndexer, AutomationConfig,
        AutomationManager, IndexerConfig, LayerGenerationConfig, LayerGenerator, OverviewConfig,
        SyncConfig, SyncManager,
    },
    embedding::{EmbeddingClient, EmbeddingConfig},
    events::EventBus,
    layers::manager::LayerManager,
    llm::LLMClient,
    search::VectorSearchEngine,
    vector_store::{QdrantVectorStore, VectorStore}, // 🔧 添加VectorStore trait
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// High-level memory operations with OpenViking-style tiered access
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
    pub(crate) auto_extractor: Option<Arc<AutoExtractor>>,
    pub(crate) layer_generator: Option<Arc<LayerGenerator>>,
    pub(crate) auto_indexer: Option<Arc<AutoIndexer>>,

    // 保存组件引用以便退出时索引使用
    pub(crate) embedding_client: Arc<EmbeddingClient>,
    pub(crate) vector_store: Arc<QdrantVectorStore>,
    pub(crate) llm_client: Arc<dyn LLMClient>,

    pub(crate) default_user_id: String,
    pub(crate) default_agent_id: String,
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

    /// Get the auto extractor (for manual extraction on exit)
    pub fn auto_extractor(&self) -> Option<&Arc<AutoExtractor>> {
        self.auto_extractor.as_ref()
    }

    /// Get the layer generator (for manual layer generation on exit)
    pub fn layer_generator(&self) -> Option<&Arc<LayerGenerator>> {
        self.layer_generator.as_ref()
    }

    /// Get the auto indexer (for manual indexing on exit)
    pub fn auto_indexer(&self) -> Option<&Arc<AutoIndexer>> {
        self.auto_indexer.as_ref()
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
        embedding_api_base_url: &str,
        embedding_api_key: &str,
        embedding_model_name: &str,
        embedding_dim: Option<usize>,
        user_id: Option<String>,
    ) -> Result<Self> {
        let tenant_id = tenant_id.into();
        let filesystem = Arc::new(CortexFilesystem::with_tenant(data_dir, &tenant_id));
        filesystem.initialize().await?;

        // 创建EventBus用于自动化
        let (event_bus, mut event_rx_main) = EventBus::new();

        let config = SessionConfig::default();
        // 使用with_llm_and_events创建SessionManager
        let session_manager = SessionManager::with_llm_and_events(
            filesystem.clone(),
            config,
            llm_client.clone(),
            event_bus.clone(),
        );
        let session_manager = Arc::new(RwLock::new(session_manager));

        // LLM-enabled LayerManager for high-quality L0/L1 generation
        let layer_manager = Arc::new(LayerManager::new(filesystem.clone(), llm_client.clone()));

        // Initialize Qdrant
        tracing::info!("Initializing Qdrant vector store: {}", qdrant_url);
        let qdrant_config = cortex_mem_core::QdrantConfig {
            url: qdrant_url.to_string(),
            collection_name: qdrant_collection.to_string(),
            embedding_dim,
            timeout_secs: 30,
            tenant_id: Some(tenant_id.clone()),
        };
        let vector_store = Arc::new(QdrantVectorStore::new(&qdrant_config).await?);
        tracing::info!(
            "Qdrant connected successfully, collection: {}",
            qdrant_config.get_collection_name()
        );

        // Initialize Embedding client
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
        };
        let embedding_client = Arc::new(EmbeddingClient::new(embedding_config)?);
        tracing::info!("Embedding client initialized");

        // Create vector search engine with LLM support for query rewriting
        let vector_engine = Arc::new(VectorSearchEngine::with_llm(
            vector_store.clone(),
            embedding_client.clone(),
            filesystem.clone(),
            llm_client.clone(),
        ));
        tracing::info!("Vector search engine created with LLM support for query rewriting");

        // 使用传入的user_id，如果没有则使用tenant_id
        let actual_user_id = user_id.unwrap_or_else(|| tenant_id.clone());

        // 🔧 创建AutoExtractor(简化配置，移除了save_user_memories和save_agent_memories)
        let auto_extract_config = AutoExtractConfig {
            min_message_count: 5,
            extract_on_close: true, // 🔧 显式设置为true，确保会话关闭时自动提取记忆
        };
        let auto_extractor = Arc::new(AutoExtractor::with_user_id(
            filesystem.clone(),
            llm_client.clone(),
            auto_extract_config,
            &actual_user_id,
        ));

        // 创建AutoIndexer用于实时索引
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

        // 创建AutomationManager
        let automation_config = AutomationConfig {
            auto_index: true,
            auto_extract: false,    // Extract由单独的监听器处理
            index_on_message: true, // ✅ 消息时自动索引L2
            index_on_close: true,   // ✅ Session关闭时生成L0/L1并索引
            index_batch_delay: 1,
            auto_generate_layers_on_startup: false, // 启动时不生成（避免阻塞）
            generate_layers_every_n_messages: 5,    // 每5条消息生成一次L0/L1
        };

        // 创建LayerGenerator（用于退出时手动生成）
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

        let automation_manager = AutomationManager::new(
            auto_indexer.clone(),
            None, // extractor由单独的监听器处理
            automation_config,
        )
        .with_layer_generator(layer_generator.clone()); // 设置LayerGenerator

        // 创建事件转发器（将主EventBus的事件转发给两个监听器）
        let (tx_automation, rx_automation) = tokio::sync::mpsc::unbounded_channel();
        let (tx_extractor, rx_extractor) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(event) = event_rx_main.recv().await {
                // 转发给AutomationManager
                let _ = tx_automation.send(event.clone());
                // 转发给AutoExtractor监听器
                let _ = tx_extractor.send(event);
            }
        });

        // 启动AutomationManager监听事件并自动索引
        let tenant_id_for_automation = tenant_id.clone();
        tokio::spawn(async move {
            tracing::info!(
                "Starting AutomationManager for tenant {}",
                tenant_id_for_automation
            );
            if let Err(e) = automation_manager.start(rx_automation).await {
                tracing::error!("AutomationManager stopped with error: {}", e);
            }
        });

        // 启动后台监听器处理SessionClosed事件
        let extractor_clone = auto_extractor.clone();
        let tenant_id_clone = tenant_id.clone();
        tokio::spawn(async move {
            tracing::info!(
                "Starting AutoExtractor event listener for tenant {}",
                tenant_id_clone
            );
            let mut rx = rx_extractor;
            while let Some(event) = rx.recv().await {
                if let cortex_mem_core::CortexEvent::Session(session_event) = event {
                    match session_event {
                        cortex_mem_core::SessionEvent::Closed { session_id } => {
                            tracing::info!("Session closed event received: {}", session_id);
                            match extractor_clone.extract_session(&session_id).await {
                                Ok(stats) => {
                                    tracing::info!(
                                        "Extraction completed for session {}: {:?}",
                                        session_id,
                                        stats
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Extraction failed for session {}: {}",
                                        session_id,
                                        e
                                    );
                                }
                            }
                        }
                        _ => {} // 忽略其他事件
                    }
                }
            }
        });

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

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            vector_engine,
            auto_extractor: Some(auto_extractor),
            layer_generator: Some(layer_generator), // 保存LayerGenerator用于退出时生成
            auto_indexer: Some(auto_indexer),       // 保存AutoIndexer用于退出时索引

            // 保存组件引用以便退出时索引使用
            embedding_client,
            vector_store,
            llm_client,

            default_user_id: actual_user_id,
            default_agent_id: tenant_id.clone(),
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

    /// Close session
    pub async fn close_session(&self, thread_id: &str) -> Result<()> {
        let mut sm = self.session_manager.write().await;
        sm.close_session(thread_id).await?;
        tracing::info!("Closed session: {}", thread_id);
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

    /// 生成所有缺失的 L0/L1 层级文件（用于退出时调用）
    ///
    /// 这个方法扫描所有目录，找出缺失 .abstract.md 或 .overview.md 的目录，
    /// 并批量生成它们。适合在应用退出时调用。
    pub async fn ensure_all_layers(&self) -> Result<cortex_mem_core::automation::GenerationStats> {
        if let Some(ref generator) = self.layer_generator {
            tracing::info!("🔍 开始扫描并生成缺失的 L0/L1 层级文件...");
            match generator.ensure_all_layers().await {
                Ok(stats) => {
                    tracing::info!(
                        "✅ L0/L1 层级生成完成: 总计 {}, 成功 {}, 失败 {}",
                        stats.total,
                        stats.generated,
                        stats.failed
                    );
                    Ok(stats)
                }
                Err(e) => {
                    tracing::error!("❌ L0/L1 层级生成失败: {}", e);
                    Err(e.into())
                }
            }
        } else {
            tracing::warn!("⚠️ LayerGenerator 未配置，跳过层级生成");
            Ok(cortex_mem_core::automation::GenerationStats::default())
        }
    }

    /// 为特定session生成 L0/L1 层级文件
    /// # Arguments
    /// * `session_id` - 会话ID
    ///
    /// # Returns
    /// 返回生成统计信息
    pub async fn ensure_session_layers(
        &self,
        session_id: &str,
    ) -> Result<cortex_mem_core::automation::GenerationStats> {
        if let Some(ref generator) = self.layer_generator {
            let timeline_uri = format!("cortex://session/{}/timeline", session_id);
            tracing::info!("🔍 为会话 {} 生成 L0/L1 层级文件", session_id);

            match generator.ensure_timeline_layers(&timeline_uri).await {
                Ok(stats) => {
                    tracing::info!(
                        "✅ 会话 {} L0/L1 层级生成完成: 总计 {}, 成功 {}, 失败 {}",
                        session_id,
                        stats.total,
                        stats.generated,
                        stats.failed
                    );
                    Ok(stats)
                }
                Err(e) => {
                    tracing::error!("❌ 会话 {} L0/L1 层级生成失败: {}", session_id, e);
                    Err(e.into())
                }
            }
        } else {
            tracing::warn!("⚠️ LayerGenerator 未配置，跳过层级生成");
            Ok(cortex_mem_core::automation::GenerationStats::default())
        }
    }

    /// 索引所有文件到向量数据库（用于退出时调用）
    /// 这个方法扫描所有文件，包括新生成的 .abstract.md 和 .overview.md，
    /// 并将它们索引到向量数据库中。适合在应用退出时调用。
    pub async fn index_all_files(&self) -> Result<cortex_mem_core::automation::SyncStats> {
        tracing::info!("📊 开始索引所有文件到向量数据库...");

        use cortex_mem_core::automation::{SyncConfig, SyncManager};

        // 创建 SyncManager
        let sync_manager = SyncManager::new(
            self.filesystem.clone(),
            self.embedding_client.clone(),
            self.vector_store.clone(),
            self.llm_client.clone(), // 不需要 Option
            SyncConfig::default(),
        );

        match sync_manager.sync_all().await {
            Ok(stats) => {
                tracing::info!(
                    "✅ 索引完成: 总计 {} 个文件, {} 个已索引, {} 个跳过, {} 个错误",
                    stats.total_files,
                    stats.indexed_files,
                    stats.skipped_files,
                    stats.error_files
                );
                Ok(stats)
            }
            Err(e) => {
                tracing::error!("❌ 索引失败: {}", e);
                Err(e.into())
            }
        }
    }

    /// 为特定session索引文件到向量数据库
    ///
    /// # Arguments
    /// * `session_id` - 会话ID
    ///
    /// # Returns
    /// 返回索引统计信息
    pub async fn index_session_files(
        &self,
        session_id: &str,
    ) -> Result<cortex_mem_core::automation::SyncStats> {
        tracing::info!("📊 开始为会话 {} 索引文件到向量数据库...", session_id);

        use cortex_mem_core::automation::{SyncConfig, SyncManager};

        // 创建 SyncManager
        let sync_manager = SyncManager::new(
            self.filesystem.clone(),
            self.embedding_client.clone(),
            self.vector_store.clone(),
            self.llm_client.clone(),
            SyncConfig::default(),
        );

        // 限定扫描范围到特定session
        let session_uri = format!("cortex://session/{}", session_id);

        match sync_manager.sync_specific_path(&session_uri).await {
            Ok(stats) => {
                tracing::info!(
                    "✅ 会话 {} 索引完成: 总计 {} 个文件, {} 个已索引, {} 个跳过, {} 个错误",
                    session_id,
                    stats.total_files,
                    stats.indexed_files,
                    stats.skipped_files,
                    stats.error_files
                );
                Ok(stats)
            }
            Err(e) => {
                tracing::error!("❌ 会话 {} 索引失败: {}", session_id, e);
                Err(e.into())
            }
        }
    }
}
