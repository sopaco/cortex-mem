use serde_json::{Map, Value, json};
use crate::{ToolsError};
use serde::{Deserialize, Serialize};

/// Memory operation payload for MCP compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperationPayload {
    pub content: Option<String>,
    pub query: Option<String>,
    pub memory_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub memory_type: Option<String>,
    pub topics: Option<Vec<String>>,
    pub k: Option<usize>,
    pub limit: Option<usize>,
    pub min_salience: Option<f64>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
}

/// Memory tools error (alias for ToolsError)
pub type MemoryToolsError = ToolsError;

/// MCP tool definition
pub struct McpToolDefinition {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
}

/// Get all MCP tool definitions
pub fn get_mcp_tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "store_memory".into(),
            title: Some("Store Memory".into()),
            description: Some("Store a new memory in the system with specified content and optional metadata.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The content of the memory to store"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "User ID associated with the memory"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID associated with the memory"
                    },
                    "memory_type": {
                        "type": "string",
                        "description": "Type of memory"
                    },
                    "topics": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topics to associate with the memory"
                    }
                },
                "required": ["content"]
            }),
            output_schema: None,
        },
        McpToolDefinition {
            name: "query_memory".into(),
            title: Some("Query Memory".into()),
            description: Some("Search memories using semantic similarity and filters.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Query string for semantic search"
                    },
                    "k": {
                        "type": "integer",
                        "description": "Maximum number of results to return"
                    },
                    "memory_type": {
                        "type": "string",
                        "description": "Type of memory to filter by"
                    },
                    "min_salience": {
                        "type": "number",
                        "description": "Minimum salience/importance score threshold (0-1)"
                    },
                    "topics": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topics to filter memories by"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "User ID to filter memories"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID to filter memories"
                    }
                },
                "required": ["query"]
            }),
            output_schema: None,
        },
        McpToolDefinition {
            name: "list_memories".into(),
            title: Some("List Memories".into()),
            description: Some("Retrieve memories with optional filtering.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of memories to return"
                    },
                    "memory_type": {
                        "type": "string",
                        "description": "Type of memory to filter by"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "User ID to filter memories"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID to filter memories"
                    },
                    "created_after": {
                        "type": "string",
                        "description": "Filter memories created after this timestamp"
                    },
                    "created_before": {
                        "type": "string",
                        "description": "Filter memories created before this timestamp"
                    }
                },
                "required": []
            }),
            output_schema: None,
        },
        McpToolDefinition {
            name: "get_memory".into(),
            title: Some("Get Memory".into()),
            description: Some("Retrieve a specific memory by its exact ID.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "memory_id": {
                        "type": "string",
                        "description": "Exact ID of the memory to retrieve"
                    }
                },
                "required": ["memory_id"]
            }),
            output_schema: None,
        },
    ]
}

/// Map MCP arguments to memory operation payload
pub fn map_mcp_arguments_to_payload(
    arguments: &Map<String, Value>,
    default_agent_id: &Option<String>,
) -> MemoryOperationPayload {
    MemoryOperationPayload {
        content: arguments.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()),
        query: arguments.get("query").and_then(|v| v.as_str()).map(|s| s.to_string()),
        memory_id: arguments.get("memory_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        user_id: arguments.get("user_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        agent_id: arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| default_agent_id.clone()),
        memory_type: arguments.get("memory_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
        topics: arguments
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }),
        k: arguments.get("k").and_then(|v| v.as_u64()).map(|n| n as usize),
        limit: arguments.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize),
        min_salience: arguments.get("min_salience").and_then(|v| v.as_f64()),
        created_after: arguments.get("created_after").and_then(|v| v.as_str()).map(|s| s.to_string()),
        created_before: arguments.get("created_before").and_then(|v| v.as_str()).map(|s| s.to_string()),
    }
}

/// Convert MemoryToolsError to MCP error code
pub fn tools_error_to_mcp_error_code(error: &MemoryToolsError) -> i32 {
    use ToolsError::*;
    match error {
        InvalidInput(_) => -32602, // Invalid params
        Runtime(_) => -32603,      // Internal error
        NotFound(_) => -32001,     // Custom: Not found
        Serialization(_) => -32700, // Parse error
        Core(_) => -32603,         // Internal error
        Io(_) => -32603,           // Internal error
    }
}

/// Get tool error message
pub fn get_tool_error_message(error: &MemoryToolsError) -> String {
    error.to_string()
}
