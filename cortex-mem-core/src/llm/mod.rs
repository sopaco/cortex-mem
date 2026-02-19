pub mod client;
pub mod extractor_types;
pub mod prompts;

pub use client::{LLMClient, LLMClientImpl, LLMConfig, MemoryExtractionResponse, ExtractedFactRaw, ExtractedDecisionRaw, ExtractedEntityRaw};
pub use extractor_types::{StructuredFactExtraction, DetailedFactExtraction, StructuredFact};
pub use prompts::Prompts;

/// Type alias for boxed LLMClient trait object
pub type BoxedLLMClient = Box<dyn LLMClient>;
