use serde::{Deserialize, Serialize};

/// Store memory request
#[derive(Debug, Clone, Deserialize)]
pub struct StoreMemoryRequest {
    /// Voice transcription content
    pub content: String,
    /// Fixed value "audio_listener" indicating source
    pub source: String,
    /// Timestamp in RFC 3339 format
    pub timestamp: String,
    /// Speaker type: "user" or "other"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_type: Option<String>,
    /// Speaker confidence (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_confidence: Option<f32>,
}

/// Store memory response
#[derive(Debug, Clone, Serialize)]
pub struct StoreMemoryResponse {
    /// Whether storage was successful
    pub success: bool,
    /// Memory ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    /// Success or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// API status
    pub status: String,
    /// Current timestamp
    pub timestamp: String,
}

/// Error response
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    /// Whether successful
    pub success: bool,
    /// Error type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// Error message
    pub error: String,
}

/// Memory item (for query and list responses)
#[derive(Debug, Clone, Serialize)]
pub struct MemoryItem {
    /// Memory ID
    pub id: String,
    /// Memory content
    pub content: String,
    /// Source
    pub source: String,
    /// Timestamp
    pub timestamp: String,
    /// Speaker type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_type: Option<String>,
    /// Speaker confidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_confidence: Option<f32>,
    /// Relevance score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance: Option<f32>,
}

/// Retrieve memory response
#[derive(Debug, Clone, Serialize)]
pub struct RetrieveMemoryResponse {
    /// Memory list
    pub memories: Vec<MemoryItem>,
}

/// List memory response
#[derive(Debug, Clone, Serialize)]
pub struct ListMemoryResponse {
    /// Memory list
    pub memories: Vec<MemoryItem>,
    /// Total count
    pub total: usize,
}
