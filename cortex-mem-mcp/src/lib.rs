use anyhow::Result;
use cortex_mem_config::Config;
use cortex_mem_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
};
use cortex_mem_tools::{MemoryOperations, MemoryToolsError, map_mcp_arguments_to_payload, tools_error_to_mcp_error_code, get_tool_error_message, get_mcp_tool_definitions};
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
        let payload = map_mcp_arguments_to_payload(arguments, &self.agent_id);

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
        let payload = map_mcp_arguments_to_payload(arguments, &self.agent_id);

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
        let payload = map_mcp_arguments_to_payload(arguments, &self.agent_id);

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
        let payload = map_mcp_arguments_to_payload(arguments, &self.agent_id);

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

    /// Helper function to convert MemoryToolsError to MCP ErrorData
    fn tools_error_to_mcp_error(&self, error: MemoryToolsError) -> ErrorData {
        ErrorData {
            code: rmcp::model::ErrorCode(tools_error_to_mcp_error_code(&error)).into(),
            message: get_tool_error_message(&error).into(),
            data: None,
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
            let tool_definitions = get_mcp_tool_definitions();
            let tools: Vec<Tool> = tool_definitions.into_iter().map(|def| {
                Tool {
                    name: def.name.into(),
                    title: def.title.map(|t| t.into()),
                    description: def.description.map(|d| d.into()),
                    input_schema: def.input_schema.as_object().unwrap().clone().into(),
                    output_schema: def.output_schema.map(|schema| schema.as_object().unwrap().clone().into()),
                    annotations: None,
                    icons: None,
                    meta: None,
                }
            }).collect();

            Ok(ListToolsResult {
                tools,
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
