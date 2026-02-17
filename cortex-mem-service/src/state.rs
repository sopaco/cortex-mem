use cortex_mem_core::{CortexFilesystem, LLMClient, SessionManager, QdrantVectorStore, EmbeddingClient, VectorSearchEngine};
use std::sync::Arc;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub filesystem: Arc<CortexFilesystem>,
    pub session_manager: Arc<tokio::sync::RwLock<SessionManager>>,
    pub llm_client: Option<Arc<dyn LLMClient>>,
    pub vector_store: Option<Arc<QdrantVectorStore>>,
    pub embedding_client: Option<Arc<EmbeddingClient>>,
    /// Vector search engine with L0/L1/L2 layered search support
    pub vector_engine: Option<Arc<VectorSearchEngine>>,
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

        // Initialize vector store and embedding client (optional)
        let (vector_store, embedding_client) = Self::init_vector_search().await?;

        // Initialize vector search engine for layered search with optional LLM support
        let vector_engine = match (&vector_store, &embedding_client, &llm_client) {
            (Some(vs), Some(ec), Some(llm)) => {
                Some(Arc::new(VectorSearchEngine::with_llm(
                    vs.clone(),
                    ec.clone(),
                    filesystem.clone(),
                    llm.clone(),
                )))
            }
            (Some(vs), Some(ec), None) => {
                // Without LLM, query rewriting will be disabled
                Some(Arc::new(VectorSearchEngine::new(
                    vs.clone(),
                    ec.clone(),
                    filesystem.clone(),
                )))
            }
            _ => None,
        };

        Ok(Self {
            filesystem,
            session_manager,
            llm_client,
            vector_store,
            embedding_client,
            vector_engine,
        })
    }

    fn init_llm_client() -> anyhow::Result<Option<Arc<dyn LLMClient>>> {
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

            let client = cortex_mem_core::llm::LLMClientImpl::new(config)?;
            tracing::info!("LLM client initialized");
            Ok(Some(Arc::new(client) as Arc<dyn cortex_mem_core::llm::LLMClient>))
        } else {
            tracing::warn!("LLM client not initialized (missing environment variables)");
            Ok(None)
        }
    }

    async fn init_vector_search() -> anyhow::Result<(Option<Arc<QdrantVectorStore>>, Option<Arc<EmbeddingClient>>)> {
        // Try to load config from file first, then fall back to environment variables
        let config_result = cortex_mem_config::Config::load("config.toml");
        
        if let Ok(config) = config_result {
            tracing::info!("Loaded config from config.toml");
            
            // Initialize embedding client
            let embedding_config = cortex_mem_core::embedding::EmbeddingConfig {
                api_base_url: config.embedding.api_base_url,
                api_key: config.embedding.api_key,
                model_name: config.embedding.model_name,
                batch_size: config.embedding.batch_size,
                timeout_secs: config.embedding.timeout_secs,
            };
            
            let embedding_client = match EmbeddingClient::new(embedding_config) {
                Ok(client) => {
                    tracing::info!("Embedding client initialized from config");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize embedding client: {}", e);
                    None
                }
            };
            
            // Initialize vector store
            let qdrant_config = cortex_mem_core::QdrantConfig {
                url: config.qdrant.url,
                collection_name: config.qdrant.collection_name,
                embedding_dim: config.qdrant.embedding_dim,
                timeout_secs: config.qdrant.timeout_secs,
            };
            
            let vector_store = match QdrantVectorStore::new(&qdrant_config).await {
                Ok(store) => {
                    tracing::info!("Vector store initialized from config");
                    Some(Arc::new(store))
                }
                Err(e) => {
                    tracing::warn!("Vector store initialization failed: {}", e);
                    None
                }
            };
            
            Ok((vector_store, embedding_client))
        } else {
            // Fallback to environment variables
            if let (Ok(url), Ok(collection)) = (
                std::env::var("QDRANT_URL"),
                std::env::var("QDRANT_COLLECTION"),
            ) {
                let qdrant_config = cortex_mem_core::QdrantConfig {
                    url,
                    collection_name: collection,
                    embedding_dim: std::env::var("QDRANT_EMBEDDING_DIM")
                        .ok()
                        .and_then(|s| s.parse().ok()),
                    timeout_secs: 30,
                };

                let vector_store = match QdrantVectorStore::new(&qdrant_config).await {
                    Ok(store) => {
                        tracing::info!("Vector store initialized from env");
                        Some(Arc::new(store))
                    }
                    Err(e) => {
                        tracing::warn!("Vector store initialization failed: {}", e);
                        None
                    }
                };
                
                // Try to initialize embedding client from env
                let embedding_client = if let (Ok(api_url), Ok(api_key), Ok(model)) = (
                    std::env::var("EMBEDDING_API_BASE_URL"),
                    std::env::var("EMBEDDING_API_KEY"),
                    std::env::var("EMBEDDING_MODEL"),
                ) {
                    let embedding_config = cortex_mem_core::embedding::EmbeddingConfig {
                        api_base_url: api_url,
                        api_key,
                        model_name: model,
                        batch_size: 10,
                        timeout_secs: 30,
                    };
                    
                    match EmbeddingClient::new(embedding_config) {
                        Ok(client) => {
                            tracing::info!("Embedding client initialized from env");
                            Some(Arc::new(client))
                        }
                        Err(e) => {
                            tracing::warn!("Failed to initialize embedding client: {}", e);
                            None
                        }
                    }
                } else {
                    tracing::warn!("Embedding client not configured");
                    None
                };

                Ok((vector_store, embedding_client))
            } else {
                tracing::info!("Vector search not configured (missing config file or environment variables)");
                Ok((None, None))
            }
        }
    }
}