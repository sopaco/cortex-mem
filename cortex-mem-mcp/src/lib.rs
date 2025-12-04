use anyhow::Result;
use memo_config::Config;
use memo_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
    types::{Filters, MemoryMetadata, MemoryType},
};
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorData, ListToolsResult,
        PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    RoleServer, ServerHandler,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Service for handling MCP tool calls related to memory management
pub struct MemoryMcpService {
    memory_manager: Arc<MemoryManager>,
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

        Ok(Self { memory_manager })
    }

    /// Tool implementation for storing a memory
    async fn store_memory(
        &self,
        arguments: &serde_json::Map<std::string::String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        use serde_json::json;
        use tracing::{error, info};

        // Extract arguments
        let content = arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'content'".into(),
                data: None,
            })?;

        let user_id = arguments
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'user_id'".into(),
                data: None,
            })?;

        let agent_id = arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let memory_type = arguments
            .get("memory_type")
            .and_then(|v| v.as_str())
            .map(|s| self.parse_memory_type(s))
            .transpose()
            .map_err(|e| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: format!("Invalid memory_type: {}", e).into(),
                data: None,
            })?
            .unwrap_or(MemoryType::Conversational);

        let topics = arguments
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        info!("Storing memory for user: {}", user_id);

        // Create metadata
        let mut metadata = MemoryMetadata::new(memory_type);
        metadata.user_id = Some(user_id.to_string());
        metadata.agent_id = agent_id;
        metadata.topics = topics;

        // Store the memory
        match self
            .memory_manager
            .store(content.to_string(), metadata)
            .await
        {
            Ok(memory_id) => {
                info!("Memory stored successfully with ID: {}", memory_id);
                let result = json!({
                    "success": true,
                    "message": "Memory stored successfully",
                    "memory_id": memory_id
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to store memory: {}", e);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32603).into(),
                    message: format!("Failed to store memory: {}", e).into(),
                    data: None,
                })
            }
        }
    }

    /// Tool implementation for searching memories
    async fn search_memory(
        &self,
        arguments: &serde_json::Map<std::string::String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        use serde_json::json;
        use tracing::{debug, error, info};

        // Extract arguments
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'query'".into(),
                data: None,
            })?;

        let user_id = arguments
            .get("user_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let agent_id = arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let memory_type = arguments
            .get("memory_type")
            .and_then(|v| v.as_str())
            .map(|s| self.parse_memory_type(s))
            .transpose()
            .map_err(|e| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: format!("Invalid memory_type: {}", e).into(),
                data: None,
            })?;

        let topics = arguments
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            });

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        debug!("Searching memories with query: {}", query);

        // Build filters
        let mut filters = Filters::default();
        filters.user_id = user_id;
        filters.agent_id = agent_id;
        filters.memory_type = memory_type;
        filters.topics = topics;

        // Search memories
        match self.memory_manager.search(query, &filters, limit).await {
            Ok(memories) => {
                info!("Found {} matching memories", memories.len());

                let results: Vec<_> = memories
                    .into_iter()
                    .map(|m| {
                        json!({
                            "id": m.memory.id,
                            "content": m.memory.content,
                            "type": format!("{:?}", m.memory.metadata.memory_type),
                            "user_id": m.memory.metadata.user_id,
                            "agent_id": m.memory.metadata.agent_id,
                            "topics": m.memory.metadata.topics,
                            "score": m.score,
                            "created_at": m.memory.created_at
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "count": results.len(),
                    "memories": results
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to search memories: {}", e);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32603).into(),
                    message: format!("Failed to search memories: {}", e).into(),
                    data: None,
                })
            }
        }
    }

    /// Tool implementation for recalling context
    async fn recall_context(
        &self,
        arguments: &serde_json::Map<std::string::String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        use serde_json::json;
        use tracing::{debug, error, info};

        // Extract arguments
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'query'".into(),
                data: None,
            })?;

        let user_id = arguments
            .get("user_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let agent_id = arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let limit = arguments.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        debug!("Recalling context with query: {}", query);

        // Build filters
        let mut filters = Filters::default();
        filters.user_id = user_id;
        filters.agent_id = agent_id;

        // Search for context
        match self.memory_manager.search(query, &filters, limit).await {
            Ok(memories) => {
                info!("Recalled {} context memories", memories.len());

                let contexts: Vec<_> = memories
                    .into_iter()
                    .map(|m| {
                        json!({
                            "id": m.memory.id,
                            "content": m.memory.content,
                            "type": format!("{:?}", m.memory.metadata.memory_type),
                            "score": m.score,
                            "created_at": m.memory.created_at
                        })
                    })
                    .collect();

                let result = json!({
                    "success": true,
                    "count": contexts.len(),
                    "contexts": contexts
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => {
                error!("Failed to recall context: {}", e);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32603).into(),
                    message: format!("Failed to recall context: {}", e).into(),
                    data: None,
                })
            }
        }
    }

    /// Find default configuration file path
    /// Tries multiple locations in order:
    /// 1. Current directory
    /// 2. User home directory/.config/memo/
    /// 3. System config directory
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

    /// Tool implementation for getting a specific memory
    async fn get_memory(
        &self,
        arguments: &serde_json::Map<std::string::String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        use serde_json::json;
        use tracing::{debug, error};

        // Extract arguments
        let memory_id = arguments
            .get("memory_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'memory_id'".into(),
                data: None,
            })?;

        debug!("Getting memory with ID: {}", memory_id);

        // Since memo-core doesn't have a direct get by ID method, we'll search with an empty query
        // and filter by ID in the metadata custom fields
        let mut filters = Filters::default();
        filters
            .custom
            .insert("memory_id".to_string(), json!(memory_id));

        match self.memory_manager.search("", &filters, 1).await {
            Ok(memories) => {
                if let Some(scored_memory) = memories.into_iter().next() {
                    let memory = scored_memory.memory;

                    let result = json!({
                        "success": true,
                        "memory": {
                            "id": memory.id,
                            "content": memory.content,
                            "type": format!("{:?}", memory.metadata.memory_type),
                            "user_id": memory.metadata.user_id,
                            "agent_id": memory.metadata.agent_id,
                            "topics": memory.metadata.topics,
                            "importance_score": memory.metadata.importance_score,
                            "created_at": memory.created_at,
                            "updated_at": memory.updated_at
                        }
                    });

                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&result).unwrap(),
                    )]))
                } else {
                    Err(ErrorData {
                        code: rmcp::model::ErrorCode(-32602).into(),
                        message: format!("Memory with ID '{}' not found", memory_id).into(),
                        data: None,
                    })
                }
            }
            Err(e) => {
                error!("Failed to get memory: {}", e);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32603).into(),
                    message: format!("Failed to get memory: {}", e).into(),
                    data: None,
                })
            }
        }
    }

    /// Helper function to parse memory type from string
    fn parse_memory_type(&self, s: &str) -> Result<MemoryType, String> {
        match s.to_lowercase().as_str() {
            "conversational" => Ok(MemoryType::Conversational),
            "procedural" => Ok(MemoryType::Procedural),
            "factual" => Ok(MemoryType::Factual),
            "semantic" => Ok(MemoryType::Semantic),
            "episodic" => Ok(MemoryType::Episodic),
            "personal" => Ok(MemoryType::Personal),
            _ => Err(format!("Invalid memory type: {}", s)),
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
                        description: Some("Store a new memory in the system".into()),
                        input_schema: serde_json::json!({
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
                            "required": ["content", "user_id"]
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
                        name: "search_memory".into(),
                        title: Some("Search Memory".into()),
                        description: Some("Search for memories using natural language query".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query to find relevant memories"
                                },
                                "user_id": {
                                    "type": "string",
                                    "description": "User ID to filter memories"
                                },
                                "agent_id": {
                                    "type": "string",
                                    "description": "Agent ID to filter memories"
                                },
                                "memory_type": {
                                    "type": "string",
                                    "enum": ["conversational", "procedural", "factual", "semantic", "episodic", "personal"],
                                    "description": "Memory type to filter by"
                                },
                                "topics": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "Topics to filter memories by"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of results to return",
                                    "default": 10
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
                        name: "recall_context".into(),
                        title: Some("Recall Context".into()),
                        description: Some("Recall relevant context based on a query".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Query for context retrieval"
                                },
                                "user_id": {
                                    "type": "string",
                                    "description": "User ID to filter memories"
                                },
                                "agent_id": {
                                    "type": "string",
                                    "description": "Agent ID to filter memories"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of context memories to return",
                                    "default": 5
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
                                        "contexts": {"type": "array", "items": {"type": "object"}}
                                    },
                                    "required": ["success", "count", "contexts"]
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
                        name: "get_memory".into(),
                        title: Some("Get Memory".into()),
                        description: Some("Retrieve a specific memory by its ID".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "memory_id": {
                                    "type": "string",
                                    "description": "ID of the memory to retrieve"
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
                            ).as_object()
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
                "search_memory" => {
                    if let Some(arguments) = &request.arguments {
                        self.search_memory(arguments).await
                    } else {
                        Err(ErrorData {
                            code: rmcp::model::ErrorCode(-32602).into(),
                            message: "Missing arguments".into(),
                            data: None,
                        })
                    }
                }
                "recall_context" => {
                    if let Some(arguments) = &request.arguments {
                        self.recall_context(arguments).await
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
                            message: "Missing arguments".into(),
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
