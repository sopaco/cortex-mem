use cortex_mem_tools::MemoryOperations;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Store Memory arguments (simplified for V2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMemoryArgs {
    pub thread_id: String,
    pub role: String,
    pub content: String,
}

/// Query Memory arguments (simplified for V2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMemoryArgs {
    pub query: String,
    pub thread_id: Option<String>,
    pub limit: Option<usize>,
}

/// Memory tools wrapper for external integrations
/// 
/// This is a simplified version that provides basic memory operations
/// without full Rig framework integration. For full Rig support,
/// use the rig-core compatible version.
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
    
    /// Store a message
    pub async fn store_memory(&self, args: StoreMemoryArgs) -> Result<String, Box<dyn std::error::Error>> {
        info!("Storing memory: thread={}, role={}", args.thread_id, args.role);
        
        let message_id = self.operations
            .add_message(&args.thread_id, &args.role, &args.content)
            .await?;
        
        Ok(format!("Message stored successfully with ID: {}", message_id))
    }
    
    /// Query memories
    pub async fn query_memory(&self, args: QueryMemoryArgs) -> Result<String, Box<dyn std::error::Error>> {
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
    
    /// Get the underlying operations
    pub fn operations(&self) -> &Arc<MemoryOperations> {
        &self.operations
    }
}

/// Create memory tools from operations
pub fn create_memory_tools(operations: Arc<MemoryOperations>) -> MemoryTools {
    MemoryTools::new(operations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_tools() {
        let tmpdir = tempfile::tempdir().unwrap();
        let tools = MemoryTools::from_data_dir(tmpdir.path().to_str().unwrap())
            .await
            .unwrap();

        // Store memory
        let store_args = StoreMemoryArgs {
            thread_id: "test-session".to_string(),
            role: "user".to_string(),
            content: "Hello world".to_string(),
        };
        let result = tools.store_memory(store_args).await.unwrap();
        assert!(result.contains("stored successfully"));

        // Query memory
        let query_args = QueryMemoryArgs {
            query: "Hello".to_string(),
            thread_id: Some("test-session".to_string()),
            limit: Some(10),
        };
        let result = tools.query_memory(query_args).await.unwrap();
        assert!(result.contains("Found"));
    }
}
