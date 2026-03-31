use cortex_mem_core::{
    CortexMem, CortexMemBuilder, EmbeddingClient, EmbeddingConfig, FilesystemOperations, LLMClient,
    MemoryIndexManager, QdrantConfig, SessionManager, VectorSearchEngine,
    automation::{SyncConfig, SyncManager},
    memory_events::MemoryEvent,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Current runtime. Rebuilt atomically on tenant switch to avoid split-brain state.
    pub cortex: Arc<RwLock<Arc<CortexMem>>>,
    /// Current SessionManager handle. Wrapped so tenant switch can replace the whole Arc.
    pub session_manager: Arc<RwLock<Arc<tokio::sync::RwLock<SessionManager>>>>,
    pub llm_client: Option<Arc<dyn LLMClient>>,
    pub vector_store: Arc<RwLock<Option<Arc<dyn cortex_mem_core::vector_store::VectorStore>>>>,
    #[allow(dead_code)]
    pub embedding_client: Option<Arc<EmbeddingClient>>,
    /// Vector search engine with L0/L1/L2 layered search support
    pub vector_engine: Arc<RwLock<Option<Arc<VectorSearchEngine>>>>,
    /// Base data directory
    pub data_dir: PathBuf,
    /// Current tenant root directory (if set)
    pub current_tenant_root: Arc<RwLock<Option<PathBuf>>>,
    /// Current tenant ID
    pub current_tenant_id: Arc<RwLock<Option<String>>>,
    /// Current runtime's memory event sender
    pub memory_event_tx: Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<MemoryEvent>>>>,
    /// Whether to use LLM intent analysis before each search (from config.toml [cortex] section).
    pub enable_intent_analysis: bool,
    /// Set of tenant IDs that have already had their bootstrap vector sync executed.
    /// Prevents duplicate bootstrap runs when the same tenant is switched multiple times.
    bootstrapped_tenants: Arc<RwLock<HashSet<String>>>,
    /// Path to the configuration file
    config_path: PathBuf,
}

impl AppState {
    /// Create new application state with unified automation
    pub async fn new(data_dir: &str, config_path: &Path) -> anyhow::Result<Self> {
        let data_dir = PathBuf::from(data_dir);

        tracing::info!("Initializing Cortex Memory with unified automation...");

        let (llm_client, embedding_config, qdrant_config) = Self::load_configs(config_path)?;

        let enable_intent_analysis = cortex_mem_config::Config::load(config_path)
            .map(|c| c.cortex.enable_intent_analysis)
            .unwrap_or(true);

        let cortex = Arc::new(
            Self::build_runtime(
                &data_dir,
                None,
                llm_client.clone(),
                embedding_config,
                qdrant_config,
            )
            .await?,
        );
        Self::bootstrap_vectors_if_collection_empty(&cortex).await?;
        tracing::info!("✅ Cortex Memory initialized with MemoryEventCoordinator");

        let session_manager = cortex.session_manager();
        let embedding_client = cortex.embedding();
        let vector_store = cortex.vector_store();
        let memory_event_tx = cortex.memory_event_tx();
        let vector_engine = Self::build_vector_engine(&cortex, enable_intent_analysis);

        Ok(Self {
            cortex: Arc::new(RwLock::new(cortex)),
            session_manager: Arc::new(RwLock::new(session_manager)),
            llm_client,
            vector_store: Arc::new(RwLock::new(vector_store)),
            embedding_client,
            vector_engine: Arc::new(RwLock::new(vector_engine)),
            data_dir,
            current_tenant_root: Arc::new(RwLock::new(None)),
            current_tenant_id: Arc::new(RwLock::new(None)),
            memory_event_tx: Arc::new(RwLock::new(memory_event_tx)),
            enable_intent_analysis,
            bootstrapped_tenants: Arc::new(RwLock::new(HashSet::new())),
            config_path: config_path.to_path_buf(),
        })
    }

    /// Get current SessionManager handle
    pub async fn current_session_manager(&self) -> Arc<RwLock<SessionManager>> {
        self.session_manager.read().await.clone()
    }

    fn build_vector_engine(
        cortex: &Arc<CortexMem>,
        enable_intent_analysis: bool,
    ) -> Option<Arc<VectorSearchEngine>> {
        let filesystem = cortex.filesystem();
        let embedding_client = cortex.embedding();
        let qdrant_store_typed = cortex.qdrant_store();
        let llm_client = cortex.llm_client();
        let memory_event_tx = cortex.memory_event_tx();
        let index_manager = Arc::new(MemoryIndexManager::new(filesystem.clone()));

        if let (Some(qdrant_arc), Some(ec)) = (qdrant_store_typed, embedding_client) {
            let mut engine = if let Some(llm) = llm_client {
                VectorSearchEngine::with_llm(qdrant_arc, ec.clone(), filesystem.clone(), llm)
            } else {
                VectorSearchEngine::new(qdrant_arc, ec.clone(), filesystem.clone())
            };

            if let Some(ref tx) = memory_event_tx {
                engine = engine.with_memory_event_tx(tx.clone());
            }
            engine = engine.with_index_manager(index_manager);
            engine = engine.with_intent_analysis(enable_intent_analysis);
            Some(Arc::new(engine))
        } else {
            None
        }
    }

    async fn build_runtime(
        runtime_root: &Path,
        tenant_id: Option<String>,
        llm_client: Option<Arc<dyn LLMClient>>,
        embedding_config: Option<EmbeddingConfig>,
        qdrant_config: Option<QdrantConfig>,
    ) -> anyhow::Result<CortexMem> {
        let expected_vector = qdrant_config.is_some() && embedding_config.is_some();
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 1..=3 {
            let mut builder = CortexMemBuilder::new(runtime_root);

            if let Some(llm) = llm_client.clone() {
                builder = builder.with_llm(llm);
            }
            if let Some(emb_cfg) = embedding_config.clone() {
                builder = builder.with_embedding(emb_cfg);
            }
            if let Some(mut qdrant_cfg) = qdrant_config.clone() {
                qdrant_cfg.tenant_id = tenant_id.clone();
                builder = builder.with_qdrant(qdrant_cfg);
            }

            match builder.build().await {
                Ok(cortex) => {
                    if expected_vector
                        && (cortex.vector_store().is_none() || cortex.memory_event_tx().is_none())
                    {
                        let err = anyhow::anyhow!(
                            "runtime built without vector/coordinator capability for {:?}",
                            runtime_root
                        );
                        tracing::warn!(
                            "Tenant runtime build attempt {} degraded unexpectedly: {}",
                            attempt,
                            err
                        );
                        last_error = Some(err);
                    } else {
                        return Ok(cortex);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Tenant runtime build attempt {} failed for {:?}: {}",
                        attempt,
                        runtime_root,
                        e
                    );
                    last_error = Some(e.into());
                }
            }

            if attempt < 3 {
                tokio::time::sleep(Duration::from_millis(800 * attempt as u64)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("failed to build runtime")))
    }

    async fn bootstrap_vectors_if_collection_empty(cortex: &Arc<CortexMem>) -> anyhow::Result<()> {
        let filesystem = cortex.filesystem();
        let mut has_bootstrap_content = false;
        for uri in [
            "cortex://user",
            "cortex://agent",
            "cortex://session",
            "cortex://resources",
        ] {
            if let Ok(entries) = filesystem.list(uri).await {
                if !entries.is_empty() {
                    has_bootstrap_content = true;
                    break;
                }
            }
        }
        if !has_bootstrap_content {
            tracing::info!(
                "Skipping bootstrap vector sync because no bootstrap content exists yet"
            );
            return Ok(());
        }

        let Some(qdrant_store) = cortex.qdrant_store() else {
            return Ok(());
        };
        let Some(embedding_client) = cortex.embedding() else {
            return Ok(());
        };
        let Some(llm_client) = cortex.llm_client() else {
            return Ok(());
        };

        // Always run sync_all to catch any files that were missed due to rate limits or interruptions.
        // The SyncManager will skip files that are already indexed (based on URI hash check).
        tracing::info!("Running bootstrap vector sync to catch any missing embeddings...");
        let sync_manager = SyncManager::new(
            filesystem,
            embedding_client,
            qdrant_store,
            llm_client,
            SyncConfig::default(),
        );
        let stats = sync_manager.sync_all().await?;
        tracing::info!(
            indexed_files = stats.indexed_files,
            skipped_files = stats.skipped_files,
            error_files = stats.error_files,
            total_files = stats.total_files,
            "Bootstrap vector sync completed"
        );
        Ok(())
    }

    /// Load configurations from config file or environment variables
    fn load_configs(
        config_path: &Path,
    ) -> anyhow::Result<(
        Option<Arc<dyn LLMClient>>,
        Option<EmbeddingConfig>,
        Option<QdrantConfig>,
    )> {
        // Try to load from config file first
        if let Ok(config) = cortex_mem_config::Config::load(config_path) {
            tracing::info!("Loaded configuration from {}", config_path.display());

            // LLM client
            let llm_client = {
                let llm_config = cortex_mem_core::llm::client::LLMConfig {
                    api_base_url: config.llm.api_base_url.clone(),
                    api_key: config.llm.api_key.clone(),
                    model_efficient: config.llm.model_efficient.clone(),
                    temperature: 0.1,
                    max_tokens: 4096,
                };
                match cortex_mem_core::llm::LLMClientImpl::new(llm_config) {
                    Ok(client) => {
                        tracing::info!("LLM client initialized from config");
                        Some(Arc::new(client) as Arc<dyn LLMClient>)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize LLM client: {}", e);
                        None
                    }
                }
            };

            // Embedding config
            let embedding_config = EmbeddingConfig {
                api_base_url: config.embedding.api_base_url,
                api_key: config.embedding.api_key,
                model_name: config.embedding.model_name,
                batch_size: config.embedding.batch_size,
                timeout_secs: config.embedding.timeout_secs,
                ..EmbeddingConfig::default()
            };

            // Qdrant config
            let qdrant_config = QdrantConfig {
                url: config.qdrant.url,
                collection_name: config.qdrant.collection_name,
                embedding_dim: config.qdrant.embedding_dim,
                timeout_secs: config.qdrant.timeout_secs,
                api_key: config.qdrant.api_key.clone(),
                tenant_id: None,
            };

            Ok((llm_client, Some(embedding_config), Some(qdrant_config)))
        } else {
            // Fallback to environment variables
            tracing::info!("Loading configuration from environment variables");

            let llm_client = if let (Ok(api_url), Ok(api_key), Ok(model)) = (
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
                match cortex_mem_core::llm::LLMClientImpl::new(config) {
                    Ok(client) => {
                        tracing::info!("LLM client initialized from environment");
                        Some(Arc::new(client) as Arc<dyn LLMClient>)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize LLM client: {}", e);
                        None
                    }
                }
            } else {
                tracing::warn!("LLM environment variables not set, LLM features disabled");
                None
            };

            let embedding_config = if let (Ok(api_base_url), Ok(api_key), Ok(model_name)) = (
                std::env::var("EMBEDDING_API_BASE_URL"),
                std::env::var("EMBEDDING_API_KEY"),
                std::env::var("EMBEDDING_MODEL_NAME"),
            ) {
                Some(EmbeddingConfig {
                    api_base_url,
                    api_key,
                    model_name,
                    batch_size: 10,
                    timeout_secs: 30,
                    ..EmbeddingConfig::default()
                })
            } else {
                tracing::warn!(
                    "Embedding environment variables not set, vector search may be disabled"
                );
                None
            };

            let qdrant_config = if let (Ok(url), Ok(collection_name)) = (
                std::env::var("QDRANT_URL"),
                std::env::var("QDRANT_COLLECTION"),
            ) {
                Some(QdrantConfig {
                    url,
                    collection_name,
                    embedding_dim: None,
                    timeout_secs: 30,
                    api_key: std::env::var("QDRANT_API_KEY").ok(),
                    tenant_id: None,
                })
            } else {
                tracing::warn!("Qdrant environment variables not set, vector search disabled");
                None
            };

            Ok((llm_client, embedding_config, qdrant_config))
        }
    }

    /// Switch to a different tenant by rebuilding a complete tenant-scoped runtime.
    pub async fn switch_tenant(&self, tenant_id: &str) -> anyhow::Result<()> {
        // Check if this tenant has already been bootstrapped
        let needs_bootstrap = {
            let bootstrapped = self.bootstrapped_tenants.read().await;
            !bootstrapped.contains(tenant_id)
        };

        let tenant_root = self.data_dir.join("tenants").join(tenant_id);

        std::fs::create_dir_all(tenant_root.join("agent"))?;
        std::fs::create_dir_all(tenant_root.join("resources"))?;
        std::fs::create_dir_all(tenant_root.join("session"))?;
        std::fs::create_dir_all(tenant_root.join("user"))?;

        let (llm_client, embedding_config, qdrant_config) = Self::load_configs(&self.config_path)?;
        let new_cortex = Arc::new(
            Self::build_runtime(
                &tenant_root,
                Some(tenant_id.to_string()),
                llm_client,
                embedding_config,
                qdrant_config,
            )
            .await?,
        );

        // Run bootstrap vector sync in background only if this tenant hasn't been bootstrapped yet
        if needs_bootstrap {
            // Mark as bootstrapped before starting the background task
            {
                let mut bootstrapped = self.bootstrapped_tenants.write().await;
                bootstrapped.insert(tenant_id.to_string());
            }

            let cortex_for_bg = new_cortex.clone();
            let tenant_id_for_log = tenant_id.to_string();
            tokio::spawn(async move {
                if let Err(e) = Self::bootstrap_vectors_if_collection_empty(&cortex_for_bg).await {
                    tracing::warn!(
                        "Background bootstrap vector sync failed for tenant {}: {}",
                        tenant_id_for_log,
                        e
                    );
                }
            });
        }

        let new_session_manager = new_cortex.session_manager();
        let new_vector_store = new_cortex.vector_store();
        let new_memory_event_tx = new_cortex.memory_event_tx();
        let new_vector_engine = Self::build_vector_engine(&new_cortex, self.enable_intent_analysis);

        {
            let mut cortex_guard = self.cortex.write().await;
            *cortex_guard = new_cortex;
        }
        {
            let mut session_guard = self.session_manager.write().await;
            *session_guard = new_session_manager;
        }
        {
            let mut store_guard = self.vector_store.write().await;
            *store_guard = new_vector_store;
        }
        {
            let mut tx_guard = self.memory_event_tx.write().await;
            *tx_guard = new_memory_event_tx;
        }
        {
            let mut engine_guard = self.vector_engine.write().await;
            *engine_guard = new_vector_engine;
        }
        {
            let mut current = self.current_tenant_root.write().await;
            *current = Some(tenant_root.clone());
        }
        {
            let mut current_id = self.current_tenant_id.write().await;
            *current_id = Some(tenant_id.to_string());
        }

        tracing::info!(
            "✅ Switched to tenant runtime: {} ({:?})",
            tenant_id,
            tenant_root
        );
        Ok(())
    }

    /// List all tenants
    pub async fn list_tenants(&self) -> Vec<String> {
        let tenants_path = self.data_dir.join("tenants");

        let mut tenants = vec![];
        if tenants_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&tenants_path) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        tenants.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
        }

        tenants
    }
}
