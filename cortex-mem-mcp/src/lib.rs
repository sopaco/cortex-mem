use anyhow::Result;
use cortex_mem_config::Config;
use cortex_mem_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
};
use cortex_mem_tools::{MemoryOperations, MemoryOperationPayload, MemoryToolsError};
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorData, ListToolsResult,
        PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    RoleServer, ServerHandler,
};
use serde_json::Map;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

/// Service for handling MCP tool calls related to memory management
pub struct MemoryMcpService {
    memory_manager: Arc<MemoryManager>,
    operations: MemoryOperations,
    agent_id: Option<String>,
}

impl MemoryMcpService {
    /// Create a new memory MCP service with default config path
    pub async fn new() -> Result<Self> {
        // Try to find config.toml in standard locations
        let config_path = Self::find_default_config_path()
            .unwrap_or_else(|| Path::new("config.toml").to_path_buf());
        Self::with_config_path(config_path).await
    }

    /// Create a new memory MCP service with specific config path
    pub async fn with_config_path<P: AsRef<Path> + Clone + std::fmt::Debug>(
        path: P,
    ) -> Result<Self> {
        Self::with_config_path_and_agent(path, None).await
    }

    /// Create a new memory MCP service with specific config path and agent
    pub async fn with_config_path_and_agent<P: AsRef<Path> + Clone + std::fmt::Debug>(
        path: P,
        agent_id: Option<String>,
    ) -> Result<Self> {
        // Load configuration from specified path
        let config = Config::load(path.clone())?;
        info!("Loaded configuration from: {:?}", path);

        // Initialize vector store and LLM client
        let (vector_store, llm_client) = initialize_memory_system(&config).await?;
        info!("Initialized vector store and LLM client");

        // Create memory manager
        let memory_manager = Arc::new(MemoryManager::new(
            vector_store,
            llm_client,
            config.memory.clone(),
        ));
        info!("Created memory manager");

        // Create operations handler
        let operations = MemoryOperations::new(
            memory_manager.clone(),
            None, // Default user ID will be derived from agent ID
            agent_id.clone(),
            100,  // Default limit
        );

        Ok(Self {
            memory_manager,
            operations,
            agent_id,
        })
    }

    /// Tool implementation for storing a memory
    async fn store_memory(
        &self,
        arguments: &Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let payload = self.map_to_payload(arguments, &self.agent_id);

        match self.operations.store_memory(payload).await {
            Ok(response) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to store memory: {}", e);
                Err(self.tools_error_to_mcp_error(e))
            }
        }
    }

    /// Tool implementation for querying memories
    async fn query_memory(
        &self,
        arguments: &Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let payload = self.map_to_payload(arguments, &self.agent_id);

        match self.operations.query_memory(payload).await {
            Ok(response) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to query memories: {}", e);
                Err(self.tools_error_to_mcp_error(e))
            }
        }
    }

    /// Tool implementation for listing memories
    async fn list_memories(
        &self,
        arguments: &Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let payload = self.map_to_payload(arguments, &self.agent_id);

        match self.operations.list_memories(payload).await {
            Ok(response) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to list memories: {}", e);
                Err(self.tools_error_to_mcp_error(e))
            }
        }
    }

    /// Tool implementation for getting a specific memory by ID
    async fn get_memory(
        &self,
        arguments: &Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let payload = self.map_to_payload(arguments, &self.agent_id);

        match self.operations.get_memory(payload).await {
            Ok(response) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to get memory: {}", e);
                Err(self.tools_error_to_mcp_error(e))
            }
        }
    }

    /// Find default configuration file path
    fn find_default_config_path() -> Option<PathBuf> {
        // Try current directory first
        if let Ok(current_dir) = std::env::current_dir() {
            let current_config = current_dir.join("config.toml");
            if current_config.exists() {
                return Some(current_config);
            }
        }

        // Try user home directory
        if let Some(home_dir) = dirs::home_dir() {
            let user_config = home_dir.join(".config").join("memo").join("config.toml");
            if user_config.exists() {
                return Some(user_config);
            }
        }

        // Try system config directory (platform-specific)
        #[cfg(target_os = "macos")]
        let system_config = Path::new("/usr/local/etc/memo/config.toml");

        #[cfg(target_os = "linux")]
        let system_config = Path::new("/etc/memo/config.toml");

        #[cfg(target_os = "windows")]
        let system_config = Path::new("C:\\ProgramData\\memo\\config.toml");

        if system_config.exists() {
            return Some(system_config.to_path_buf());
        }

        None
    }

    /// Helper function to convert MCP arguments to MemoryOperationPayload
    fn map_to_payload(
        &self,
        arguments: &Map<String, serde_json::Value>,
        default_agent_id: &Option<String>,
    ) -> MemoryOperationPayload {
        let mut payload = MemoryOperationPayload::default();

        // Extract common fields
        if let Some(content) = arguments.get("content").and_then(|v| v.as_str()) {
            payload.content = Some(content.to_string());
        }

        if let Some(query) = arguments.get("query").and_then(|v| v.as_str()) {
            payload.query = Some(query.to_string());
        }

        if let Some(memory_id) = arguments.get("memory_id").and_then(|v| v.as_str()) {
            payload.memory_id = Some(memory_id.to_string());
        }

        // User ID can be provided or derived from agent ID
        if let Some(user_id) = arguments.get("user_id").and_then(|v| v.as_str()) {
            payload.user_id = Some(user_id.to_string());
        } else if let Some(agent_id) = default_agent_id {
            // If agent_id is set, derive user_id from it
            payload.user_id = Some(format!("user_of_{}", agent_id));
        }

        // Agent ID can be provided or use default
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

    /// Helper function to convert MemoryToolsError to MCP ErrorData
    fn tools_error_to_mcp_error(&self, error: MemoryToolsError) -> ErrorData {
        use MemoryToolsError::*;

        match error {
            InvalidInput(msg) => ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: msg.into(),
                data: None,
            },
            Runtime(msg) => ErrorData {
                code: rmcp::model::ErrorCode(-32603).into(),
                message: msg.into(),
                data: None,
            },
            MemoryNotFound(msg) => ErrorData {
                code: rmcp::model::ErrorCode(-32601).into(),
                message: msg.into(),
                data: None,
            },
            Serialization(_) => ErrorData {
                code: rmcp::model::ErrorCode(-32603).into(),
                message: "Serialization error".into(),
                data: None,
            },
            Core(_) => ErrorData {
                code: rmcp::model::ErrorCode(-32603).into(),
                message: "Core error".into(),
                data: None,
            },
        }
    }
}

impl ServerHandler for MemoryMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation::from_build_env(),
            instructions: Some(
                "A memory management system for AI agents. Store, search, and retrieve memories using natural language queries. Supports different types of memories including conversational, procedural, and factual memories."
                    .to_string(),
            ),
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        async move {
            Ok(ListToolsResult {
                tools: vec![
                    Tool {
                        name: "store_memory".into(),
                        title: Some("Store Memory".into()),
                        description: Some("Store a new memory in the system with specified content and optional metadata.".into()),
                        input_schema: serde_json::json!({
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
                        }).as_object().unwrap().clone().into(),
                        output_schema: Some(
                            serde_json::json!(
                                {
                                    "type": "object",
                                    "properties": {
                                        "success": {"type": "boolean"},
                                        "memory_id": {"type": "string"},
                                        "message": {"type": "string"}
                                    },
                                    "required": ["success", "memory_id", "message"]
                                }
                            )
                            .as_object()
                            .unwrap()
                            .clone()
                            .into(),
                        ),
                        annotations: None,
                        icons: None,
                        meta: None,
                    },
                    Tool {
                        name: "query_memory".into(),
                        title: Some("Query Memory".into()),
                        description: Some("Search memories using semantic similarity and filters.".into()),
                        input_schema: serde_json::json!({
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
                        }).as_object().unwrap().clone().into(),
                        output_schema: Some(
                            serde_json::json!(
                                {
                                    "type": "object",
                                    "properties": {
                                        "success": {"type": "boolean"},
                                        "count": {"type": "number"},
                                        "memories": {"type": "array", "items": {"type": "object"}}
                                    },
                                    "required": ["success", "count", "memories"]
                                }
                            ).as_object()
                            .unwrap()
                            .clone()
                            .into(),
                        ),
                        annotations: None,
                        icons: None,
                        meta: None,
                    },
                    Tool {
                        name: "list_memories".into(),
                        title: Some("List Memories".into()),
                        description: Some("Retrieve memories with optional filtering. Adjust the limit parameter to control the number of results returned (default: 100, max: 1000).".into()),
                        input_schema: serde_json::json!({
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
                        }).as_object().unwrap().clone().into(),
                        output_schema: Some(
                            serde_json::json!(
                                {
                                    "type": "object",
                                    "properties": {
                                        "success": {"type": "boolean"},
                                        "count": {"type": "number"},
                                        "memories": {"type": "array", "items": {"type": "object"}}
                                    },
                                    "required": ["success", "count", "memories"]
                                }
                            )
                            .as_object()
                            .unwrap()
                            .clone()
                            .into(),
                        ),
                        annotations: None,
                        icons: None,
                        meta: None,
                    },
                    Tool {
                        name: "get_memory".into(),
                        title: Some("Get Memory by ID".into()),
                        description: Some("Retrieve a specific memory by its exact ID.".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "memory_id": {
                                    "type": "string",
                                    "description": "Exact ID of the memory to retrieve (required)"
                                }
                            },
                            "required": ["memory_id"]
                        }).as_object().unwrap().clone().into(),
                        output_schema: Some(
                            serde_json::json!(
                                {
                                    "type": "object",
                                    "properties": {
                                        "success": {"type": "boolean"},
                                        "memory": {"type": "object"}
                                    },
                                    "required": ["success", "memory"]
                                }
                            )
                            .as_object()
                            .unwrap()
                            .clone()
                            .into(),
                        ),
                        annotations: None,
                        icons: None,
                        meta: None,
                    },
                ],
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
            let tool_name = &request.name;

            match tool_name.as_ref() {
                "store_memory" => {
                    if let Some(arguments) = &request.arguments {
                        self.store_memory(arguments).await
                    } else {
                        Err(ErrorData {
                            code: rmcp::model::ErrorCode(-32602).into(),
                            message: "Missing arguments".into(),
                            data: None,
                        })
                    }
                }
                "query_memory" => {
                    if let Some(arguments) = &request.arguments {
                        self.query_memory(arguments).await
                    } else {
                        Err(ErrorData {
                            code: rmcp::model::ErrorCode(-32602).into(),
                            message: "Missing arguments".into(),
                            data: None,
                        })
                    }
                }
                "list_memories" => {
                    if let Some(arguments) = &request.arguments {
                        self.list_memories(arguments).await
                    } else {
                        Err(ErrorData {
                            code: rmcp::model::ErrorCode(-32602).into(),
                            message: "Missing arguments".into(),
                            data: None,
                        })
                    }
                }
                "get_memory" => {
                    if let Some(arguments) = &request.arguments {
                        self.get_memory(arguments).await
                    } else {
                        Err(ErrorData {
                            code: rmcp::model::ErrorCode(-32602).into(),
                            message:
                                "Missing arguments. You must provide 'memory_id' for this tool."
                                    .into(),
                            data: None,
                        })
                    }
                }
                _ => Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32601).into(),
                    message: format!("Unknown tool: {}", tool_name).into(),
                    data: None,
                }),
            }
        }
    }
}
