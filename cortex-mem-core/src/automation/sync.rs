#![cfg(feature = "vector-search")]

use crate::{
    embedding::EmbeddingClient,
    filesystem::{CortexFilesystem, FilesystemOperations},
    layers::manager::LayerManager,
    types::{Memory, MemoryMetadata, MemoryType},
    vector_store::QdrantVectorStore,
    Result,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

// Import VectorStore trait to use its methods
use crate::vector_store::VectorStore as _;

/// 自动同步管理器
///
/// 负责：
/// 1. 扫描文件系统中的所有Markdown文件
/// 2. 为未索引的文件生成embedding
/// 3. 批量同步到Qdrant向量数据库
/// 4. 支持增量更新
pub struct SyncManager {
    filesystem: Arc<CortexFilesystem>,
    embedding: Arc<EmbeddingClient>,
    vector_store: Arc<crate::vector_store::QdrantVectorStore>,
    config: SyncConfig,
}

/// 自动同步管理器配置
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// 是否自动索引新消息
    pub auto_index: bool,
    /// 是否同步agents维度
    pub sync_agents: bool,
    /// 是否同步threads维度
    pub sync_threads: bool,
    /// 是否同步users维度
    pub sync_users: bool,
    /// 是否同步global维度
    pub sync_global: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_index: true,
            sync_agents: true,
            sync_threads: true,
            sync_users: true,
            sync_global: true,
        }
    }
}

/// 同步统计
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    pub total_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub error_files: usize,
}

impl SyncManager {
    /// 创建新的自动同步管理器
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        embedding: Arc<EmbeddingClient>,
        vector_store: Arc<crate::vector_store::QdrantVectorStore>,
        config: SyncConfig,
    ) -> Self {
        Self {
            filesystem,
            embedding,
            vector_store,
            config,
        }
    }

    /// 创建默认配置的自动同步管理器
    pub fn with_defaults(
        filesystem: Arc<CortexFilesystem>,
        embedding: Arc<EmbeddingClient>,
        vector_store: Arc<QdrantVectorStore>,
    ) -> Self {
        Self::new(filesystem, embedding, vector_store, SyncConfig::default())
    }

    /// 同步所有内容到向量数据库
    pub async fn sync_all(&self) -> Result<SyncStats> {
        info!("Starting full sync to vector database");

        let mut total_stats = SyncStats::default();

        // 同步用户记忆
        if self.config.sync_users {
            let stats = self
                .sync_directory("cortex://users", MemoryType::Semantic)
                .await?;
            total_stats.add(&stats);
        }

        // 同步Agent记忆
        if self.config.sync_agents {
            let stats = self
                .sync_directory("cortex://agents", MemoryType::Semantic)
                .await?;
            total_stats.add(&stats);
        }

        // 同步会话
        if self.config.sync_threads {
            let stats = self.sync_directory_recursive("cortex://session").await?;
            total_stats.add(&stats);
        }

        // 同步全局记忆
        if self.config.sync_global {
            // global目录可能不存在，如果存在则同步
            if let Ok(entries) = self.filesystem.list("cortex://global").await {
                if !entries.is_empty() {
                    let stats = self
                        .sync_directory("cortex://global", MemoryType::Semantic)
                        .await?;
                    total_stats.add(&stats);
                }
            }
        }

        info!(
            "Sync completed: {} files processed, {} indexed, {} skipped, {} errors",
            total_stats.total_files,
            total_stats.indexed_files,
            total_stats.skipped_files,
            total_stats.error_files
        );

        Ok(total_stats)
    }

    /// 同步单个目录（非递归）
    fn sync_directory<'a>(
        &'a self,
        uri: &'a str,
        memory_type: MemoryType,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SyncStats>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.list(uri).await?;
            let mut stats = SyncStats::default();

            for entry in entries {
                if entry.is_directory {
                    // 递归处理子目录
                    let sub_stats = self.sync_directory(&entry.uri, memory_type.clone()).await?;
                    stats.add(&sub_stats);
                } else if entry.name.ends_with(".md") {
                    // 处理Markdown文件
                    match self.sync_file(&entry.uri, memory_type.clone()).await {
                        Ok(true) => stats.indexed_files += 1,
                        Ok(false) => stats.skipped_files += 1,
                        Err(e) => {
                            warn!("Failed to sync {}: {}", entry.uri, e);
                            stats.error_files += 1;
                        }
                    }
                    stats.total_files += 1;
                }
            }

            Ok(stats)
        })
    }

    /// 同步目录（递归，用于threads）
    fn sync_directory_recursive<'a>(
        &'a self,
        uri: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SyncStats>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.list(uri).await?;
            let mut stats = SyncStats::default();

            // ✅ 新增: 如果是timeline目录,生成L0/L1层
            if uri.contains("/timeline") && !uri.contains(".md") {
                if let Err(e) = self.generate_timeline_layers(uri).await {
                    warn!("Failed to generate timeline layers for {}: {}", uri, e);
                } else {
                    info!("Generated timeline layers for {}", uri);
                }
            }

            for entry in entries {
                if entry.is_directory {
                    // 递归处理子目录
                    let sub_stats = self.sync_directory_recursive(&entry.uri).await?;
                    stats.add(&sub_stats);
                } else if entry.name.ends_with(".md") {
                    // 处理Markdown文件
                    match self.sync_file(&entry.uri, MemoryType::Conversational).await {
                        Ok(true) => stats.indexed_files += 1,
                        Ok(false) => stats.skipped_files += 1,
                        Err(e) => {
                            warn!("Failed to sync {}: {}", entry.uri, e);
                            stats.error_files += 1;
                        }
                    }
                    stats.total_files += 1;
                }
            }

            Ok(stats)
        })
    }

    /// 同步单个文件（支持分层向量索引）
    async fn sync_file(&self, uri: &str, memory_type: MemoryType) -> Result<bool> {
        // 检查是否已经索引（检查L2层）
        let l2_id = format!("{}#L2", uri);
        if self.is_indexed(&l2_id).await? {
            debug!("File already indexed: {}", uri);
            return Ok(false);
        }

        // 1. 读取并索引L2原始内容
        let l2_content = self.filesystem.read(uri).await?;
        let l2_embedding = self.embedding.embed(&l2_content).await?;
        let l2_metadata = self.parse_metadata(uri, memory_type.clone(), "L2")?;

        let l2_memory = Memory {
            id: l2_id.clone(),
            content: l2_content.clone(),
            embedding: l2_embedding,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: l2_metadata,
        };
        self.vector_store.insert(&l2_memory).await?;
        debug!("L2 indexed: {}", uri);

        // 2. 尝试读取并索引L0 abstract
        let abstract_uri = Self::get_layer_uri(uri, "L0");
        if let Ok(l0_content) = self.filesystem.read(&abstract_uri).await {
            let l0_id = format!("{}#L0", uri);
            if !self.is_indexed(&l0_id).await? {
                let l0_embedding = self.embedding.embed(&l0_content).await?;
                let l0_metadata = self.parse_metadata(uri, memory_type.clone(), "L0")?;

                let l0_memory = Memory {
                    id: l0_id,
                    content: l0_content,
                    embedding: l0_embedding,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    metadata: l0_metadata,
                };
                self.vector_store.insert(&l0_memory).await?;
                debug!("L0 indexed: {}", abstract_uri);
            }
        }

        // 3. 尝试读取并索引L1 overview
        let overview_uri = Self::get_layer_uri(uri, "L1");
        if let Ok(l1_content) = self.filesystem.read(&overview_uri).await {
            let l1_id = format!("{}#L1", uri);
            if !self.is_indexed(&l1_id).await? {
                let l1_embedding = self.embedding.embed(&l1_content).await?;
                let l1_metadata = self.parse_metadata(uri, memory_type.clone(), "L1")?;

                let l1_memory = Memory {
                    id: l1_id,
                    content: l1_content,
                    embedding: l1_embedding,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    metadata: l1_metadata,
                };
                self.vector_store.insert(&l1_memory).await?;
                debug!("L1 indexed: {}", overview_uri);
            }
        }

        Ok(true)
    }

    /// 获取分层文件URI
    fn get_layer_uri(base_uri: &str, layer: &str) -> String {
        let dir = base_uri
            .rsplit_once('/')
            .map(|(dir, _)| dir)
            .unwrap_or(base_uri);
        match layer {
            "L0" => format!("{}/.abstract.md", dir),
            "L1" => format!("{}/.overview.md", dir),
            _ => base_uri.to_string(),
        }
    }

    /// 检查文件是否已索引
    async fn is_indexed(&self, uri: &str) -> Result<bool> {
        // 尝试从向量数据库查询
        match self.vector_store.get(uri).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => {
                debug!("Error checking if indexed: {}", e);
                Ok(false)
            }
        }
    }

    /// 解析URI获取元数据（支持layer标识）
    fn parse_metadata(
        &self,
        uri: &str,
        memory_type: MemoryType,
        layer: &str,
    ) -> Result<MemoryMetadata> {
        use serde_json::Value;

        // 从URI中提取信息
        // 格式: cortex://dimension/path/to/file.md
        let parts: Vec<&str> = uri.split('/').collect();

        let (dimension, path): (&str, String) = if parts.len() >= 3 {
            (parts[2], parts[3..].join("/"))
        } else {
            (
                "threads",
                uri.strip_prefix("cortex://").unwrap_or(uri).to_string(),
            )
        };

        let hash = self.calculate_hash(uri);

        let mut custom = std::collections::HashMap::new();
        custom.insert("uri".to_string(), Value::String(uri.to_string()));
        custom.insert("path".to_string(), Value::String(path.clone()));
        custom.insert("layer".to_string(), Value::String(layer.to_string()));

        Ok(MemoryMetadata {
            user_id: if dimension == "users" {
                Some(path.clone())
            } else {
                None
            },
            agent_id: if dimension == "agents" {
                Some(path.clone())
            } else {
                None
            },
            run_id: if dimension == "threads" {
                Some(path.clone())
            } else {
                None
            },
            actor_id: None,
            role: None,
            memory_type,
            hash,
            importance_score: 0.5,
            entities: vec![],
            topics: vec![],
            custom,
        })
    }

    /// 计算内容的哈希值
    fn calculate_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// 为timeline目录生成L0/L1层
    ///
    /// 调用LayerManager生成timeline级别的abstract和overview
    async fn generate_timeline_layers(&self, timeline_uri: &str) -> Result<()> {
        let layer_manager = LayerManager::new(self.filesystem.clone());
        layer_manager.generate_timeline_layers(timeline_uri).await
    }
}

impl SyncStats {
    pub fn add(&mut self, other: &SyncStats) {
        self.total_files += other.total_files;
        self.indexed_files += other.indexed_files;
        self.skipped_files += other.skipped_files;
        self.error_files += other.error_files;
    }
}
