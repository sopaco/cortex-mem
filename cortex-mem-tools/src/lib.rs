pub mod errors;
pub mod operations;
pub mod types;
pub mod mcp;
pub mod tools;

pub use errors::{ToolsError, Result};
pub use operations::MemoryOperations;
pub use types::*;
pub use mcp::{ToolDefinition, get_mcp_tool_definitions, get_mcp_tool_definition};

// 🆕 重新导出 GenerationStats 以便外部使用
pub use cortex_mem_core::automation::GenerationStats;
