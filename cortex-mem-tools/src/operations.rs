use crate::{errors::*, types::*};
use cortex_mem_core::{
    layers::manager::LayerManager, 
    llm::LLMClient, 
    search::VectorSearchEngine,
    CortexFilesystem, 
    FilesystemOperations,
    SessionConfig, 
    SessionManager,
    automation::{SyncConfig, SyncManager, AutoExtractor, AutoExtractConfig},  // 🆕 添加AutoExtractor
    embedding::{EmbeddingClient, EmbeddingConfig},
    vector_store::QdrantVectorStore,
    events::EventBus,  // 🆕 添加EventBus
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
    pub(crate) auto_extractor: Option<Arc<AutoExtractor>>,  // 🆕 AutoExtractor用于退出时提取
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
    
    /// 🆕 Get the session manager
    pub fn session_manager(&self) -> &Arc<RwLock<SessionManager>> {
        &self.session_manager
    }
    
    /// 🆕 Get the auto extractor (for manual extraction on exit)
    pub fn auto_extractor(&self) -> Option<&Arc<AutoExtractor>> {
        self.auto_extractor.as_ref()
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
    ) -> Result<Self> {
        let tenant_id = tenant_id.into();
        let filesystem = Arc::new(CortexFilesystem::with_tenant(data_dir, &tenant_id));
        filesystem.initialize().await?;

        // 🆕 创建EventBus用于自动化
        let (event_bus, mut event_rx) = EventBus::new();

        let config = SessionConfig::default();
        // 🆕 使用with_llm_and_events创建SessionManager
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
        };
        let vector_store = Arc::new(QdrantVectorStore::new(&qdrant_config).await?);
        tracing::info!("Qdrant connected successfully");

        // Initialize Embedding client
        tracing::info!("Initializing Embedding client with model: {}", embedding_model_name);
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

        // 🆕 创建AutoExtractor用于退出时提取（带user_id）
        let auto_extract_config = AutoExtractConfig {
            min_message_count: 5,
            extract_on_close: true,
            save_user_memories: true,
            save_agent_memories: true,
        };
        let auto_extractor = Arc::new(AutoExtractor::with_user_id(
            filesystem.clone(),
            llm_client.clone(),
            auto_extract_config,
            &tenant_id,  // 使用tenant_id作为user_id
        ));
        
        // 🆕 启动后台监听器处理SessionClosed事件
        let extractor_clone = auto_extractor.clone();
        tokio::spawn(async move {
            tracing::info!("Starting AutoExtractor event listener for tenant {}", tenant_id);
            while let Some(event) = event_rx.recv().await {
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
                        _ => {}  // 忽略其他事件
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
            auto_extractor: Some(auto_extractor),  // 🆕
        })
    }

    /// Add a message to a session
    pub async fn add_message(&self, thread_id: &str, role: &str, content: &str) -> Result<String> {
        let thread_id = if thread_id.is_empty() { "default" } else { thread_id };

        let sm = self.session_manager.read().await;

        if !sm.session_exists(thread_id).await? {
            drop(sm);
            let sm = self.session_manager.write().await;
            sm.create_session(thread_id).await?;
            drop(sm);
        }

        let sm = self.session_manager.read().await;

        let message = cortex_mem_core::Message::new(
            match role {
                "user" => cortex_mem_core::MessageRole::User,
                "assistant" => cortex_mem_core::MessageRole::Assistant,
                "system" => cortex_mem_core::MessageRole::System,
                _ => cortex_mem_core::MessageRole::User,
            },
            content,
        );

        let message_uri = sm.message_storage().save_message(thread_id, &message).await?;

        let message_id = message_uri.rsplit('/').next().unwrap_or("unknown").to_string();

        tracing::info!("Added message {} to session {}", message_id, thread_id);
        Ok(message_id)
    }

    /// List sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let entries = self.filesystem.list("cortex://session").await?;

        let mut session_infos = Vec::new();
        for entry in entries {
            if entry.is_directory {
                let thread_id = entry.name;
                if let Ok(metadata) = self.session_manager.read().await.load_session(&thread_id).await {
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
        self.filesystem.delete(uri).await?;
        tracing::info!("Deleted: {}", uri);
        Ok(())
    }

    /// Check if file/directory exists
    pub async fn exists(&self, uri: &str) -> Result<bool> {
        let exists = self.filesystem.exists(uri).await.map_err(ToolsError::Core)?;
        Ok(exists)
    }
}