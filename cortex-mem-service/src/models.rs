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
}

impl From<cortex_mem_core::FileEntry> for FileEntryResponse {
    fn from(entry: cortex_mem_core::FileEntry) -> Self {
        Self {
            uri: entry.uri,
            name: entry.name,
            is_directory: entry.is_directory,
            size: entry.size,
            modified: entry.modified,
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

/// Message request
#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
}

/// Search request
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub thread: Option<String>,
    pub limit: Option<usize>,
    pub min_score: Option<f32>,
}

/// Search result response
#[derive(Debug, Serialize)]
pub struct SearchResultResponse {
    pub uri: String,
    pub score: f32,
    pub snippet: String,
    pub content: Option<String>,
    pub source: String,
}
