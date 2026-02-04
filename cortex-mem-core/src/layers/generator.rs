use crate::{Result, llm::LLMClient};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Abstract (L0) generator
/// 
/// Generates a 1-2 sentence summary (~100 tokens) from content
pub struct AbstractGenerator;

impl AbstractGenerator {
    pub fn new() -> Self {
        Self
    }
    
    /// Generate abstract from content using LLM
    pub async fn generate_with_llm(&self, content: &str, llm: &Arc<LLMClient>) -> Result<String> {
        let system = "You are a summarization expert. Generate a concise 1-2 sentence abstract (max 100 tokens) that captures the core essence of the content.";
        let prompt = format!("Summarize this content in 1-2 sentences:\n\n{}", content);
        
        llm.complete_with_system(system, &prompt).await
    }
    
    /// Generate abstract from content (fallback without LLM)
    pub async fn generate(&self, content: &str) -> Result<String> {
        // Simple implementation: take first paragraph or first 200 chars
        let abstract_text = if content.len() <= 200 {
            content.to_string()
        } else {
            // Find first paragraph or take first 200 chars
            let first_para = content
                .lines()
                .take_while(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            
            if first_para.len() <= 200 {
                first_para
            } else {
                format!("{}...", &first_para[..197])
            }
        };
        
        Ok(abstract_text)
    }
    
    /// Estimate token count (rough approximation)
    pub fn estimate_tokens(text: &str) -> usize {
        // Simple estimation: ~4 chars per token for English, ~1.5 for Chinese
        text.len() / 3
    }
}

/// Overview (L1) generator
/// 
/// Generates structured overview (~500-2000 tokens) from content
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
    pub async fn generate_with_llm(&self, content: &str, llm: &Arc<LLMClient>) -> Result<String> {
        let system = r#"You are a content analysis expert. Generate a structured overview (500-2000 tokens) with:
1. Core Topics (3-5 main themes)
2. Key Points (5-10 important takeaways)  
3. Entities (people, places, technologies mentioned)
4. Summary (2-3 paragraph overview)

Format as markdown."#;
        
        let prompt = format!("Analyze and create a structured overview of this content:\n\n{}", content);
        
        llm.complete_with_system(system, &prompt).await
    }
    
    /// Generate overview from content (fallback without LLM)
    pub async fn generate(&self, content: &str) -> Result<String> {
        // Simple implementation: extract basic structure
        let overview = Overview {
            core_topics: Self::extract_topics(content),
            key_points: Self::extract_key_points(content),
            entities: Self::extract_entities(content),
            summary: Self::create_summary(content),
        };
        
        Ok(Self::format_overview(&overview))
    }
    
    fn extract_topics(content: &str) -> Vec<String> {
        // Simple: extract markdown headers
        content
            .lines()
            .filter(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .collect()
    }
    
    fn extract_key_points(content: &str) -> Vec<String> {
        // Simple: extract bullet points
        content
            .lines()
            .filter(|line| line.trim().starts_with('-') || line.trim().starts_with('*'))
            .map(|line| {
                line.trim()
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim()
                    .to_string()
            })
            .take(5)
            .collect()
    }
    
    fn extract_entities(_content: &str) -> Vec<String> {
        // TODO: Implement entity extraction with LLM
        Vec::new()
    }
    
    fn create_summary(content: &str) -> String {
        // Simple: take first few lines
        content
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    fn format_overview(overview: &Overview) -> String {
        let mut md = String::from("# Overview\n\n");
        
        if !overview.summary.is_empty() {
            md.push_str("## Summary\n\n");
            md.push_str(&overview.summary);
            md.push_str("\n\n");
        }
        
        if !overview.core_topics.is_empty() {
            md.push_str("## Core Topics\n\n");
            for topic in &overview.core_topics {
                md.push_str(&format!("- {}\n", topic));
            }
            md.push('\n');
        }
        
        if !overview.key_points.is_empty() {
            md.push_str("## Key Points\n\n");
            for (i, point) in overview.key_points.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", i + 1, point));
            }
            md.push('\n');
        }
        
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_abstract_generator() {
        let gen = AbstractGenerator::new();
        let content = "This is a test message about OAuth 2.0 implementation.\n\nOAuth provides secure authorization.";
        
        let abstract_text = gen.generate(content).await.unwrap();
        assert!(!abstract_text.is_empty());
        assert!(abstract_text.len() <= 200);
    }
    
    #[tokio::test]
    async fn test_overview_generator() {
        let gen = OverviewGenerator::new();
        let content = "# OAuth 2.0\n\n## Introduction\n\n- Secure authorization\n- Industry standard\n\nOAuth provides delegated access.";
        
        let overview = gen.generate(content).await.unwrap();
        assert!(overview.contains("# Overview"));
        assert!(overview.contains("OAuth 2.0"));
    }
}
