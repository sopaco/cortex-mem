#![cfg(feature = "vector-search")]

use crate::{
    embedding::EmbeddingClient,
    filesystem::{CortexFilesystem, FilesystemOperations},
    session::Message,
    vector_store::{QdrantVectorStore, VectorStore},
    Result,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 自动索引管理器配置
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// 是否自动索引新消息
    pub auto_index: bool,
    /// 批量索引的batch大小
    pub batch_size: usize,
    /// 是否在后台异步索引
    pub async_index: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            auto_index: true,
            batch_size: 10,
            async_index: true,
        }
    }
}

/// 索引统计
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub total_indexed: usize,
    pub total_skipped: usize,
    pub total_errors: usize,
}

/// 自动索引管理器
///
/// 负责：
/// 1. 监听新消息并自动生成embedding
/// 2. 批量索引现有消息
/// 3. 增量更新索引
#[cfg(feature = "vector-search")]
pub struct AutoIndexer {
    filesystem: Arc<CortexFilesystem>,
    embedding: Arc<EmbeddingClient>,
    vector_store: Arc<QdrantVectorStore>,
    config: IndexerConfig,
}

#[cfg(feature = "vector-search")]
impl AutoIndexer {
    /// 创建新的自动索引器
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        embedding: Arc<EmbeddingClient>,
        vector_store: Arc<QdrantVectorStore>,
        config: IndexerConfig,
    ) -> Self {
        Self {
            filesystem,
            embedding,
            vector_store,
            config,
        }
    }

    /// 索引单个消息
    pub async fn index_message(&self, thread_id: &str, message: &Message) -> Result<()> {
        info!("Indexing message {} in thread {}", message.id, thread_id);

        // 1. 生成embedding
        let embedding = self.embedding.embed(&message.content).await?;

        // 2. 创建Memory对象
        let memory = crate::types::Memory {
            id: message.id.clone(),
            content: message.content.clone(),
            embedding,
            created_at: message.created_at,
            updated_at: message.created_at,
            metadata: crate::types::MemoryMetadata {
                user_id: None,
                agent_id: None,
                run_id: Some(thread_id.to_string()),
                actor_id: None,
                role: Some(format!("{:?}", message.role)),
                memory_type: crate::types::MemoryType::Conversational,
                hash: self.calculate_hash(&message.content),
                importance_score: 0.5,
                entities: vec![],
                topics: vec![],
                custom: std::collections::HashMap::new(),
            },
        };

        // 3. 存储到向量数据库
        self.vector_store.as_ref().insert(&memory).await?;

        debug!("Message {} indexed successfully", message.id);
        Ok(())
    }

    /// 批量索引线程中的所有消息
    pub async fn index_thread(&self, thread_id: &str) -> Result<IndexStats> {
        self.index_thread_with_progress::<fn(usize, usize)>(thread_id, None)
            .await
    }

    /// 批量索引线程中的所有消息，带进度回调
    pub async fn index_thread_with_progress<F>(
        &self,
        thread_id: &str,
        mut progress_callback: Option<F>,
    ) -> Result<IndexStats>
    where
        F: FnMut(usize, usize) + Send,
    {
        info!("Starting batch indexing for thread: {}", thread_id);

        let mut stats = IndexStats::default();

        // 1. 扫描timeline目录获取所有消息
        let messages = self.collect_messages(thread_id).await?;
        let total_messages = messages.len();
        info!("Found {} messages to index", total_messages);

        if total_messages == 0 {
            return Ok(stats);
        }

        // 2. 检查哪些消息已经被索引（通过查询向量数据库）
        let existing_ids = self.get_indexed_message_ids(thread_id).await?;
        let messages_to_index: Vec<_> = messages
            .into_iter()
            .filter(|m| !existing_ids.contains(&m.id))
            .collect();

        info!(
            "Skipping {} already indexed messages",
            total_messages - messages_to_index.len()
        );
        stats.total_skipped = total_messages - messages_to_index.len();

        if messages_to_index.is_empty() {
            info!("All messages already indexed");
            return Ok(stats);
        }

        // 3. 分批处理
        let total_to_index = messages_to_index.len();
        for (batch_idx, chunk) in messages_to_index.chunks(self.config.batch_size).enumerate() {
            let batch_start = batch_idx * self.config.batch_size;

            // 通知进度
            if let Some(ref mut callback) = progress_callback {
                callback(batch_start, total_to_index);
            }

            // 生成所有embedding
            let contents: Vec<String> = chunk.iter().map(|m| m.content.clone()).collect();

            match self.embedding.embed_batch(&contents).await {
                Ok(embeddings) => {
                    // 为每个消息创建Memory并存储
                    for (message, embedding) in chunk.iter().zip(embeddings.iter()) {
                        let memory = crate::types::Memory {
                            id: message.id.clone(),
                            content: message.content.clone(),
                            embedding: embedding.clone(),
                            created_at: message.created_at,
                            updated_at: message.created_at,
                            metadata: crate::types::MemoryMetadata {
                                user_id: None,
                                agent_id: None,
                                run_id: Some(thread_id.to_string()),
                                actor_id: None,
                                role: Some(format!("{:?}", message.role)),
                                memory_type: crate::types::MemoryType::Conversational,
                                hash: self.calculate_hash(&message.content),
                                importance_score: 0.5,
                                entities: vec![],
                                topics: vec![],
                                custom: std::collections::HashMap::new(),
                            },
                        };

                        match self.vector_store.as_ref().insert(&memory).await {
                            Ok(_) => {
                                stats.total_indexed += 1;
                                debug!("Indexed message {}", message.id);
                            }
                            Err(e) => {
                                warn!("Failed to index message {}: {}", message.id, e);
                                stats.total_errors += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to generate embeddings for batch {}: {}",
                        batch_idx, e
                    );
                    stats.total_errors += chunk.len();
                }
            }
        }

        info!(
            "Batch indexing complete: {} indexed, {} skipped, {} errors",
            stats.total_indexed, stats.total_skipped, stats.total_errors
        );

        Ok(stats)
    }

    /// 获取已索引的消息ID列表
    async fn get_indexed_message_ids(
        &self,
        thread_id: &str,
    ) -> Result<std::collections::HashSet<String>> {
        use crate::vector_store::VectorStore;

        // 使用scroll API获取所有已索引的消息ID
        let filters = crate::types::Filters {
            run_id: Some(thread_id.to_string()),
            ..Default::default()
        };

        // 滚动查询获取所有ID（不需要embedding）
        match self.vector_store.as_ref().scroll_ids(&filters, 1000).await {
            Ok(ids) => Ok(ids.into_iter().collect()),
            Err(e) => {
                warn!(
                    "Failed to get indexed message IDs: {}, assuming none indexed",
                    e
                );
                Ok(std::collections::HashSet::new())
            }
        }
    }

    /// 收集线程中的所有消息
    async fn collect_messages(&self, thread_id: &str) -> Result<Vec<Message>> {
        let timeline_uri = format!("cortex://session/{}/timeline", thread_id);
        let mut messages = Vec::new();

        self.collect_messages_recursive(&timeline_uri, &mut messages)
            .await?;

        Ok(messages)
    }

    /// 递归收集消息
    fn collect_messages_recursive<'a>(
        &'a self,
        uri: &'a str,
        messages: &'a mut Vec<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.as_ref().list(uri).await?;

            for entry in entries {
                if entry.is_directory && !entry.name.starts_with('.') {
                    self.collect_messages_recursive(&entry.uri, messages)
                        .await?;
                } else if entry.name.ends_with(".md") && !entry.name.starts_with('.') {
                    if let Ok(content) = self.filesystem.as_ref().read(&entry.uri).await {
                        if let Some(message) = self.parse_message_markdown(&content) {
                            messages.push(message);
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// 解析Markdown格式的消息
    fn parse_message_markdown(&self, content: &str) -> Option<Message> {
        use crate::session::MessageRole;

        let mut role = MessageRole::User;
        let mut message_content = String::new();
        let mut id = String::new();
        let mut timestamp = chrono::Utc::now();

        for line in content.lines() {
            if line.starts_with("# 👤 User") {
                role = MessageRole::User;
            } else if line.starts_with("# 🤖 Assistant") {
                role = MessageRole::Assistant;
            } else if line.starts_with("# 🔧 System") {
                role = MessageRole::System;
            } else if line.starts_with("**ID**: `") {
                if let Some(id_str) = line
                    .strip_prefix("**ID**: `")
                    .and_then(|s| s.strip_suffix("`"))
                {
                    id = id_str.to_string();
                }
            } else if line.starts_with("**Timestamp**: ") {
                if let Some(ts_str) = line.strip_prefix("**Timestamp**: ") {
                    if let Ok(parsed_ts) =
                        chrono::DateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S %Z")
                    {
                        timestamp = parsed_ts.with_timezone(&chrono::Utc);
                    }
                }
            } else if line.starts_with("## Content") {
                // 内容开始
            } else if !line.starts_with('#') && !line.starts_with("**") && !line.trim().is_empty() {
                if !message_content.is_empty() {
                    message_content.push('\n');
                }
                message_content.push_str(line);
            }
        }

        if !id.is_empty() && !message_content.is_empty() {
            Some(Message {
                id,
                role,
                content: message_content.trim().to_string(),
                timestamp,
                created_at: timestamp,
                metadata: None,
            })
        } else {
            None
        }
    }

    /// 计算内容哈希
    fn calculate_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
