pub mod client;
pub mod extractor_types;

pub use client::{LLMClient, LLMClientImpl, LLMConfig, MemoryExtractionResponse, ExtractedFactRaw, ExtractedDecisionRaw, ExtractedEntityRaw};
pub use extractor_types::{StructuredFactExtraction, DetailedFactExtraction, StructuredFact};

/// Type alias for boxed LLMClient trait object
pub type BoxedLLMClient = Box<dyn LLMClient>;
