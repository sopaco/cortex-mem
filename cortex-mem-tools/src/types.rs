use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Operation result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> OperationResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> OperationResult<()> {
        OperationResult {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp: Utc::now(),
        }
    }
}

/// Store memory parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMemoryParams {
    pub content: String,
    pub thread_id: Option<String>,
    pub role: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Search memory parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMemoryParams {
    pub query: String,
    pub thread_id: Option<String>,
    pub limit: Option<usize>,
    pub min_score: Option<f32>,
}

/// Memory info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub uri: String,
    pub content: String,
    pub score: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub thread_id: String,
    pub status: String,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
