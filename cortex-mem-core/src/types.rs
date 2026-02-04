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

/// File entry in the virtual filesystem
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
    pub layer: Option<ContextLayer>,
    pub tags: Vec<String>,
}

/// Memory metadata stored in .metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub id: String,
    pub dimension: Dimension,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl MemoryMetadata {
    pub fn new(id: String, dimension: Dimension) -> Self {
        let now = Utc::now();
        Self {
            id,
            dimension,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            summary: None,
            custom: HashMap::new(),
        }
    }
}

/// Memory content
#[derive(Debug, Clone)]
pub struct Memory {
    pub uri: String,
    pub content: String,
    pub layer: ContextLayer,
    pub metadata: Option<MemoryMetadata>,
}
