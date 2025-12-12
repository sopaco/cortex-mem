use serde_json::{Map, Value, json};
use crate::{MemoryOperationPayload, MemoryToolsError};

/// MCP工具定义
pub struct McpToolDefinition {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
}

/// 获取所有MCP工具的定义
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
                        "description": "User ID associated with the memory (required unless --agent was specified on startup)"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID associated with the memory (optional, defaults to configured agent)"
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["conversational", "procedural", "factual", "semantic", "episodic", "personal"],
                        "description": "Type of memory",
                        "default": "conversational"
                    },
                    "topics": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topics to associate with the memory"
                    }
                },
                "required": ["content"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "memory_id": {"type": "string"},
                    "message": {"type": "string"}
                },
                "required": ["success", "memory_id", "message"]
            })),
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
                        "description": "Maximum number of results to return",
                        "default": 10
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["conversational", "procedural", "factual", "semantic", "episodic", "personal"],
                        "description": "Type of memory to filter by"
                    },
                    "min_salience": {
                        "type": "number",
                        "description": "Minimum salience/importance score threshold (0-1)",
                        "minimum": 0,
                        "maximum": 1
                    },
                    "topics": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topics to filter memories by"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "User ID to filter memories (optional, defaults to configured agent's user)"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID to filter memories (optional, defaults to configured agent)"
                    }
                },
                "required": ["query"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "count": {"type": "number"},
                    "memories": {"type": "array", "items": {"type": "object"}}
                },
                "required": ["success", "count", "memories"]
            })),
        },
        McpToolDefinition {
            name: "list_memories".into(),
            title: Some("List Memories".into()),
            description: Some("Retrieve memories with optional filtering. Adjust the limit parameter to control the number of results returned (default: 100, max: 1000).".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of memories to return (default: 100, max: 1000)",
                        "default": 100,
                        "maximum": 1000
                    },
                    "memory_type": {
                        "type": "string",
                        "enum": ["conversational", "procedural", "factual", "semantic", "episodic", "personal"],
                        "description": "Type of memory to filter by"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "User ID to filter memories (optional, defaults to configured agent's user)"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID to filter memories (optional, defaults to configured agent)"
                    }
                }
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "count": {"type": "number"},
                    "memories": {"type": "array", "items": {"type": "object"}}
                },
                "required": ["success", "count", "memories"]
            })),
        },
        McpToolDefinition {
            name: "get_memory".into(),
            title: Some("Get Memory by ID".into()),
            description: Some("Retrieve a specific memory by its exact ID.".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "memory_id": {
                        "type": "string",
                        "description": "Exact ID of the memory to retrieve (required)"
                    }
                },
                "required": ["memory_id"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "memory": {"type": "object"}
                },
                "required": ["success", "memory"]
            })),
        },
    ]
}

/// 将MCP参数映射到MemoryOperationPayload
pub fn map_mcp_arguments_to_payload(
    arguments: &Map<String, Value>,
    default_agent_id: &Option<String>,
) -> MemoryOperationPayload {
    let mut payload = MemoryOperationPayload::default();

    // 提取公共字段
    if let Some(content) = arguments.get("content").and_then(|v| v.as_str()) {
        payload.content = Some(content.to_string());
    }

    if let Some(query) = arguments.get("query").and_then(|v| v.as_str()) {
        payload.query = Some(query.to_string());
    }

    if let Some(memory_id) = arguments.get("memory_id").and_then(|v| v.as_str()) {
        payload.memory_id = Some(memory_id.to_string());
    }

    // User ID可以从参数提供，或从agent ID派生
    if let Some(user_id) = arguments.get("user_id").and_then(|v| v.as_str()) {
        payload.user_id = Some(user_id.to_string());
    } else if let Some(agent_id) = default_agent_id {
        // 如果设置了agent_id，从中派生user_id
        payload.user_id = Some(format!("user_of_{}", agent_id));
    }

    // Agent ID可以从参数提供，或使用默认值
    if let Some(agent_id) = arguments.get("agent_id").and_then(|v| v.as_str()) {
        payload.agent_id = Some(agent_id.to_string());
    } else {
        payload.agent_id = default_agent_id.clone();
    }

    if let Some(memory_type) = arguments.get("memory_type").and_then(|v| v.as_str()) {
        payload.memory_type = Some(memory_type.to_string());
    }

    if let Some(topics) = arguments.get("topics").and_then(|v| v.as_array()) {
        payload.topics = Some(
            topics
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect(),
        );
    }

    if let Some(keywords) = arguments.get("keywords").and_then(|v| v.as_array()) {
        payload.keywords = Some(
            keywords
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect(),
        );
    }

    if let Some(limit) = arguments.get("limit").and_then(|v| v.as_u64()) {
        payload.limit = Some(limit as usize);
    }

    if let Some(k) = arguments.get("k").and_then(|v| v.as_u64()) {
        payload.k = Some(k as usize);
    }

    if let Some(min_salience) = arguments.get("min_salience").and_then(|v| v.as_f64()) {
        payload.min_salience = Some(min_salience);
    }

    payload
}

/// 将MemoryToolsError转换为MCP错误代码
pub fn tools_error_to_mcp_error_code(error: &MemoryToolsError) -> i32 {
    use MemoryToolsError::*;
    
    match error {
        InvalidInput(_) => -32602,  // Invalid params
        Runtime(_) => -32603,       // Internal error
        MemoryNotFound(_) => -32601, // Method not found (for memory not found)
        Serialization(_) => -32603,  // Internal error
        Core(_) => -32603,          // Internal error
    }
}

/// 获取工具的错误消息
pub fn get_tool_error_message(error: &MemoryToolsError) -> String {
    use MemoryToolsError::*;
    
    match error {
        InvalidInput(msg) => msg.clone(),
        Runtime(msg) => msg.clone(),
        MemoryNotFound(msg) => msg.clone(),
        Serialization(e) => format!("Serialization error: {}", e),
        Core(e) => format!("Core error: {}", e),
    }
}