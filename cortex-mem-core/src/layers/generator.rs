use crate::{Result, llm::LLMClient};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

/// Abstract (L0) generator
///
/// Generates a concise summary (~100 tokens) from content using LLM
/// for quick relevance checking and filtering.
/// Supports entity preservation to prevent named entities from being
/// compressed away during summarization.
pub struct AbstractGenerator;

impl AbstractGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate abstract from content using LLM.
    ///
    /// `known_entities`: optional list of named entities that MUST be preserved
    /// verbatim in the generated abstract if present in the content.
    /// Pass an empty slice when no entity preservation is required.
    pub async fn generate_with_llm(
        &self,
        content: &str,
        llm: &Arc<dyn LLMClient>,
        known_entities: &[String],
    ) -> Result<String> {
        // 截断保护：content 最多取前 8000 个字符（abstract_generation_with_entities 内部也会截断，双重保护）
        let char_count = content.chars().count();
        info!(
            "Generating L0 Abstract (content: {} chars, entities: {:?})",
            char_count, known_entities
        );

        let system = r#"You are an expert at creating concise abstracts.
Your goal is to generate summaries that capture multiple key aspects of content for quick relevance checking.
Keep abstracts under 100 tokens. Prioritize breadth over depth - cover more topics briefly rather than elaborating on one.
Be direct and informative. Use compact phrasing to maximize information density.
When asked to preserve specific named entities, include them verbatim in the abstract."#;

        let prompt = crate::llm::prompts::Prompts::abstract_generation_with_entities(
            content,
            known_entities,
        );
        debug!("L0 Abstract prompt length: {} chars", prompt.chars().count());

        let result = llm.complete_with_system(system, &prompt).await?;

        info!("L0 Abstract generated ({} chars)", result.chars().count());
        Ok(result)
    }

    /// Estimate token count (rough approximation)
    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / 3
    }
}

/// Overview (L1) generator
///
/// Generates structured overview (~500-2000 tokens) from content using LLM
pub struct OverviewGenerator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub core_topics: Vec<String>,
    pub key_points: Vec<String>,
    pub entities: Vec<String>,
    pub summary: String,
}

impl OverviewGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate overview from content using LLM
    pub async fn generate_with_llm(
        &self,
        content: &str,
        llm: &Arc<dyn LLMClient>,
    ) -> Result<String> {
        // 截断保护：overview 内容最多取前 16000 个字符
        let safe_content: String = content.chars().take(16000).collect();

        info!(
            "Generating L1 Overview (content: {} chars, truncated from {})",
            safe_content.chars().count(),
            content.chars().count()
        );

        let system = r#"You are an expert at creating structured overviews.
Your goal is to provide comprehensive yet concise summaries (500-2000 tokens) that help users understand and make decisions about content.
Use clear markdown structure with sections for Summary, Core Topics, Key Points, Entities, and Context."#;

        let prompt = crate::llm::prompts::Prompts::overview_generation(&safe_content);
        debug!("L1 Overview prompt length: {} chars", prompt.chars().count());

        let result = llm.complete_with_system(system, &prompt).await?;

        info!("L1 Overview generated ({} chars)", result.chars().count());
        Ok(result)
    }
}
