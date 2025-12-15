use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to create a new memory
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub role: Option<String>,
    pub memory_type: Option<String>,
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Request to update an existing memory
#[derive(Debug, Deserialize)]
pub struct UpdateMemoryRequest {
    pub content: String,
}

/// Request to batch delete memories
#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<String>,
}

/// Request to batch update memories
#[derive(Debug, Deserialize)]
pub struct BatchUpdateRequest {
    pub updates: Vec<MemoryUpdate>,
}

/// Single memory update for batch operation
#[derive(Debug, Deserialize)]
pub struct MemoryUpdate {
    pub id: String,
    pub content: String,
}

/// Response for batch operations
#[derive(Debug, Serialize)]
pub struct BatchOperationResponse {
    pub success_count: usize,
    pub failure_count: usize,
    pub errors: Vec<String>,
    pub message: String,
}

/// Request to search memories
#[derive(Debug, Deserialize)]
pub struct SearchMemoryRequest {
    pub query: String,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub memory_type: Option<String>,
    pub limit: Option<usize>,
    pub similarity_threshold: Option<f32>,
}

/// Query parameters for listing memories
#[derive(Debug, Deserialize)]
pub struct ListMemoryQuery {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub memory_type: Option<String>,
    pub limit: Option<usize>,
}

/// Response for memory operations
#[derive(Debug, Serialize)]
pub struct MemoryResponse {
    pub id: String,
    pub content: String,
    pub metadata: MemoryMetadataResponse,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for memory metadata
#[derive(Debug, Serialize)]
pub struct MemoryMetadataResponse {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub role: Option<String>,
    pub memory_type: String,
    pub hash: String,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Response for search results
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<ScoredMemoryResponse>,
    pub total: usize,
}

/// Response for scored memory
#[derive(Debug, Serialize)]
pub struct ScoredMemoryResponse {
    pub memory: MemoryResponse,
    pub score: f32,
}

/// Response for list results
#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub memories: Vec<MemoryResponse>,
    pub total: usize,
}

/// Response for successful operations
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
    pub id: Option<String>,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub vector_store: bool,
    pub llm_service: bool,
    pub timestamp: String,
}


