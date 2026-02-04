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
pub mod filesystem;
pub mod layers;
pub mod llm;
pub mod retrieval;
pub mod index;
pub mod session;
pub mod extraction;

pub use error::{Error, Result};
pub use types::*;
pub use filesystem::{CortexUri, CortexFilesystem, FilesystemOperations};
pub use layers::{LayerManager, AbstractGenerator, OverviewGenerator};
pub use llm::{LLMClient};
pub use retrieval::{
    Intent, IntentAnalyzer, RelevanceCalculator, 
    RetrievalEngine, RetrievalOptions, RetrievalResult, SearchResult
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
    MemoryType, MemoryImportance,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
