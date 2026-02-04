use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dimension of memory storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dimension {
    /// Agent-specific memories
    Agents,
    /// User-specific memories
    Users,
    /// Thread/conversation memories
    Threads,
    /// Global shared memories
    Global,
}

impl Dimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Dimension::Agents => "agents",
            Dimension::Users => "users",
            Dimension::Threads => "threads",
            Dimension::Global => "global",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "agents" => Some(Dimension::Agents),
            "users" => Some(Dimension::Users),
            "threads" => Some(Dimension::Threads),
            "global" => Some(Dimension::Global),
            _ => None,
        }
    }
}

/// Context layer for tiered loading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextLayer {
    /// L0: Abstract (~100 tokens)
    L0Abstract,
    /// L1: Overview (~500-2000 tokens)
    L1Overview,
    /// L2: Full detail
    L2Detail,
}

impl ContextLayer {
    pub fn filename(&self) -> &'static str {
        match self {
            ContextLayer::L0Abstract => ".abstract.md",
            ContextLayer::L1Overview => ".overview.md",
            ContextLayer::L2Detail => "",
        }
    }
    
    pub fn max_tokens(&self) -> usize {
        match self {
            ContextLayer::L0Abstract => 100,
            ContextLayer::L1Overview => 2000,
            ContextLayer::L2Detail => usize::MAX,
        }
    }
}

/// File entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub uri: String,
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub size: u64,
    pub is_directory: bool,
}

/// Memory metadata (for V1 compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub role: Option<String>,
    pub memory_type: MemoryType,
    pub hash: String,
    pub importance_score: f32,
    pub entities: Vec<String>,
    pub topics: Vec<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Memory type (for V1 compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    Conversational,
    Procedural,
    Semantic,
    Episodic,
}

impl MemoryType {
    pub fn parse(s: &str) -> Self {
        match s {
            "Conversational" => MemoryType::Conversational,
            "Procedural" => MemoryType::Procedural,
            "Semantic" => MemoryType::Semantic,
            "Episodic" => MemoryType::Episodic,
            _ => MemoryType::Conversational, // Default fallback
        }
    }
}

/// Memory struct (for vector store)
#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: MemoryMetadata,
}

/// Scored memory (search result)
#[derive(Debug, Clone)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}

/// Filters for memory search
#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub topics: Option<Vec<String>>,
    pub entities: Option<Vec<String>>,
    pub min_importance: Option<f32>,
    pub max_importance: Option<f32>,
    pub custom: HashMap<String, serde_json::Value>,
}
