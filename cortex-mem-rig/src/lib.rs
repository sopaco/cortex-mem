pub mod tools;

pub use cortex_mem_tools::MemoryOperations;
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
