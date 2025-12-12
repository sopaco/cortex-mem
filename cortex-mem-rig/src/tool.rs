use cortex_mem_config::Config;
use cortex_mem_core::{MemoryManager};
use cortex_mem_tools::{MemoryOperations, MemoryOperationPayload, MemoryToolsError};
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{debug, error, info};

// Re-export the error type from cortex-mem-tools for backward compatibility
pub use cortex_mem_tools::MemoryToolsError as MemoryToolError;

/// Memory tool configuration
pub struct MemoryToolConfig {
    pub default_user_id: Option<String>,
    pub default_agent_id: Option<String>,
    pub max_search_results: Option<usize>,
    pub auto_enhance: Option<bool>,
    pub search_similarity_threshold: Option<f32>,
}

/// Memory tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryArgs {
    pub action: String,
    pub content: Option<String>,
    pub query: Option<String>,
    pub memory_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub memory_type: Option<String>,
    pub topics: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub limit: Option<usize>,
}

/// Memory tool output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

/// Memory Tool implementation using shared operations
pub struct MemoryTool {
    operations: MemoryOperations,
    config: MemoryToolConfig,
}

impl MemoryTool {
    /// Create a new memory tool with the provided memory manager and configuration
    pub fn new(
        memory_manager: Arc<MemoryManager>,
        global_config: &Config,
        custom_config: Option<MemoryToolConfig>,
    ) -> Self {
        let mut config = MemoryToolConfig::default();

        // Apply custom config overrides if provided
        if let Some(custom) = custom_config {
            config.default_user_id = custom.default_user_id.or(config.default_user_id);
            config.default_agent_id = custom.default_agent_id.or(config.default_agent_id);
            config.max_search_results = custom.max_search_results.or(config.max_search_results);
            config.auto_enhance = custom.auto_enhance.or(config.auto_enhance);
            config.search_similarity_threshold = custom
                .search_similarity_threshold
                .or(config.search_similarity_threshold);
        }

        // Fallback to values from global config if not set in custom
        if config.max_search_results.is_none() {
            config.max_search_results = Some(global_config.memory.max_search_results);
        }
        if config.auto_enhance.is_none() {
            config.auto_enhance = Some(global_config.memory.auto_enhance);
        }
        if config.search_similarity_threshold.is_none() {
            config.search_similarity_threshold = global_config.memory.search_similarity_threshold;
        }

        // Create operations handler
        let operations = MemoryOperations::new(
            memory_manager.clone(),
            config.default_user_id.clone(),
            config.default_agent_id.clone(),
            config.max_search_results.unwrap_or(10),
        );

        Self {
            operations,
            config,
        }
    }

    /// Get the effective max search results
    fn get_effective_max_search_results(&self) -> usize {
        self.config.max_search_results.unwrap_or(10)
    }

    /// Get the effective search similarity threshold
    fn get_effective_search_similarity_threshold(&self) -> Option<f32> {
        self.config.search_similarity_threshold
    }

    /// Store a new memory
    async fn store_memory(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        let payload = self.map_args_to_payload(args, "store");

        match self.operations.store_memory(payload).await {
            Ok(response) => {
                info!("Memory stored via rig tool");
                Ok(MemoryOutput {
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

    /// Search memories by semantic similarity
    async fn search_memory(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        // If query is None, fall back to listing by filters
        if args.query.is_none() {
            return self.list_memory_by_filters(args).await;
        }

        let payload = self.map_args_to_payload(args, "search");

        match self.operations.query_memory(payload).await {
            Ok(response) => {
                Ok(MemoryOutput {
                    success: response.success,
                    message: response.message,
                    data: response.data,
                })
            }
            Err(e) => {
                error!("Failed to search memories via rig tool: {}", e);
                Err(e)
            }
        }
    }

    /// List memories by filters without vector search
    async fn list_memory_by_filters(
        &self,
        args: &MemoryArgs,
    ) -> Result<MemoryOutput, MemoryToolError> {
        let payload = self.map_args_to_payload(args, "list");

        match self.operations.list_memories(payload).await {
            Ok(response) => {
                Ok(MemoryOutput {
                    success: response.success,
                    message: response.message,
                    data: response.data,
                })
            }
            Err(e) => {
                error!("Failed to list memories via rig tool: {}", e);
                Err(e)
            }
        }
    }

    /// Recall context for a query
    async fn recall_context(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        if args.query.is_none() {
            return Err(MemoryToolError::InvalidInput(
                "Query is required for recall context".to_string(),
            ));
        }

        // Recall context is essentially a query operation
        self.search_memory(args).await
    }

    /// Get a specific memory by ID
    async fn get_memory(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        if args.memory_id.is_none() {
            return Err(MemoryToolError::InvalidInput(
                "Memory ID is required for get action".to_string(),
            ));
        }

        let payload = self.map_args_to_payload(args, "get");

        match self.operations.get_memory(payload).await {
            Ok(response) => {
                Ok(MemoryOutput {
                    success: response.success,
                    message: response.message,
                    data: response.data,
                })
            }
            Err(e) => {
                error!("Failed to get memory via rig tool: {}", e);
                Err(e)
            }
        }
    }

    /// Process memory content for display
    fn process_memory_content(&self, content: &str, memory_type: &str) -> String {
        // Truncate content for preview if it's too long
        if content.len() > 500 {
            let safe_end = content
                .char_indices()
                .nth(500)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            format!("{}...", &content[..safe_end])
        } else {
            content.to_string()
        }
    }

    /// Helper function to convert Rig MemoryArgs to MemoryOperationPayload
    fn map_args_to_payload(&self, args: &MemoryArgs, operation: &str) -> MemoryOperationPayload {
        let mut payload = MemoryOperationPayload::default();

        // Store operation
        if operation == "store" && args.content.is_some() {
            payload.content = args.content.clone();
        }

        // Query/Search/Recall operations
        if (operation == "query" || operation == "recall" || operation == "search") && args.query.is_some() {
            payload.query = args.query.clone();
        }

        // Get operation
        if operation == "get" && args.memory_id.is_some() {
            payload.memory_id = args.memory_id.clone();
        }

        // Common fields
        if args.user_id.is_some() {
            payload.user_id = args.user_id.clone();
        } else {
            payload.user_id = self.config.default_user_id.clone();
        }

        if args.agent_id.is_some() {
            payload.agent_id = args.agent_id.clone();
        } else {
            payload.agent_id = self.config.default_agent_id.clone();
        }

        payload.memory_type = args.memory_type.clone();
        payload.topics = args.topics.clone();
        payload.keywords = args.keywords.clone();
        payload.limit = args.limit;

        payload
    }
}

impl Tool for MemoryTool {
    const NAME: &'static str = "CortexMemoryTool";

    type Error = MemoryToolError;
    type Args = MemoryArgs;
    type Output = MemoryOutput;

    fn definition(
        &self,
        _prompt: String,
    ) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
        async move {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "Store, search, and retrieve agent memories. Supports storing new memories, searching existing ones, and recalling context.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["store", "search", "recall", "get"],
                            "description": "Action to perform: store (save new memory), search (find memories), recall (get context), get (retrieve specific memory)"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to store (required for store action)"
                        },
                        "query": {
                            "type": "string",
                            "description": "Search query (required for search and recall actions)"
                        },
                        "memory_id": {
                            "type": "string",
                            "description": "Memory ID (required for get action)"
                        },
                        "user_id": {
                            "type": "string",
                            "description": "User ID for filtering (optional)"
                        },
                        "agent_id": {
                            "type": "string",
                            "description": "Agent ID for filtering (optional)"
                        },
                        "memory_type": {
                            "type": "string",
                            "enum": ["conversational", "procedural", "factual"],
                            "description": "Type of memory (optional, defaults to conversational)"
                        },
                        "topics": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Topics to filter memories by (optional)"
                        },
                        "keywords": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Keywords to filter memories by (optional)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results (optional, defaults to configured max)"
                        }
                    },
                    "required": ["action"]
                }),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            match args.action.as_str() {
                "store" => self.store_memory(&args).await,
                "search" => self.search_memory(&args).await,
                "recall" => self.recall_context(&args).await,
                "get" => self.get_memory(&args).await,
                _ => Err(MemoryToolError::InvalidInput(format!(
                    "Unknown action: {}. Supported actions: store, search, recall, get",
                    args.action
                ))),
            }
        }
    }
}

impl Default for MemoryToolConfig {
    fn default() -> Self {
        Self {
            default_user_id: None,
            default_agent_id: None,
            max_search_results: None, // Will be taken from global config
            auto_enhance: None,       // Will be taken from global config
            search_similarity_threshold: None, // Will be taken from global config
        }
    }
}

/// Create a memory tool with default configuration
pub fn create_memory_tool(
    memory_manager: Arc<MemoryManager>,
    global_config: &Config,
    custom_config: Option<MemoryToolConfig>,
) -> MemoryTool {
    MemoryTool::new(memory_manager, global_config, custom_config)
}
