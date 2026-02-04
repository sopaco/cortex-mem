/// Prompt templates for various LLM tasks
pub struct Prompts;

impl Prompts {
    /// Prompt for generating L0 abstract
    pub fn abstract_generation(content: &str) -> String {
        format!(
            r#"Summarize the following content in 1-2 sentences (maximum 100 tokens).
Focus on the core topic and key information.

Content:
{}

Summary:"#,
            content
        )
    }
    
    /// Prompt for generating L1 overview
    pub fn overview_generation(content: &str) -> String {
        format!(
            r#"Extract and structure the following information from the content:

1. Core topics (list 2-5 main topics)
2. Key points (list 3-7 important points)
3. Important entities (people, organizations, technologies mentioned)
4. Brief summary (2-3 sentences)

Format the output as a structured markdown document with clear sections.

Content:
{}

Structured Overview:"#,
            content
        )
    }
    
    /// Prompt for memory extraction from conversation
    pub fn memory_extraction(conversation: &str) -> String {
        format!(
            r#"Analyze the following conversation and extract:

1. **Facts**: Factual information that was shared or discovered
2. **Decisions**: Decisions that were made during the conversation
3. **Action Items**: Tasks or next steps that were identified
4. **User Preferences**: Any preferences, habits, or patterns expressed by the user
5. **Agent Learnings**: Insights or lessons learned that could help in future interactions

Format your response as JSON with the following structure:
{{
  "facts": [{{ "content": "...", "confidence": 0.9 }}],
  "decisions": [{{ "description": "...", "rationale": "..." }}],
  "action_items": [{{ "description": "...", "priority": "high|medium|low" }}],
  "user_preferences": [{{ "category": "...", "content": "..." }}],
  "agent_learnings": [{{ "task_type": "...", "learned_approach": "...", "success_rate": 0.8 }}]
}}

Conversation:
{}

Extracted Memories (JSON):"#,
            conversation
        )
    }
    
    /// Prompt for intent analysis in retrieval
    pub fn intent_analysis(query: &str) -> String {
        format!(
            r#"Analyze the following query and extract:

1. **Keywords**: Important keywords for search (2-5 words)
2. **Entities**: Named entities mentioned (people, places, technologies)
3. **Time Range**: Any time-related constraints (if mentioned)
4. **Query Type**: The type of query (factual, procedural, conceptual, etc.)

Format as JSON:
{{
  "keywords": ["...", "..."],
  "entities": ["...", "..."],
  "time_range": {{ "start": "...", "end": "..." }},
  "query_type": "..."
}}

Query: {}

Intent Analysis (JSON):"#,
            query
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_abstract_prompt() {
        let prompt = Prompts::abstract_generation("This is test content about OAuth 2.0.");
        assert!(prompt.contains("Summarize"));
        assert!(prompt.contains("OAuth 2.0"));
    }
    
    #[test]
    fn test_overview_prompt() {
        let prompt = Prompts::overview_generation("Test content");
        assert!(prompt.contains("Core topics"));
        assert!(prompt.contains("Key points"));
    }
}
