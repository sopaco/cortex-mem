use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common data structure for memory operation payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperationPayload {
    /// The content to store (for store operations)
    pub content: Option<String>,

    /// The query string (for search/query operations)
    pub query: Option<String>,

    /// Memory ID (for get operations)
    pub memory_id: Option<String>,

    /// User ID for filtering
    pub user_id: Option<String>,

    /// Agent ID for filtering
    pub agent_id: Option<String>,

    /// Type of memory
    pub memory_type: Option<String>,

    /// Topics to filter by
    pub topics: Option<Vec<String>>,

    /// Keywords to filter by
    pub keywords: Option<Vec<String>>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Minimum salience/importance score
    pub min_salience: Option<f64>,

    /// Maximum number of results (alias for limit)
    pub k: Option<usize>,

    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Time range filter: find memories created after this ISO 8601 datetime
    pub created_after: Option<String>,

    /// Time range filter: find memories created before this ISO 8601 datetime
    pub created_before: Option<String>,
}

impl Default for MemoryOperationPayload {
    fn default() -> Self {
        Self {
            content: None,
            query: None,
            memory_id: None,
            user_id: None,
            agent_id: None,
            memory_type: None,
            topics: None,
            keywords: None,
            limit: None,
            min_salience: None,
            k: None,
            metadata: None,
            created_after: None,
            created_before: None,
        }
    }
}

/// Memory operation types supported by the tools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryOperationType {
    Store,
    Query,
    List,
    Get,
}

/// Common response structure for memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperationResponse {
    /// Whether the operation was successful
    pub success: bool,

    /// Message describing the result
    pub message: String,

    /// Optional data payload
    pub data: Option<serde_json::Value>,

    /// Optional error details
    pub error: Option<String>,
}

impl MemoryOperationResponse {
    /// Create a successful response
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            error: None,
        }
    }

    /// Create a successful response with data
    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            message: "Operation failed".to_string(),
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Helper struct to extract query parameters
pub struct QueryParams {
    pub query: String,
    pub limit: usize,
    pub min_salience: Option<f64>,
    pub memory_type: Option<String>,
    pub topics: Option<Vec<String>>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

impl QueryParams {
    pub fn from_payload(payload: &MemoryOperationPayload, default_limit: usize) -> crate::errors::MemoryToolsResult<Self> {
        let query = payload.query.as_ref()
            .ok_or_else(|| crate::errors::MemoryToolsError::InvalidInput("Query is required".to_string()))?
            .clone();

        let limit = payload.limit
            .or(payload.k)
            .unwrap_or(default_limit);

        // Parse time range parameters
        let created_after = payload.created_after.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let created_before = payload.created_before.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        Ok(Self {
            query,
            limit,
            min_salience: payload.min_salience,
            memory_type: payload.memory_type.clone(),
            topics: payload.topics.clone(),
            user_id: payload.user_id.clone(),
            agent_id: payload.agent_id.clone(),
            created_after,
            created_before,
        })
    }
}

/// Helper struct to extract store parameters
pub struct StoreParams {
    pub content: String,
    pub user_id: String,
    pub agent_id: Option<String>,
    pub memory_type: String,
    pub topics: Option<Vec<String>>,
}

impl StoreParams {
    pub fn from_payload(payload: &MemoryOperationPayload, default_user_id: Option<String>, default_agent_id: Option<String>) -> crate::errors::MemoryToolsResult<Self> {
        let content = payload.content.as_ref()
            .ok_or_else(|| crate::errors::MemoryToolsError::InvalidInput("Content is required".to_string()))?
            .clone();

        let user_id = payload.user_id
            .clone()
            .or(default_user_id)
            .ok_or_else(|| crate::errors::MemoryToolsError::InvalidInput("User ID is required".to_string()))?;

        let agent_id = payload.agent_id.clone().or(default_agent_id);

        let memory_type = payload.memory_type
            .clone()
            .unwrap_or_else(|| "conversational".to_string());

        Ok(Self {
            content,
            user_id,
            agent_id,
            memory_type,
            topics: payload.topics.clone(),
        })
    }
}

/// Helper struct to extract filter parameters
pub struct FilterParams {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub memory_type: Option<String>,
    pub limit: usize,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

impl FilterParams {
    pub fn from_payload(payload: &MemoryOperationPayload, default_limit: usize) -> crate::errors::MemoryToolsResult<Self> {
        let limit = payload.limit.or(payload.k).unwrap_or(default_limit);

        // Parse time range parameters
        let created_after = payload.created_after.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let created_before = payload.created_before.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        Ok(Self {
            user_id: payload.user_id.clone(),
            agent_id: payload.agent_id.clone(),
            memory_type: payload.memory_type.clone(),
            limit,
            created_after,
            created_before,
        })
    }
}
