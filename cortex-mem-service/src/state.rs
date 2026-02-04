use cortex_mem_core::{CortexFilesystem, LLMClient, SessionManager};
use std::sync::Arc;

#[cfg(feature = "vector-search")]
use cortex_mem_core::QdrantVectorStore;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub filesystem: Arc<CortexFilesystem>,
    pub session_manager: Arc<tokio::sync::RwLock<SessionManager>>,
    pub llm_client: Option<Arc<LLMClient>>,
    #[cfg(feature = "vector-search")]
    pub vector_store: Option<Arc<QdrantVectorStore>>,
}

impl AppState {
    /// Create new application state
    pub async fn new(data_dir: &str) -> anyhow::Result<Self> {
        // Initialize filesystem
        let filesystem = Arc::new(CortexFilesystem::new(data_dir));
        filesystem.initialize().await?;

        // Initialize session manager with default config
        let session_config = cortex_mem_core::SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), session_config);
        let session_manager = Arc::new(tokio::sync::RwLock::new(session_manager));

        // Initialize LLM client (optional, based on env or config)
        let llm_client = Self::init_llm_client()?;

        // Initialize vector store (optional, feature-gated)
        #[cfg(feature = "vector-search")]
        let vector_store = Self::init_vector_store().await?;

        Ok(Self {
            filesystem,
            session_manager,
            llm_client,
            #[cfg(feature = "vector-search")]
            vector_store,
        })
    }

    fn init_llm_client() -> anyhow::Result<Option<Arc<LLMClient>>> {
        // Try to initialize from environment variables
        if let (Ok(api_url), Ok(api_key), Ok(model)) = (
            std::env::var("LLM_API_BASE_URL"),
            std::env::var("LLM_API_KEY"),
            std::env::var("LLM_MODEL"),
        ) {
            let config = cortex_mem_core::llm::client::LLMConfig {
                api_base_url: api_url,
                api_key,
                model_efficient: model,
                temperature: 0.1,
                max_tokens: 4096,
            };

            let client = LLMClient::new(config)?;
            tracing::info!("LLM client initialized");
            Ok(Some(Arc::new(client)))
        } else {
            tracing::warn!("LLM client not initialized (missing environment variables)");
            Ok(None)
        }
    }

    #[cfg(feature = "vector-search")]
    async fn init_vector_store() -> anyhow::Result<Option<Arc<QdrantVectorStore>>> {
        // Try to initialize from environment variables or config file
        if let (Ok(url), Ok(collection)) = (
            std::env::var("QDRANT_URL"),
            std::env::var("QDRANT_COLLECTION"),
        ) {
            let config = cortex_mem_core::QdrantConfig {
                url,
                collection_name: collection,
                embedding_dim: std::env::var("QDRANT_EMBEDDING_DIM")
                    .ok()
                    .and_then(|s| s.parse().ok()),
                timeout_secs: 10,
            };

            match QdrantVectorStore::new(&config).await {
                Ok(store) => {
                    tracing::info!("Vector store initialized");
                    Ok(Some(Arc::new(store)))
                }
                Err(e) => {
                    tracing::warn!("Vector store initialization failed: {}", e);
                    Ok(None)
                }
            }
        } else {
            tracing::info!("Vector store not configured (missing environment variables)");
            Ok(None)
        }
    }
}
