use cortex_mem_tools::MemoryOperations;
use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

/// Store Memory tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMemoryArgs {
    pub thread_id: String,
    pub role: String,
    pub content: String,
}

/// Query Memory tool arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMemoryArgs {
    pub query: String,
    pub thread_id: Option<String>,
    pub limit: Option<usize>,
}

/// Memory tools for Rig framework
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
}

impl MemoryTools {
    /// Create new memory tools
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
    
    /// Create from data directory
    pub async fn from_data_dir(data_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let operations = MemoryOperations::from_data_dir(data_dir).await?;
        Ok(Self {
            operations: Arc::new(operations),
        })
    }
}

/// Store memory tool
pub struct StoreMemoryTool {
    operations: Arc<MemoryOperations>,
}

impl Tool for StoreMemoryTool {
    const NAME: &'static str = "store_memory";
    
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Args = StoreMemoryArgs;
    type Output = String;
    
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Store a message in memory for a given thread/session".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "string",
                        "description": "The thread/session ID"
                    },
                    "role": {
                        "type": "string",
                        "description": "Message role (user/assistant/system)",
                        "enum": ["user", "assistant", "system"]
                    },
                    "content": {
                        "type": "string",
                        "description": "The message content"
                    }
                },
                "required": ["thread_id", "role", "content"]
            }),
        }
    }
    
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("Storing memory: thread={}, role={}", args.thread_id, args.role);
        
        let message_id = self.operations
            .add_message(&args.thread_id, &args.role, &args.content)
            .await?;
        
        Ok(format!("Message stored successfully with ID: {}", message_id))
    }
}

/// Query memory tool
pub struct QueryMemoryTool {
    operations: Arc<MemoryOperations>,
}

impl Tool for QueryMemoryTool {
    const NAME: &'static str = "query_memory";
    
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Args = QueryMemoryArgs;
    type Output = String;
    
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search through memories using semantic search".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "thread_id": {
                        "type": "string",
                        "description": "Optional: limit search to a specific thread"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results (default: 10)"
                    }
                },
                "required": ["query"]
            }),
        }
    }
    
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("Querying memory: query={}", args.query);
        
        let limit = args.limit.unwrap_or(10);
        let results = self.operations
            .search(&args.query, args.thread_id.as_deref(), limit)
            .await?;
        
        if results.is_empty() {
            return Ok("No memories found matching the query.".to_string());
        }
        
        let mut output = format!("Found {} memories:\n\n", results.len());
        for (i, memory) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. {} (score: {:.2})\n   {}\n\n",
                i + 1,
                memory.uri,
                memory.score.unwrap_or(0.0),
                memory.content
            ));
        }
        
        Ok(output)
    }
}

/// Create memory tools for Rig agent
pub fn create_memory_tools(
    operations: Arc<MemoryOperations>,
) -> Vec<Box<dyn Tool<Error = Box<dyn std::error::Error + Send + Sync>, Output = String>>> {
    vec![
        Box::new(StoreMemoryTool {
            operations: operations.clone(),
        }),
        Box::new(QueryMemoryTool {
            operations: operations.clone(),
        }),
    ]
}
