use crate::{errors::*, types::*};
use cortex_mem_core::{
    CortexFilesystem, SessionManager, SessionConfig, FilesystemOperations, LayerManager,
    llm::LLMClient,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "vector-search")]
use cortex_mem_core::search::VectorSearchEngine;

/// High-level memory operations with OpenViking-style tiered access
pub struct MemoryOperations {
    pub(crate) filesystem: Arc<CortexFilesystem>,
    pub(crate) session_manager: Arc<RwLock<SessionManager>>,
    pub(crate) layer_manager: Arc<LayerManager>,
    
    #[cfg(feature = "vector-search")]
    pub(crate) vector_engine: Option<Arc<VectorSearchEngine>>,
}

impl MemoryOperations {
    /// Create new memory operations
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        session_manager: Arc<RwLock<SessionManager>>,
    ) -> Self {
        let layer_manager = Arc::new(LayerManager::new(filesystem.clone()));
        
        Self {
            filesystem,
            session_manager,
            layer_manager,
            #[cfg(feature = "vector-search")]
            vector_engine: None,
        }
    }
    
    /// Get the underlying filesystem
    pub fn filesystem(&self) -> &Arc<CortexFilesystem> {
        &self.filesystem
    }

    /// Create from data directory (no tenant isolation)
    pub async fn from_data_dir(data_dir: &str) -> Result<Self> {
        let filesystem = Arc::new(CortexFilesystem::new(data_dir));
        filesystem.initialize().await?;

        let config = SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), config);
        let session_manager = Arc::new(RwLock::new(session_manager));
        let layer_manager = Arc::new(LayerManager::new(filesystem.clone()));

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            #[cfg(feature = "vector-search")]
            vector_engine: None,
        })
    }
    
    /// Create from data directory with tenant isolation
    pub async fn with_tenant(data_dir: &str, tenant_id: impl Into<String>) -> Result<Self> {
        let filesystem = Arc::new(CortexFilesystem::with_tenant(data_dir, tenant_id));
        filesystem.initialize().await?;

        let config = SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), config);
        let session_manager = Arc::new(RwLock::new(session_manager));
        
        // Use fallback LayerManager (no LLM) - for LLM support, use with_tenant_and_llm()
        let layer_manager = Arc::new(LayerManager::new(filesystem.clone()));

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            #[cfg(feature = "vector-search")]
            vector_engine: None,
        })
    }
    
    /// Create from data directory with tenant isolation and LLM support
    pub async fn with_tenant_and_llm(
        data_dir: &str,
        tenant_id: impl Into<String>,
        llm_client: Arc<dyn LLMClient>,
    ) -> Result<Self> {
        let filesystem = Arc::new(CortexFilesystem::with_tenant(data_dir, tenant_id));
        filesystem.initialize().await?;

        let config = SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), config);
        let session_manager = Arc::new(RwLock::new(session_manager));
        
        // Use LLM-enabled LayerManager for high-quality L0/L1 generation
        let layer_manager = Arc::new(LayerManager::with_llm(filesystem.clone(), llm_client));

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            #[cfg(feature = "vector-search")]
            vector_engine: None,
        })
    }
    
    /// Create from data directory with tenant isolation, LLM support, and vector search
    #[cfg(feature = "vector-search")]
    pub async fn with_tenant_and_vector(
        data_dir: &str,
        tenant_id: impl Into<String>,
        llm_client: Arc<dyn LLMClient>,
        qdrant_url: &str,
        qdrant_collection: &str,
        embedding_api_base_url: &str,
        embedding_api_key: &str,
    ) -> Result<Self> {
        use cortex_mem_core::{
            embedding::EmbeddingClient, embedding::EmbeddingConfig,
            search::VectorSearchEngine,
            vector_store::QdrantVectorStore,
            automation::SyncManager, automation::SyncConfig,
        };

        let filesystem = Arc::new(CortexFilesystem::with_tenant(data_dir, tenant_id));
        filesystem.initialize().await?;

        let config = SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), config);
        let session_manager = Arc::new(RwLock::new(session_manager));
        
        // Use LLM-enabled LayerManager for high-quality L0/L1 generation
        let layer_manager = Arc::new(LayerManager::with_llm(filesystem.clone(), llm_client));

        // Initialize Qdrant
        tracing::info!("正在初始化 Qdrant 向量存储: {}", qdrant_url);
        let qdrant_config = cortex_mem_core::QdrantConfig {
            url: qdrant_url.to_string(),
            collection_name: qdrant_collection.to_string(),
            embedding_dim: Some(1536), // Default for OpenAI embeddings
            timeout_secs: 30,
        };
        let vector_store = Arc::new(QdrantVectorStore::new(&qdrant_config).await?);
        tracing::info!("✅ Qdrant 连接成功");

        // Initialize Embedding client
        tracing::info!("正在初始化 Embedding 客户端");
        let embedding_config = EmbeddingConfig {
            api_base_url: embedding_api_base_url.to_string(),
            api_key: embedding_api_key.to_string(),
            model_name: "text-embedding-3-small".to_string(),
            batch_size: 10,
            timeout_secs: 30,
        };
        let embedding_client = Arc::new(EmbeddingClient::new(embedding_config)?);
        tracing::info!("✅ Embedding 客户端初始化成功");

        // Create vector search engine
        let vector_engine = Arc::new(VectorSearchEngine::new(
            vector_store.clone(),
            embedding_client.clone(),
            filesystem.clone(),
        ));
        tracing::info!("✅ 向量搜索引擎创建成功");

        // Auto-sync existing content to vector database (in background)
        let sync_manager = SyncManager::new(
            filesystem.clone(),
            embedding_client.clone(),
            vector_store.clone(),
            SyncConfig::default(),
        );

        // Spawn background sync task
        tokio::spawn(async move {
            tracing::info!("🔄 开始后台同步到向量数据库...");
            match sync_manager.sync_all().await {
                Ok(stats) => {
                    tracing::info!(
                        "✅ 自动同步完成: {} 个文件已索引, {} 个文件跳过",
                        stats.indexed_files,
                        stats.skipped_files
                    );
                }
                Err(e) => {
                    tracing::warn!("⚠️ 自动同步失败: {}", e);
                }
            }
        });

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            vector_engine: Some(vector_engine),
        })
    }

    /// Create from data directory with vector search enabled
    #[cfg(feature = "vector-search")]
    pub async fn from_data_dir_with_vector(
        data_dir: &str,
        qdrant_url: &str,
        qdrant_collection: &str,
        embedding_api_base_url: &str,
        embedding_api_key: &str,
    ) -> Result<Self> {
        use cortex_mem_core::{
            embedding::EmbeddingClient, embedding::EmbeddingConfig,
            search::VectorSearchEngine,
            vector_store::QdrantVectorStore,
            automation::SyncManager, automation::SyncConfig,
        };

        let filesystem = Arc::new(CortexFilesystem::new(data_dir));
        filesystem.initialize().await?;

        let config = SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), config);
        let session_manager = Arc::new(RwLock::new(session_manager));
        let layer_manager = Arc::new(LayerManager::new(filesystem.clone()));

        // Initialize Qdrant
        let qdrant_config = cortex_mem_core::QdrantConfig {
            url: qdrant_url.to_string(),
            collection_name: qdrant_collection.to_string(),
            embedding_dim: Some(1536), // Default for OpenAI embeddings
            timeout_secs: 30,
        };
        let vector_store = Arc::new(QdrantVectorStore::new(&qdrant_config).await?);

        // Initialize Embedding client
        let embedding_config = EmbeddingConfig {
            api_base_url: embedding_api_base_url.to_string(),
            api_key: embedding_api_key.to_string(),
            model_name: "text-embedding-3-small".to_string(),
            batch_size: 10,
            timeout_secs: 30,
        };
        let embedding_client = Arc::new(EmbeddingClient::new(embedding_config)?);

        // Create vector search engine
        let vector_engine = Arc::new(VectorSearchEngine::new(
            vector_store.clone(),
            embedding_client.clone(),
            filesystem.clone(),
        ));

        // Auto-sync existing content to vector database
        let sync_manager = SyncManager::new(
            filesystem.clone(),
            embedding_client.clone(),
            vector_store.clone(),
            SyncConfig::default(),
        );

        tracing::info!("Starting auto-sync to vector database...");
        match sync_manager.sync_all().await {
            Ok(stats) => {
                tracing::info!(
                    "Auto-sync completed: {} indexed, {} skipped",
                    stats.indexed_files,
                    stats.skipped_files
                );
            }
            Err(e) => {
                tracing::warn!("Auto-sync failed: {}", e);
                // Don't fail initialization if sync fails
            }
        }

        Ok(Self {
            filesystem,
            session_manager,
            layer_manager,
            vector_engine: Some(vector_engine),
        })
    }
    
    /// Create with vector search engine (requires vector-search feature)
    #[cfg(feature = "vector-search")]
    pub fn with_vector_engine(mut self, engine: Arc<VectorSearchEngine>) -> Self {
        self.vector_engine = Some(engine);
        self
    }

    /// Add a message to a session
    pub async fn add_message(
        &self,
        thread_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String> {
        // Ensure thread_id is not empty
        let thread_id = if thread_id.is_empty() {
            "default"
        } else {
            thread_id
        };
        
        let sm = self.session_manager.read().await;
        
        // Ensure session exists
        if !sm.session_exists(thread_id).await? {
            drop(sm);
            let sm = self.session_manager.write().await;
            sm.create_session(thread_id).await?;
            drop(sm);
        }
        
        // Add message using MessageStorage
        let sm = self.session_manager.read().await;
        
        // Create message
        let message = cortex_mem_core::Message::new(
            match role {
                "user" => cortex_mem_core::MessageRole::User,
                "assistant" => cortex_mem_core::MessageRole::Assistant,
                "system" => cortex_mem_core::MessageRole::System,
                _ => cortex_mem_core::MessageRole::User,
            },
            content
        );
        
        let message_uri = sm.message_storage().save_message(thread_id, &message).await?;
        
        // Extract message ID from URI
        let message_id = message_uri.rsplit('/').next()
            .unwrap_or("unknown")
            .to_string();
        
        tracing::info!("Added message {} to session {}", message_id, thread_id);
        Ok(message_id)
    }

    /// List sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        // List all thread directories
        let entries = self.filesystem.list("cortex://threads").await?;
        
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_operations() {
        let tmpdir = tempfile::tempdir().unwrap();
        let ops = MemoryOperations::from_data_dir(tmpdir.path().to_str().unwrap())
            .await
            .unwrap();

        // Add message
        let msg_id = ops.add_message("test-session", "user", "Hello").await.unwrap();
        assert!(!msg_id.is_empty());

        // List sessions
        let sessions = ops.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
    }
}
