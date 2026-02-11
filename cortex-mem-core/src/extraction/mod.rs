pub mod extractor;
pub mod types;
pub mod user_profile;  // ✅ 新增

pub use extractor::{MemoryExtractor, ExtractionConfig};
pub use types::{
    ExtractedMemories, ExtractedFact, ExtractedDecision,
    ExtractedEntity, MemoryType, MemoryImportance,
};
pub use user_profile::*;  // ✅ 导出用户信息类型
