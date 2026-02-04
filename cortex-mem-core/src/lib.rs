//! Cortex-Mem Core Library
//! 
//! A file-system based memory management system for AI Agents.
//! 
//! # Features
//! 
//! - Virtual filesystem with `cortex://` URI protocol
//! - Three-layer memory architecture (L0/L1/L2)
//! - Recursive retrieval engine
//! - Session management with automatic memory extraction
//! 
//! # Architecture
//! 
//! ```text
//! cortex://
//! ├── agents/{agent_id}/
//! ├── users/{user_id}/
//! ├── threads/{thread_id}/
//! └── global/
//! ```

pub mod error;
pub mod types;
pub mod config;
pub mod filesystem;
pub mod layers;
pub mod llm;
pub mod embedding;
pub mod search;
pub mod retrieval;
pub mod index;
pub mod session;
pub mod extraction;
pub mod automation;

#[cfg(feature = "vector-search")]
pub mod vector_store;

pub use error::{Error, Result};
pub use types::*;
pub use config::*;
pub use filesystem::{CortexUri, CortexFilesystem, FilesystemOperations};
pub use layers::{LayerManager, AbstractGenerator, OverviewGenerator};
pub use llm::{LLMClient};
pub use embedding::{EmbeddingClient, EmbeddingConfig};
pub use search::{SearchOptions, SearchResult};
pub use retrieval::{
    Intent, IntentAnalyzer, RelevanceCalculator, 
    RetrievalEngine, RetrievalOptions, RetrievalResult, SearchResult as RetrievalSearchResult
};
pub use index::{SQLiteIndex, IndexEntry, FullTextIndex, FullTextResult};
pub use session::{
    SessionManager, SessionConfig, SessionMetadata, SessionStatus,
    Message, MessageRole, MessageStorage,
    TimelineGenerator, TimelineEntry, TimelineAggregation,
    Participant, ParticipantRole, ParticipantManager,
};
pub use extraction::{
    MemoryExtractor, ExtractionConfig,
    ExtractedMemories, ExtractedFact, ExtractedDecision, ExtractedEntity,
    MemoryType as ExtractedMemoryType, MemoryImportance,
};
pub use automation::{
    IndexerConfig, IndexStats,
    AutoExtractConfig, AutoExtractStats, AutoExtractor, AutoSessionManager,
};

#[cfg(feature = "vector-search")]
pub use vector_store::{QdrantVectorStore, VectorStore};

#[cfg(feature = "vector-search")]
pub use search::VectorSearchEngine;

#[cfg(feature = "vector-search")]
pub use automation::AutoIndexer;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
