pub mod tools;

pub use cortex_mem_tools::MemoryOperations;
pub use cortex_mem_core::llm::LLMClient;
pub use tools::*;

use std::sync::Arc;

/// Memory tools collection for Rig agents
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
}

impl MemoryTools {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
    
    /// Get the underlying MemoryOperations
    pub fn operations(&self) -> &Arc<MemoryOperations> {
        &self.operations
    }
    
    // ==================== Tiered Access Tools ====================
    
    pub fn abstract_tool(&self) -> AbstractTool {
        AbstractTool::new(self.operations.clone())
    }
    
    pub fn overview_tool(&self) -> OverviewTool {
        OverviewTool::new(self.operations.clone())
    }
    
    pub fn read_tool(&self) -> ReadTool {
        ReadTool::new(self.operations.clone())
    }
    
    // ==================== Search Tools ====================
    
    pub fn search_tool(&self) -> SearchTool {
        SearchTool::new(self.operations.clone())
    }
    
    pub fn find_tool(&self) -> FindTool {
        FindTool::new(self.operations.clone())
    }
    
    // ==================== Filesystem Tools ====================
    
    pub fn ls_tool(&self) -> LsTool {
        LsTool::new(self.operations.clone())
    }
    
    pub fn explore_tool(&self) -> ExploreTool {
        ExploreTool::new(self.operations.clone())
    }
    
    // ==================== Storage Tools ====================
    
    pub fn store_tool(&self) -> StoreTool {
        StoreTool::new(self.operations.clone())
    }
}

/// Create memory tools for Rig agents
pub fn create_memory_tools(operations: Arc<MemoryOperations>) -> MemoryTools {
    MemoryTools::new(operations)
}

/// Create memory tools with full features (LLM + Vector Search)
/// 
/// This is the primary constructor that requires all dependencies.
pub async fn create_memory_tools_with_tenant_and_vector(
    data_dir: impl AsRef<std::path::Path>,
    tenant_id: impl Into<String>,
    llm_client: Arc<dyn LLMClient>,
    qdrant_url: &str,
    qdrant_collection: &str,
    qdrant_api_key: Option<&str>,
    embedding_api_base_url: &str,
    embedding_api_key: &str,
    embedding_model_name: &str,
    embedding_dim: Option<usize>,
    user_id: Option<String>,  // 🆕 添加user_id参数
) -> Result<MemoryTools, Box<dyn std::error::Error>> {
    let operations = MemoryOperations::new(
        data_dir.as_ref().to_str().unwrap(),
        tenant_id,
        llm_client,
        qdrant_url,
        qdrant_collection,
        qdrant_api_key,
        embedding_api_base_url,
        embedding_api_key,
        embedding_model_name,
        embedding_dim,
        user_id,  // 🆕 传递user_id
    ).await?;
    Ok(MemoryTools::new(Arc::new(operations)))
}
