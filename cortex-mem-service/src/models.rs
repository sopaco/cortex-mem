use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }
}

/// File entry response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntryResponse {
    pub uri: String,
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: DateTime<Utc>,
    /// L0 abstract text (only included when include_abstracts=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
}

impl From<cortex_mem_core::FileEntry> for FileEntryResponse {
    fn from(entry: cortex_mem_core::FileEntry) -> Self {
        Self {
            uri: entry.uri,
            name: entry.name,
            is_directory: entry.is_directory,
            size: entry.size,
            modified: entry.modified,
            abstract_text: None,
        }
    }
}

/// Session response
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub thread_id: String,
    pub status: String,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Close-and-wait request
#[derive(Debug, Deserialize)]
pub struct CloseAndWaitRequest {
    /// Maximum seconds to wait for extraction + indexing readiness.
    #[serde(default = "default_close_wait_timeout_secs")]
    pub timeout_secs: u64,
    /// Poll interval in milliseconds.
    #[serde(default = "default_close_wait_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

fn default_close_wait_timeout_secs() -> u64 {
    120
}

fn default_close_wait_poll_interval_ms() -> u64 {
    500
}

/// Close-and-wait response
#[derive(Debug, Serialize)]
pub struct CloseAndWaitResponse {
    pub thread_id: String,
    pub status: String,
    pub user_id: String,
    pub agent_id: String,
    pub waited_ms: u64,
    pub user_index_exists: bool,
    pub user_memory_count: usize,
    pub session_summary_exists: bool,
    pub session_summary_memory_count: usize,
    pub vector_sync_confirmed: bool,
    pub timeline_abstract_exists: bool,
    pub timeline_overview_exists: bool,
}

/// Message request
#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
    /// Optional metadata (tags, importance, etc.)
    /// Note: Currently accepted but not processed by the backend.
    /// Reserved for future enhancement.
    #[serde(default)]
    #[allow(dead_code)]
    pub metadata: Option<serde_json::Value>,
}

/// Search request
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub thread: Option<String>,
    pub limit: Option<usize>,
    pub min_score: Option<f32>,
    /// Which layers to return: ["L0"], ["L0","L1"], ["L0","L1","L2"]
    /// Default: ["L0"] (only snippets)
    #[serde(default = "default_return_layers")]
    pub return_layers: Vec<String>,
}

fn default_return_layers() -> Vec<String> {
    vec!["L0".to_string()]
}

/// Search result response
#[derive(Debug, Serialize)]
pub struct SearchResultResponse {
    pub uri: String,
    pub score: f32,
    /// L0 abstract/snippet
    pub snippet: String,
    /// L1 overview text (only when return_layers contains "L1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    /// L2 full content (only when return_layers contains "L2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub source: String,
    /// Which layers are included in this result
    pub layers: Vec<String>,
}

/// List directory request
#[derive(Debug, Deserialize)]
pub struct LsRequest {
    /// Directory URI to list
    #[serde(default = "default_uri")]
    pub uri: String,
    /// Whether to recursively list subdirectories
    #[serde(default)]
    pub recursive: bool,
    /// Whether to include L0 abstracts for files
    #[serde(default)]
    pub include_abstracts: bool,
    /// Whether to include layer files (.abstract.md for L0, .overview.md for L1)
    #[serde(default)]
    pub include_layers: bool,
}

fn default_uri() -> String {
    "cortex://session".to_string()
}

/// List directory response
#[derive(Debug, Serialize)]
pub struct LsResponse {
    pub uri: String,
    pub total: usize,
    pub entries: Vec<FileEntryResponse>,
}

/// Explore request
#[derive(Debug, Deserialize)]
pub struct ExploreRequest {
    pub query: String,
    /// Starting URI for exploration
    #[serde(default = "default_explore_start")]
    pub start_uri: String,
    /// Which layers to return in matches
    #[serde(default = "default_return_layers")]
    pub return_layers: Vec<String>,
}

fn default_explore_start() -> String {
    "cortex://session".to_string()
}

/// Explore response
#[derive(Debug, Serialize)]
pub struct ExploreResponse {
    pub query: String,
    /// Path taken during exploration
    pub exploration_path: Vec<ExplorationPathItem>,
    /// Matching results found
    pub matches: Vec<SearchResultResponse>,
    /// Total items explored
    pub total_explored: usize,
    /// Total matches found
    pub total_matches: usize,
}

/// Item in exploration path
#[derive(Debug, Serialize)]
pub struct ExplorationPathItem {
    pub uri: String,
    pub relevance_score: f32,
    pub abstract_text: Option<String>,
}
