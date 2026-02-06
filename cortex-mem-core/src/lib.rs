//! # Cortex-Mem Core Library
//!
//! Cortex-Mem 是一个基于文件系统的记忆管理系统，支持向量搜索、会话管理和智能记忆提取。
//!
//! ## 主要功能
//!
//! - **文件系统**: 基于 `cortex://` URI 的虚拟文件系统
//! - **向量搜索**: 集成 Qdrant 向量数据库，支持语义搜索
//! - **会话管理**: 多线程会话管理，支持时间轴和参与者
//! - **记忆提取**: 使用 LLM 自动提取和分类记忆
//! - **索引自动化**: 自动监听文件变化并增量索引
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use cortex_mem_core::{CortexFilesystem, FilesystemOperations};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 初始化文件系统
//!     let filesystem = Arc::new(CortexFilesystem::new("./cortex-data"));
//!     filesystem.initialize().await?;
//!     
//!     // 写入数据
//!     filesystem.write("cortex://test.md", "Hello, Cortex!").await?;
//!     
//!     // 读取数据
//!     let content = filesystem.read("cortex://test.md").await?;
//!     println!("Content: {}", content);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## 向量搜索示例
//!
//! ```rust,no_run
//! #[cfg(feature = "vector-search")]
//! use cortex_mem_core::{
//!     QdrantConfig, QdrantVectorStore, VectorStore,
//!     EmbeddingClient, EmbeddingConfig,
//!     types::{Memory, MemoryMetadata, Filters},
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     #[cfg(feature = "vector-search")]
//!     {
//!         // 初始化 Qdrant
//!         let config = QdrantConfig {
//!             url: "http://localhost:6334".to_string(),
//!             collection_name: "cortex-mem".to_string(),
//!             embedding_dim: Some(1536),
//!             timeout_secs: 30,
//!         };
//!         let vector_store = Arc::new(QdrantVectorStore::new(&config).await?);
//!         
//!         // 初始化 Embedding 客户端
//!         let embedding_config = EmbeddingConfig::default();
//!         let embedding_client = Arc::new(EmbeddingClient::new(embedding_config)?);
//!         
//!         // 生成 embedding 并存储
//!         let text = "This is a test memory";
//!         let embedding = embedding_client.embed(text).await?;
//!         
//!         let memory = Memory {
//!             id: "mem-1".to_string(),
//!             content: text.to_string(),
//!             embedding,
//!             created_at: chrono::Utc::now(),
//!             updated_at: chrono::Utc::now(),
//!             metadata: MemoryMetadata::default(),
//!         };
//!         
//!         vector_store.insert(&memory).await?;
//!         
//!         // 搜索相似记忆
//!         let query = "test";
//!         let query_embedding = embedding_client.embed(query).await?;
//!         let results = vector_store.search(
//!             &query_embedding,
//!             &Filters::default(),
//!             10
//!         ).await?;
//!         
//!         println!("Found {} results", results.len());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## 模块说明
//!
//! - [`filesystem`]: 文件系统操作和 URI 处理
//! - [`session`]: 会话管理和消息处理
//! - [`vector_store`]: 向量存储接口（需要 `vector-search` feature）
//! - [`embedding`]: Embedding 生成客户端（需要 `vector-search` feature）
//! - [`search`]: 向量搜索引擎（需要 `vector-search` feature）
//! - [`automation`]: 自动化索引和提取
//! - [`extraction`]: 记忆提取和分类
//! - [`llm`]: LLM 客户端接口
//!
//! ## Feature Flags
//!
//! - `vector-search`: 启用向量搜索功能（需要 Qdrant）

pub mod config;
pub mod error;
pub mod types;
pub mod logging;

pub mod filesystem;
pub mod session;
pub mod extraction;
pub mod llm;
pub mod automation;
pub mod index;
pub mod init;
pub mod layers;
pub mod memory;
pub mod retrieval;

#[cfg(feature = "vector-search")]
pub mod vector_store;

#[cfg(feature = "vector-search")]
pub mod embedding;

#[cfg(feature = "vector-search")]
pub mod search;

// Re-exports
pub use config::*;
pub use error::{Error, Result};
pub use types::*;

pub use filesystem::{CortexFilesystem, FilesystemOperations};
pub use session::{SessionManager, SessionConfig, Message, MessageRole, Participant, ParticipantManager};
pub use extraction::{MemoryExtractor, ExtractionConfig};
pub use llm::LLMClient;
pub use automation::{AutoExtractor, AutoExtractConfig};

#[cfg(feature = "vector-search")]
pub use automation::{IndexerConfig, IndexStats};
pub use layers::LayerManager;
pub use retrieval::{RetrievalEngine, RetrievalOptions, RetrievalResult, SearchResult};

#[cfg(feature = "vector-search")]
pub use vector_store::{VectorStore, QdrantVectorStore};

#[cfg(feature = "vector-search")]
pub use embedding::{EmbeddingClient, EmbeddingConfig};

#[cfg(feature = "vector-search")]
pub use search::{VectorSearchEngine, SearchOptions};

#[cfg(feature = "vector-search")]
pub use automation::{AutoIndexer, FsWatcher, WatcherConfig};

// Session-related re-exports
pub use session::message::MessageStorage;
