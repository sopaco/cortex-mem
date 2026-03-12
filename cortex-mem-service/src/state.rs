use cortex_mem_core::{
    CortexFilesystem, CortexMem, CortexMemBuilder, EmbeddingClient,
    EmbeddingConfig, LLMClient, MemoryIndexManager, QdrantConfig,
    SessionManager, VectorSearchEngine,
    memory_events::MemoryEvent,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub cortex: Arc<CortexMem>,
    #[allow(dead_code)]
    pub filesystem: Arc<CortexFilesystem>,
    pub session_manager: Arc<tokio::sync::RwLock<SessionManager>>,
    pub llm_client: Option<Arc<dyn LLMClient>>,
    #[allow(dead_code)]
    pub vector_store: Option<Arc<dyn cortex_mem_core::vector_store::VectorStore>>,
    pub embedding_client: Option<Arc<EmbeddingClient>>,
    /// Vector search engine with L0/L1/L2 layered search support
    /// 使用RwLock支持租户切换时重新创建
    pub vector_engine: Arc<RwLock<Option<Arc<VectorSearchEngine>>>>,
    /// Base data directory
    pub data_dir: PathBuf,
    /// Current tenant root directory (if set)
    pub current_tenant_root: Arc<RwLock<Option<PathBuf>>>,
    /// Current tenant ID (for recreating tenant-specific vector store)
    pub current_tenant_id: Arc<RwLock<Option<String>>>,
    /// Memory event sender — used by close_session handler to trigger memory extraction.
    /// Sending `MemoryEvent::SessionClosed` triggers the full extraction pipeline via
    /// `MemoryEventCoordinator`.
    pub memory_event_tx: Option<tokio::sync::mpsc::UnboundedSender<MemoryEvent>>,
}

impl AppState {
    /// Create new application state with unified automation
    pub async fn new(data_dir: &str) -> anyhow::Result<Self> {
        let data_dir = PathBuf::from(data_dir);

        // 使用Cortex MemoryBuilder统一初始化
        tracing::info!("Initializing Cortex Memory with unified automation...");

        let cortex_dir = data_dir.join("cortex");

        // 获取配置（优先从config.toml，否则从环境变量）
        let (llm_client, embedding_config, qdrant_config) = Self::load_configs()?;

        // 构建Cortex Memory
        let mut builder = CortexMemBuilder::new(&cortex_dir);

        // 配置LLM（如果有）
        if let Some(llm) = llm_client.clone() {
            builder = builder.with_llm(llm);
        }

        // 配置Embedding（如果有）
        if let Some(emb_cfg) = embedding_config {
            builder = builder.with_embedding(emb_cfg);
        }

        // 配置Qdrant（如果有）
        if let Some(qdrant_cfg) = qdrant_config {
            builder = builder.with_qdrant(qdrant_cfg);
        }

        // 使用 MemoryEventCoordinator 进行记忆提取和层级更新
        // 配置协调器（可选，使用默认配置即可）
        // builder = builder.with_coordinator_config(CoordinatorConfig::default());

        // 构建Cortex Memory
        let cortex = builder.build().await?;
        tracing::info!("✅ Cortex Memory initialized with MemoryEventCoordinator");

        // 从Cortex Memory获取组件
        let filesystem = cortex.filesystem();
        let session_manager = cortex.session_manager();
        let embedding_client = cortex.embedding();
        let vector_store = cortex.vector_store();

        let memory_event_tx = cortex.memory_event_tx();

        // Vector search engine — reuse the Qdrant connection from CortexMem builder
        // instead of creating a third connection from config.
        let qdrant_store_typed = cortex.qdrant_store();
        let index_manager = Arc::new(MemoryIndexManager::new(filesystem.clone()));

        let vector_engine = if let (Some(qdrant_arc), Some(ec)) =
            (qdrant_store_typed, &embedding_client)
        {
            let mut engine = if let Some(llm) = &llm_client {
                VectorSearchEngine::with_llm(
                    qdrant_arc,
                    ec.clone(),
                    filesystem.clone(),
                    llm.clone(),
                )
            } else {
                VectorSearchEngine::new(
                    qdrant_arc,
                    ec.clone(),
                    filesystem.clone(),
                )
            };
            // Wire up forgetting-mechanism event tracking and archived-memory filter.
            // Clone the sender so we can still store the original in AppState.
            if let Some(ref tx) = memory_event_tx {
                engine = engine.with_memory_event_tx(tx.clone());
            }
            engine = engine.with_index_manager(index_manager.clone());
            Some(Arc::new(engine))
        } else {
            None
        };

        Ok(Self {
            cortex: Arc::new(cortex),
            filesystem,
            session_manager,
            llm_client,
            vector_store,
            embedding_client,
            vector_engine: Arc::new(RwLock::new(vector_engine)),
            data_dir,
            current_tenant_root: Arc::new(RwLock::new(None)),
            current_tenant_id: Arc::new(RwLock::new(None)),
            memory_event_tx,
        })
    }

    /// Load configurations from config.toml or environment variables
    fn load_configs() -> anyhow::Result<(
        Option<Arc<dyn LLMClient>>,
        Option<EmbeddingConfig>,
        Option<QdrantConfig>,
    )> {
        // Try to load from config.toml first
        if let Ok(config) = cortex_mem_config::Config::load("config.toml") {
            tracing::info!("Loaded configuration from config.toml");

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
                tenant_id: None, // 初始化时不设置租户ID（global）
            };

            Ok((llm_client, Some(embedding_config), Some(qdrant_config)))
        } else {
            // Fallback to environment variables
            tracing::info!("Loading configuration from environment variables");

            // LLM client from env
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
                        tracing::info!("LLM client initialized from env");
                        Some(Arc::new(client) as Arc<dyn LLMClient>)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize LLM client: {}", e);
                        None
                    }
                }
            } else {
                tracing::warn!("LLM client not configured");
                None
            };

            // Embedding config from env
            let embedding_config = if let (Ok(api_url), Ok(api_key), Ok(model)) = (
                std::env::var("EMBEDDING_API_BASE_URL"),
                std::env::var("EMBEDDING_API_KEY"),
                std::env::var("EMBEDDING_MODEL"),
            ) {
                Some(EmbeddingConfig {
                    api_base_url: api_url,
                    api_key,
                    model_name: model,
                    batch_size: 10,
                    timeout_secs: 30,
                    ..EmbeddingConfig::default()
                })
            } else {
                tracing::warn!("Embedding not configured");
                None
            };

            // Qdrant config from env
            let qdrant_config = if let (Ok(url), Ok(collection)) = (
                std::env::var("QDRANT_URL"),
                std::env::var("QDRANT_COLLECTION"),
            ) {
                Some(QdrantConfig {
                    url,
                    collection_name: collection,
                    embedding_dim: std::env::var("QDRANT_EMBEDDING_DIM")
                        .ok()
                        .and_then(|s| s.parse().ok()),
                    timeout_secs: 30,
                    api_key: std::env::var("QDRANT_API_KEY").ok(),
                    tenant_id: None, // 初始化时不设置租户ID（global）
                })
            } else {
                tracing::warn!("Qdrant not configured");
                None
            };

            Ok((llm_client, embedding_config, qdrant_config))
        }
    }

    /// List all available tenants
    pub async fn list_tenants(&self) -> Vec<String> {
        // Try both possible tenant locations
        let possible_paths = vec![
            self.data_dir.join("tenants"),
            self.data_dir.join("cortex").join("tenants"),
        ];

        let mut tenants = vec![];
        for tenants_path in possible_paths {
            if tenants_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&tenants_path) {
                    for entry in entries.flatten() {
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            tenants.push(entry.file_name().to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        tenants
    }

    /// Switch to a different tenant
    /// Recreates VectorSearchEngine with tenant-specific collection
    pub async fn switch_tenant(&self, tenant_id: &str) -> anyhow::Result<()> {
        // Try both possible tenant locations
        let possible_paths = vec![
            self.data_dir.join("tenants").join(tenant_id),
            self.data_dir.join("cortex").join("tenants").join(tenant_id),
        ];

        let mut tenant_root = None;
        for path in possible_paths {
            if path.exists() {
                tenant_root = Some(path);
                break;
            }
        }

        // Auto-provision: if tenant doesn't exist, create it under cortex/tenants/
        let tenant_root = match tenant_root {
            Some(p) => p,
            None => {
                let new_tenant_root = self.data_dir.join("cortex").join("tenants").join(tenant_id);
                tracing::info!("Tenant {} not found, auto-provisioning at {:?}", tenant_id, new_tenant_root);
                for subdir in &["agent", "resources", "session", "user"] {
                    std::fs::create_dir_all(new_tenant_root.join(subdir))
                        .map_err(|e| anyhow::anyhow!("Failed to create tenant dir: {}", e))?;
                }
                tracing::info!("✅ Tenant {} auto-provisioned successfully", tenant_id);
                new_tenant_root
            }
        };

        // Update current tenant root
        let mut current = self.current_tenant_root.write().await;
        *current = Some(tenant_root.clone());
        drop(current);

        // Update current tenant ID
        let mut current_id = self.current_tenant_id.write().await;
        *current_id = Some(tenant_id.to_string());
        drop(current_id);

        tracing::info!("Switched to tenant root: {:?}", tenant_root);

        // Recreate VectorSearchEngine with tenant-specific collection
        if let (Some(ec), Some(llm)) = (&self.embedding_client, &self.llm_client) {
            let (_, _, qdrant_cfg_opt) = Self::load_configs()?;
            if let Some(mut qdrant_cfg) = qdrant_cfg_opt {
                // Set tenant ID in config
                qdrant_cfg.tenant_id = Some(tenant_id.to_string());

                if let Ok(qdrant_store) = cortex_mem_core::QdrantVectorStore::new(&qdrant_cfg).await
                {
                    let qdrant_arc = Arc::new(qdrant_store);

                    // Create tenant-specific filesystem
                    let tenant_filesystem = Arc::new(CortexFilesystem::new(
                        tenant_root.to_string_lossy().as_ref(),
                    ));

                    let new_vector_engine = Arc::new(
                        VectorSearchEngine::with_llm(
                            qdrant_arc,
                            ec.clone(),
                            tenant_filesystem,
                            llm.clone(),
                        )
                        // Re-wire forgetting-mechanism tracking.
                        // Note: no per-tenant memory_event_tx here (the coordinator is global);
                        // we reuse the global index_manager with the tenant filesystem scope.
                        .with_index_manager(Arc::new(MemoryIndexManager::new(
                            self.filesystem.clone(),
                        ))),
                    );

                    // Update vector_engine
                    let mut engine = self.vector_engine.write().await;
                    *engine = Some(new_vector_engine);

                    tracing::info!(
                        "✅ VectorSearchEngine recreated for tenant: {} with collection: {}",
                        tenant_id,
                        qdrant_cfg.get_collection_name()
                    );
                }
            }
        }

        Ok(())
    }

}
