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
//! ## 模块说明
//!
//! - [`filesystem`]: 文件系统操作和 URI 处理
//! - [`session`]: 会话管理和消息处理
//! - [`vector_store`]: 向量存储接口
//! - [`embedding`]: Embedding 生成客户端
//! - [`search`]: 向量搜索引擎
//! - [`automation`]: 自动化索引和提取
//! - [`extraction`]: 记忆提取和分类
//! - [`llm`]: LLM 客户端接口

pub mod config;
pub mod error;
pub mod events;
pub mod logging;
pub mod types;

pub mod automation;
pub mod builder;
pub mod embedding;
pub mod extraction;
pub mod filesystem;
pub mod layers;
pub mod llm;
pub mod search;
pub mod session;
pub mod vector_store;

// Re-exports
pub use config::*;
pub use error::{Error, Result};
pub use events::{CortexEvent, EventBus, FilesystemEvent, SessionEvent};
pub use types::*;

pub use automation::{
    AutoExtractConfig, AutoExtractor, AutoIndexer, AutomationConfig, AutomationManager, FsWatcher,
    IndexStats, IndexerConfig, SyncConfig, SyncManager, SyncStats, WatcherConfig,
};
pub use builder::{CortexMem, CortexMemBuilder};
pub use extraction::ExtractionConfig;
// Note: MemoryExtractor is also exported from session module
pub use embedding::{EmbeddingClient, EmbeddingConfig};
pub use filesystem::{CortexFilesystem, FilesystemOperations};
pub use llm::LLMClient;
pub use search::{SearchOptions, VectorSearchEngine};
pub use session::{
    CaseMemory, EntityMemory, EventMemory, ExtractedMemories, MemoryExtractor, Message,
    MessageRole, Participant, ParticipantManager, PreferenceMemory, SessionConfig, SessionManager,
};
pub use vector_store::{QdrantVectorStore, VectorStore, parse_vector_id, uri_to_vector_id};

// Session-related re-exports
pub use session::message::MessageStorage;
