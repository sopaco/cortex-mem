use cortex_mem_config::Config;
use cortex_mem_core::{Filters, MemoryManager, MemoryMetadata, MemoryType};
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum MemoryToolError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Runtime error: {0}")]
    Runtime(String),
}

pub struct MemoryTool {
    memory_manager: Arc<MemoryManager>,
    config: MemoryToolConfig,
}

/// Memory Tool Configuration that uses values from the global config as defaults but allows overrides
pub struct MemoryToolConfig {
    pub default_user_id: Option<String>,
    pub default_agent_id: Option<String>,
    pub max_search_results: Option<usize>, // Can override global config value
    pub auto_enhance: Option<bool>,        // Can override global config value
    pub search_similarity_threshold: Option<f32>, // Can override global config value
}

/// Arguments for memory tool operations
#[derive(Debug, Deserialize)]
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

/// Output from memory tool operations
#[derive(Debug, Serialize)]
pub struct MemoryOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

impl MemoryTool {
    /// Create a new memory tool with configuration from global config with possible overrides
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

        // For memory-related config values, fallback to values from global config if not set in custom
        if config.max_search_results.is_none() {
            config.max_search_results = Some(global_config.memory.max_search_results);
        }
        if config.auto_enhance.is_none() {
            config.auto_enhance = Some(global_config.memory.auto_enhance);
        }
        if config.search_similarity_threshold.is_none() {
            config.search_similarity_threshold = global_config.memory.search_similarity_threshold;
        }

        Self {
            memory_manager,
            config,
        }
    }

    /// Get actual config values with defaults from global config applied
    fn get_effective_max_search_results(&self) -> usize {
        self.config.max_search_results.unwrap_or(10)
    }

    fn get_effective_search_similarity_threshold(&self) -> Option<f32> {
        self.config.search_similarity_threshold
    }

    /// Store a new memory
    async fn store_memory(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        let content = args.content.as_ref().ok_or_else(|| {
            MemoryToolError::InvalidInput("Content is required for store action".to_string())
        })?;

        let memory_type = args
            .memory_type
            .as_ref()
            .map(|t| parse_memory_type(t))
            .unwrap_or(MemoryType::Conversational);

        let mut metadata = MemoryMetadata::new(memory_type);

        // Use provided user_id or default
        if let Some(user_id) = &args.user_id {
            metadata = metadata.with_user_id(user_id.clone());
        } else if let Some(default_user_id) = &self.config.default_user_id {
            metadata = metadata.with_user_id(default_user_id.clone());
        }

        // Use provided agent_id or default
        if let Some(agent_id) = &args.agent_id {
            metadata = metadata.with_agent_id(agent_id.clone());
        } else if let Some(default_agent_id) = &self.config.default_agent_id {
            metadata = metadata.with_agent_id(default_agent_id.clone());
        }

        match self.memory_manager.store(content.clone(), metadata).await {
            Ok(memory_id) => {
                info!("Memory stored via rig tool: {}", memory_id);
                Ok(MemoryOutput {
                    success: true,
                    message: "Memory stored successfully".to_string(),
                    data: Some(json!({
                        "memory_id": memory_id,
                        "content": content
                    })),
                })
            }
            Err(e) => {
                error!("Failed to store memory via rig tool: {}", e);
                Err(MemoryToolError::Runtime(format!(
                    "Failed to store memory: {}",
                    e
                )))
            }
        }
    }

    /// Search for memories
    async fn search_memory(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        let query = args.query.as_ref();

        // 如果为空查询，转换为使用过滤器的列表查询
        if query.is_none() {
            return self.list_memory_by_filters(args).await;
        }
        let query = query.unwrap();

        let mut filters = Filters::new();

        // Apply filters
        if let Some(user_id) = &args.user_id {
            filters.user_id = Some(user_id.clone());
        } else if let Some(default_user_id) = &self.config.default_user_id {
            filters.user_id = Some(default_user_id.clone());
        }

        if let Some(agent_id) = &args.agent_id {
            filters.agent_id = Some(agent_id.clone());
        } else if let Some(default_agent_id) = &self.config.default_agent_id {
            filters.agent_id = Some(default_agent_id.clone());
        }

        if let Some(memory_type_str) = &args.memory_type {
            filters.memory_type = Some(parse_memory_type(memory_type_str));
        }

        if let Some(topics) = &args.topics {
            filters.topics = Some(topics.clone());
        }

        if let Some(keywords) = &args.keywords {
            filters.custom.insert("keywords".to_string(), json!(keywords));
        }

        let limit = args
            .limit
            .unwrap_or(self.get_effective_max_search_results());

        // 使用明确带阈值的搜索方法，确保结果的相关性
        // 优先使用工具配置中的自定义阈值，否则使用记忆管理器配置的默认阈值
        let search_results =
            if let Some(custom_threshold) = self.get_effective_search_similarity_threshold() {
                self.memory_manager
                    .search_with_threshold(query, &filters, limit, Some(custom_threshold))
                    .await
            } else {
                self.memory_manager
                    .search_with_config_threshold(query, &filters, limit)
                    .await
            };

        match search_results {
            Ok(results) => {
                let search_results: Vec<Value> = results
                    .into_iter()
                    .map(|scored_memory| {
                        let memory_type_str =
                            format!("{:?}", scored_memory.memory.metadata.memory_type);
                        let processed_content = self.process_memory_content(
                            &scored_memory.memory.content,
                            &memory_type_str,
                        );

                        json!({
                            "id": scored_memory.memory.id,
                            "content": processed_content,
                            "original_content": scored_memory.memory.content,
                            "score": scored_memory.score,
                            "memory_type": memory_type_str,
                            "created_at": scored_memory.memory.created_at.to_rfc3339(),
                        })
                    })
                    .collect();

                debug!(
                    "Memory search via rig tool: {} results found",
                    search_results.len()
                );

                Ok(MemoryOutput {
                    success: true,
                    message: format!("Found {} memories", search_results.len()),
                    data: Some(json!({
                        "results": search_results,
                        "total": search_results.len()
                    })),
                })
            }
            Err(e) => {
                error!("Failed to search memories via rig tool: {}", e);
                Err(MemoryToolError::Runtime(format!(
                    "Failed to search memories: {}",
                    e
                )))
            }
        }
    }

    /// List memories by filters without vector search (when query is None)
    async fn list_memory_by_filters(
        &self,
        args: &MemoryArgs,
    ) -> Result<MemoryOutput, MemoryToolError> {
        let mut filters = Filters::new();

        // Apply filters
        if let Some(user_id) = &args.user_id {
            filters.user_id = Some(user_id.clone());
        } else if let Some(default_user_id) = &self.config.default_user_id {
            filters.user_id = Some(default_user_id.clone());
        }

        if let Some(agent_id) = &args.agent_id {
            filters.agent_id = Some(agent_id.clone());
        } else if let Some(default_agent_id) = &self.config.default_agent_id {
            filters.agent_id = Some(default_agent_id.clone());
        }

        if let Some(memory_type_str) = &args.memory_type {
            filters.memory_type = Some(parse_memory_type(memory_type_str));
        }

        if let Some(topics) = &args.topics {
            filters.topics = Some(topics.clone());
        }

        if let Some(keywords) = &args.keywords {
            filters.custom.insert("keywords".to_string(), json!(keywords));
        }

        let limit = args
            .limit
            .unwrap_or(self.get_effective_max_search_results());

        let list_results = self.memory_manager.list(&filters, Some(limit)).await;

        match list_results {
            Ok(memories) => {
                let list_results: Vec<Value> = memories
                    .into_iter()
                    .map(|memory| {
                        let memory_type_str = format!("{:?}", memory.metadata.memory_type);
                        let processed_content =
                            self.process_memory_content(&memory.content, &memory_type_str);

                        json!({
                            "id": memory.id,
                            "content": processed_content,
                            "original_content": memory.content,
                            "score": 0.0_f32, // No similarity score for list results
                            "memory_type": memory_type_str,
                            "created_at": memory.created_at.to_rfc3339(),
                        })
                    })
                    .collect();

                debug!(
                    "Memory list via rig tool: {} results found",
                    list_results.len()
                );

                Ok(MemoryOutput {
                    success: true,
                    message: format!("Found {} memories", list_results.len()),
                    data: Some(json!({
                        "results": list_results,
                        "total": list_results.len()
                    })),
                })
            }
            Err(e) => {
                error!("Failed to list memories via rig tool: {}", e);
                Err(MemoryToolError::Runtime(format!(
                    "Failed to list memories: {}",
                    e
                )))
            }
        }
    }

    /// Recall context from memories
    async fn recall_context(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        let query = args.query.as_ref().ok_or_else(|| {
            MemoryToolError::InvalidInput("Query is required for recall action".to_string())
        })?;

        // Search for relevant memories
        let search_result = self.search_memory(args).await?;

        if let Some(data) = search_result.data {
            if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
                // Extract content from top results for context
                let context: Vec<String> = results
                    .iter()
                    .take(5) // Limit to top 5 results for context
                    .filter_map(|result| {
                        result
                            .get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();

                let context_text = context.join("\n\n");

                debug!(
                    "Memory context recalled via rig tool: {} memories",
                    context.len()
                );

                Ok(MemoryOutput {
                    success: true,
                    message: format!("Recalled context from {} memories", context.len()),
                    data: Some(json!({
                        "context": context_text,
                        "memories_count": context.len(),
                        "query": query
                    })),
                })
            } else {
                Ok(MemoryOutput {
                    success: true,
                    message: "No relevant memories found for context".to_string(),
                    data: Some(json!({
                        "context": "",
                        "memories_count": 0,
                        "query": query
                    })),
                })
            }
        } else {
            Err(MemoryToolError::Runtime(
                "Failed to process search results".to_string(),
            ))
        }
    }

    /// Semantic processing of memory content for natural language responses
    fn process_memory_content(&self, content: &str, memory_type: &str) -> String {
        let content = content.trim();

        // Handle common patterns that need semantic processing
        match memory_type {
            "Personal" => {
                // Process personal information for more natural responses
                if content.contains("user's name is") || content.contains("name is") {
                    // Extract name from patterns like "The user's name is Alex" or "User's name is John"
                    if let Some(name_start) = content
                        .find("is ")
                        .and_then(|i| content[i + 3..].find(' ').map(|j| i + 3 + j + 1))
                    {
                        if let Some(name_end) = content[name_start..]
                            .find(|c: char| !c.is_alphanumeric() && c != '\'')
                            .map(|i| name_start + i)
                        {
                            let name = &content[name_start..name_end];
                            return format!("Your name is {}", name);
                        }
                    }
                    // Fallback: remove "The user's" prefix
                    return content
                        .replace("The user's", "Your")
                        .replace("user's", "your");
                }
                content.to_string()
            }
            "Preference" => {
                // Process preferences for natural responses
                if content.contains("likes") {
                    return content.replace("likes", "you like");
                }
                if content.contains("prefers") {
                    return content.replace("prefers", "you prefer");
                }
                content.to_string()
            }
            _ => content.to_string(),
        }
    }

    /// Get a specific memory by ID
    async fn get_memory(&self, args: &MemoryArgs) -> Result<MemoryOutput, MemoryToolError> {
        let memory_id = args.memory_id.as_ref().ok_or_else(|| {
            MemoryToolError::InvalidInput("Memory ID is required for get action".to_string())
        })?;

        match self.memory_manager.get(memory_id).await {
            Ok(Some(memory)) => {
                debug!("Memory retrieved via rig tool: {}", memory_id);

                Ok(MemoryOutput {
                    success: true,
                    message: "Memory retrieved successfully".to_string(),
                    data: Some(json!({
                        "id": memory.id,
                        "content": memory.content,
                        "memory_type": format!("{:?}", memory.metadata.memory_type),
                        "created_at": memory.created_at.to_rfc3339(),
                        "updated_at": memory.updated_at.to_rfc3339(),
                        "metadata": {
                            "user_id": memory.metadata.user_id,
                            "agent_id": memory.metadata.agent_id,
                            "run_id": memory.metadata.run_id,
                            "actor_id": memory.metadata.actor_id,
                            "role": memory.metadata.role,
                        }
                    })),
                })
            }
            Ok(None) => Ok(MemoryOutput {
                success: false,
                message: "Memory not found".to_string(),
                data: None,
            }),
            Err(e) => {
                error!("Failed to get memory via rig tool: {}", e);
                Err(MemoryToolError::Runtime(format!(
                    "Failed to get memory: {}",
                    e
                )))
            }
        }
    }
}

#[async_trait::async_trait]
impl Tool for MemoryTool {
    const NAME: &'static str = "memory";

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

fn parse_memory_type(memory_type_str: &str) -> MemoryType {
    match memory_type_str.to_lowercase().as_str() {
        "conversational" => MemoryType::Conversational,
        "procedural" => MemoryType::Procedural,
        "factual" => MemoryType::Factual,
        "semantic" => MemoryType::Semantic,
        "episodic" => MemoryType::Episodic,
        "personal" => MemoryType::Personal,
        _ => MemoryType::Conversational,
    }
}

pub fn create_memory_tool(
    memory_manager: Arc<MemoryManager>,
    global_config: &Config,
    custom_config: Option<MemoryToolConfig>,
) -> MemoryTool {
    MemoryTool::new(memory_manager, global_config, custom_config)
}
