use anyhow::Result;
use cortex_mem_config::Config;
use cortex_mem_core::{
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

        Ok(Self {
            memory_manager,
            agent_id,
        })
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

        // Use provided user_id or default based on agent_id
        let user_id: String = arguments
            .get("user_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // If no user_id provided but we have an agent_id, use default user_id
                if let Some(agent) = &self.agent_id {
                    Some(format!("user_of_{}", agent))
                } else {
                    None
                }
            })
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'user_id' or --agent parameter not specified"
                    .into(),
                data: None,
            })?;

        // Use provided agent_id or default from service
        let agent_id = arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| self.agent_id.clone());

        let memory_type = arguments
            .get("memory_type")
            .and_then(|v| v.as_str())
            .map(|s| MemoryType::parse_with_result(s))
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

    /// Tool implementation for querying memories (replaces search_memory and recall_context)
    async fn query_memory(
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
            .map(|s| s.to_string())
            .or_else(|| {
                // If no user_id provided but we have an agent_id, use default user_id
                if let Some(agent) = &self.agent_id {
                    Some(format!("user_of_{}", agent))
                } else {
                    None
                }
            });

        // Use provided agent_id or default from service
        let agent_id = arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| self.agent_id.clone());

        let memory_type = arguments
            .get("memory_type")
            .and_then(|v| v.as_str())
            .map(|s| MemoryType::parse_with_result(s))
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

        let k = arguments.get("k").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let min_salience = arguments
            .get("min_salience")
            .and_then(|v| v.as_f64())
            .map(|s| s as f32);

        debug!("Querying memories with query: {}", query);

        // Build filters
        let mut filters = Filters::default();
        filters.user_id = user_id;
        filters.agent_id = agent_id;
        filters.memory_type = memory_type;
        filters.topics = topics;

        // Apply min_salience filter if provided
        if let Some(salience) = min_salience {
            filters.min_importance = Some(salience);
        }

        // Search memories
        match self.memory_manager.search(query, &filters, k).await {
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
                            "salience": m.memory.metadata.importance_score,
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
                error!("Failed to query memories: {}", e);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32603).into(),
                    message: format!("Failed to query memories: {}", e).into(),
                    data: None,
                })
            }
        }
    }

    /// Tool implementation for listing memories
    async fn list_memories(
        &self,
        arguments: &serde_json::Map<std::string::String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        use serde_json::json;
        use tracing::{debug, error, info};

        // Extract arguments
        let user_id = arguments
            .get("user_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // If no user_id provided but we have an agent_id, use default user_id
                if let Some(agent) = &self.agent_id {
                    Some(format!("user_of_{}", agent))
                } else {
                    None
                }
            });

        // Use provided agent_id or default from service
        let agent_id = arguments
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| self.agent_id.clone());

        let memory_type = arguments
            .get("memory_type")
            .and_then(|v| v.as_str())
            .map(|s| MemoryType::parse_with_result(s))
            .transpose()
            .map_err(|e| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: format!("Invalid memory_type: {}", e).into(),
                data: None,
            })?;

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        debug!("Listing memories with limit: {}", limit);

        // Build filters
        let mut filters = Filters::default();
        filters.user_id = user_id;
        filters.agent_id = agent_id;
        filters.memory_type = memory_type;

        // List memories
        match self.memory_manager.list(&filters, Some(limit)).await {
            Ok(memories) => {
                info!("Found {} memories", memories.len());

                let results: Vec<_> = memories
                    .into_iter()
                    .map(|m| {
                        // Create a preview of the content (first 100 characters)
                        let preview = if m.content.len() > 100 {
                            format!("{}...", &m.content[..100])
                        } else {
                            m.content.clone()
                        };

                        json!({
                            "id": m.id,
                            "type": format!("{:?}", m.metadata.memory_type),
                            "salience": m.metadata.importance_score,
                            "preview": preview,
                            "user_id": m.metadata.user_id,
                            "agent_id": m.metadata.agent_id,
                            "topics": m.metadata.topics,
                            "created_at": m.created_at
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
                error!("Failed to list memories: {}", e);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32603).into(),
                    message: format!("Failed to list memories: {}", e).into(),
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
        use tracing::{debug, error, info};

        // Extract arguments
        let memory_id = arguments
            .get("memory_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData {
                code: rmcp::model::ErrorCode(-32602).into(),
                message: "Missing required argument 'memory_id'. To get a specific memory, you must provide its ID. If you want to list all memories, use 'list_memories'. If you want to search for memories, use 'query_memory'.".into(),
                data: None,
            })?;

        debug!("Getting memory with ID: {}", memory_id);

        // Get memory
        match self.memory_manager.get(memory_id).await {
            Ok(Some(memory)) => {
                info!("Retrieved memory with ID: {}", memory_id);

                let result = json!({
                    "success": true,
                    "memory": {
                        "id": memory.id,
                        "content": memory.content,
                        "type": format!("{:?}", memory.metadata.memory_type),
                        "user_id": memory.metadata.user_id,
                        "agent_id": memory.metadata.agent_id,
                        "topics": memory.metadata.topics,
                        "salience": memory.metadata.importance_score,
                        "created_at": memory.created_at,
                        "updated_at": memory.updated_at,
                        "metadata": memory.metadata
                    }
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Ok(None) => {
                info!("No memory found with ID: {}", memory_id);
                Err(ErrorData {
                    code: rmcp::model::ErrorCode(-32602).into(),
                    message: format!("Memory with ID '{}' not found", memory_id).into(),
                    data: None,
                })
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
                        description: Some("Store a new memory in the system. If agent is configured via command line, agent_id and user_id will default to the configured values unless overridden.".into()),
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
                        description: Some("Query memories with semantic search, filters and salience threshold. Replaces search_memory and recall_context.".into()),
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
                        description: Some("List recent memories with summary information".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of memories to return",
                                    "default": 20
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
                        title: Some("Get Memory by ID".into()),
                        description: Some("Retrieve a specific memory by its exact ID. NOTE: This tool requires a specific memory_id parameter. If you don't know the memory ID but want to explore memories, use 'get_all_memories' to see all memories or 'query_memory' to search by content. This tool is best when you already have a memory ID from a previous search.".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "memory_id": {
                                    "type": "string",
                                    "description": "Exact ID of the memory to retrieve (required). You must obtain this ID from previous calls to get_all_memories, list_memories, or query_memory."
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
                    Tool {
                        name: "get_all_memories".into(),
                        title: Some("Get All Memories".into()),
                        description: Some("Retrieve all memories for the current agent/user. This is a convenience tool that internally calls list_memories with a higher limit.".into()),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "memory_type": {
                                    "type": "string",
                                    "enum": ["conversational", "procedural", "factual", "semantic", "episodic", "personal"],
                                    "description": "Type of memory to filter by (optional)"
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
                            message: "Missing arguments. You must provide 'memory_id' for this tool. Consider using 'list_memories' to find memory IDs or 'get_all_memories' to retrieve all memories.".into(),
                            data: None,
                        })
                    }
                }
                "get_all_memories" => {
                    if let Some(arguments) = &request.arguments {
                        // Convert get_all_memories request to list_memories with high limit
                        let mut list_args = arguments.clone();
                        list_args.insert("limit".into(), serde_json::json!(100)); // Set a high limit
                        self.list_memories(&list_args).await
                    } else {
                        // If no arguments, call list_memories with empty args and high limit
                        let mut empty_args = serde_json::Map::new();
                        empty_args.insert("limit".into(), serde_json::json!(100));
                        self.list_memories(&empty_args).await
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
