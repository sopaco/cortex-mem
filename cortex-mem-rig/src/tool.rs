use cortex_mem_tools::{MemoryOperations, get_mcp_tool_definitions, map_mcp_arguments_to_payload};
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tracing::{error, info};

// Re-export the error type from cortex-mem-tools
pub use cortex_mem_tools::MemoryToolsError as MemoryToolError;

/// Memory tool configuration
#[derive(Debug, Clone)]
pub struct MemoryToolConfig {
    pub default_user_id: Option<String>,
    pub default_agent_id: Option<String>,
}

impl Default for MemoryToolConfig {
    fn default() -> Self {
        Self {
            default_user_id: None,
            default_agent_id: None,
        }
    }
}

/// Store Memory tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMemoryArgs {
    pub content: String,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub memory_type: Option<String>,
    pub topics: Option<Vec<String>>,
}

/// Query Memory tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMemoryArgs {
    pub query: String,
    pub k: Option<usize>,
    pub memory_type: Option<String>,
    pub min_salience: Option<f64>,
    pub topics: Option<Vec<String>>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
}

/// List Memories tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMemoriesArgs {
    pub limit: Option<usize>,
    pub memory_type: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
}

/// Get Memory tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMemoryArgs {
    pub memory_id: String,
}

/// Common tool output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryToolOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

/// Base struct for memory tools that shares common functionality
pub struct MemoryToolsBase {
    operations: Arc<MemoryOperations>,
    config: MemoryToolConfig,
}

impl MemoryToolsBase {
    /// Create a new memory tools base with the provided operations and configuration
    pub fn new(operations: Arc<MemoryOperations>, custom_config: Option<MemoryToolConfig>) -> Self {
        let config = custom_config.unwrap_or_default();
        Self { operations, config }
    }

    /// Convert JSON values to a Map for the map_mcp_arguments_to_payload function
    fn args_to_map(&self, args: &serde_json::Value) -> Map<String, Value> {
        if let Value::Object(map) = args {
            map.clone()
        } else {
            Map::new()
        }
    }
}

/// Store Memory Tool
pub struct StoreMemoryTool {
    base: Arc<MemoryToolsBase>,
}

impl StoreMemoryTool {
    pub fn new(base: Arc<MemoryToolsBase>) -> Self {
        Self { base }
    }
}

impl Tool for StoreMemoryTool {
    const NAME: &'static str = "store_memory";

    type Error = MemoryToolError;
    type Args = StoreMemoryArgs;
    type Output = MemoryToolOutput;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async move {
            // Get tool definition from MCP definitions
            let tool_definitions = get_mcp_tool_definitions();
            let def = tool_definitions
                .iter()
                .find(|d| d.name == "store_memory")
                .expect("store_memory tool definition should exist");

            ToolDefinition {
                name: Self::NAME.to_string(),
                description: def.description.clone().unwrap_or_default(),
                parameters: def.input_schema.clone(),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            // Convert args to JSON Value
            let args_json = json!(args);
            let arguments = self.base.args_to_map(&args_json);

            // Map to payload using shared function
            let payload =
                map_mcp_arguments_to_payload(&arguments, &self.base.config.default_agent_id);

            match self.base.operations.store_memory(payload).await {
                Ok(response) => {
                    info!("Memory stored via rig tool");
                    Ok(MemoryToolOutput {
                        success: response.success,
                        message: response.message,
                        data: response.data,
                    })
                }
                Err(e) => {
                    error!("Failed to store memory via rig tool: {}", e);
                    Err(e)
                }
            }
        }
    }
}

/// Query Memory Tool
pub struct QueryMemoryTool {
    base: Arc<MemoryToolsBase>,
}

impl QueryMemoryTool {
    pub fn new(base: Arc<MemoryToolsBase>) -> Self {
        Self { base }
    }
}

impl Tool for QueryMemoryTool {
    const NAME: &'static str = "query_memory";

    type Error = MemoryToolError;
    type Args = QueryMemoryArgs;
    type Output = MemoryToolOutput;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async move {
            // Get tool definition from MCP definitions
            let tool_definitions = get_mcp_tool_definitions();
            let def = tool_definitions
                .iter()
                .find(|d| d.name == "query_memory")
                .expect("query_memory tool definition should exist");

            ToolDefinition {
                name: Self::NAME.to_string(),
                description: def.description.clone().unwrap_or_default(),
                parameters: def.input_schema.clone(),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            // Convert args to JSON Value
            let args_json = json!(args);
            let arguments = self.base.args_to_map(&args_json);

            // Map to payload using shared function
            let payload =
                map_mcp_arguments_to_payload(&arguments, &self.base.config.default_agent_id);

            match self.base.operations.query_memory(payload).await {
                Ok(response) => Ok(MemoryToolOutput {
                    success: response.success,
                    message: response.message,
                    data: response.data,
                }),
                Err(e) => {
                    error!("Failed to query memories via rig tool: {}", e);
                    Err(e)
                }
            }
        }
    }
}

/// List Memories Tool
pub struct ListMemoriesTool {
    base: Arc<MemoryToolsBase>,
}

impl ListMemoriesTool {
    pub fn new(base: Arc<MemoryToolsBase>) -> Self {
        Self { base }
    }
}

impl Tool for ListMemoriesTool {
    const NAME: &'static str = "list_memories";

    type Error = MemoryToolError;
    type Args = ListMemoriesArgs;
    type Output = MemoryToolOutput;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async move {
            // Get tool definition from MCP definitions
            let tool_definitions = get_mcp_tool_definitions();
            let def = tool_definitions
                .iter()
                .find(|d| d.name == "list_memories")
                .expect("list_memories tool definition should exist");

            ToolDefinition {
                name: Self::NAME.to_string(),
                description: def.description.clone().unwrap_or_default(),
                parameters: def.input_schema.clone(),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            // Convert args to JSON Value
            let args_json = json!(args);
            let arguments = self.base.args_to_map(&args_json);

            // Map to payload using shared function
            let payload =
                map_mcp_arguments_to_payload(&arguments, &self.base.config.default_agent_id);

            match self.base.operations.list_memories(payload).await {
                Ok(response) => Ok(MemoryToolOutput {
                    success: response.success,
                    message: response.message,
                    data: response.data,
                }),
                Err(e) => {
                    error!("Failed to list memories via rig tool: {}", e);
                    Err(e)
                }
            }
        }
    }
}

/// Get Memory Tool
pub struct GetMemoryTool {
    base: Arc<MemoryToolsBase>,
}

impl GetMemoryTool {
    pub fn new(base: Arc<MemoryToolsBase>) -> Self {
        Self { base }
    }
}

impl Tool for GetMemoryTool {
    const NAME: &'static str = "get_memory";

    type Error = MemoryToolError;
    type Args = GetMemoryArgs;
    type Output = MemoryToolOutput;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async move {
            // Get tool definition from MCP definitions
            let tool_definitions = get_mcp_tool_definitions();
            let def = tool_definitions
                .iter()
                .find(|d| d.name == "get_memory")
                .expect("get_memory tool definition should exist");

            ToolDefinition {
                name: Self::NAME.to_string(),
                description: def.description.clone().unwrap_or_default(),
                parameters: def.input_schema.clone(),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            // Convert args to JSON Value
            let args_json = json!(args);
            let arguments = self.base.args_to_map(&args_json);

            // Map to payload using shared function
            let payload =
                map_mcp_arguments_to_payload(&arguments, &self.base.config.default_agent_id);

            match self.base.operations.get_memory(payload).await {
                Ok(response) => Ok(MemoryToolOutput {
                    success: response.success,
                    message: response.message,
                    data: response.data,
                }),
                Err(e) => {
                    error!("Failed to get memory via rig tool: {}", e);
                    Err(e)
                }
            }
        }
    }
}

/// MemoryTools struct that provides all memory tools
pub struct MemoryTools {
    base: Arc<MemoryToolsBase>,
}

impl MemoryTools {
    /// Create new memory tools with the provided operations and configuration
    pub fn new(operations: Arc<MemoryOperations>, custom_config: Option<MemoryToolConfig>) -> Self {
        let base = Arc::new(MemoryToolsBase::new(operations, custom_config));
        Self { base }
    }

    /// Get the store memory tool
    pub fn store_memory(&self) -> StoreMemoryTool {
        StoreMemoryTool::new(self.base.clone())
    }

    /// Get the query memory tool
    pub fn query_memory(&self) -> QueryMemoryTool {
        QueryMemoryTool::new(self.base.clone())
    }

    /// Get the list memories tool
    pub fn list_memories(&self) -> ListMemoriesTool {
        ListMemoriesTool::new(self.base.clone())
    }

    /// Get the get memory tool
    pub fn get_memory(&self) -> GetMemoryTool {
        GetMemoryTool::new(self.base.clone())
    }
}

/// Create memory tools with configuration
pub fn create_memory_tools(
    operations: Arc<MemoryOperations>,
    custom_config: Option<MemoryToolConfig>,
) -> MemoryTools {
    MemoryTools::new(operations, custom_config)
}
