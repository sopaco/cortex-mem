//! Session memory extraction module
//!
//! Implements OpenViking-style memory extraction from sessions:
//! - Extract user preferences
//! - Extract entities (people, projects)
//! - Extract events/decisions
//! - Extract agent cases (problem + solution)

use crate::{
    llm::LLMClient,
    filesystem::FilesystemOperations,
    CortexFilesystem,
    Result, Error,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Extracted memory from session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemories {
    /// User preferences extracted
    pub preferences: Vec<PreferenceMemory>,
    /// Entities mentioned (people, projects)
    pub entities: Vec<EntityMemory>,
    /// Events/decisions
    pub events: Vec<EventMemory>,
    /// Agent cases (problem + solution)
    pub cases: Vec<CaseMemory>,
}

impl Default for ExtractedMemories {
    fn default() -> Self {
        Self {
            preferences: Vec::new(),
            entities: Vec::new(),
            events: Vec::new(),
            cases: Vec::new(),
        }
    }
}

/// User preference memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceMemory {
    pub topic: String,
    pub preference: String,
    pub confidence: f32,
}

/// Entity memory (person, project, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMemory {
    pub name: String,
    pub entity_type: String,
    pub description: String,
    pub context: String,
}

/// Event memory (decision, milestone)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMemory {
    pub title: String,
    pub event_type: String,
    pub summary: String,
    pub timestamp: Option<String>,
}

/// Case memory (problem + solution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseMemory {
    pub title: String,
    pub problem: String,
    pub solution: String,
    pub lessons_learned: Vec<String>,
}

/// Memory extractor for session commit
pub struct MemoryExtractor {
    llm_client: Arc<dyn LLMClient>,
    filesystem: Arc<CortexFilesystem>,
}

impl MemoryExtractor {
    /// Create a new memory extractor
    pub fn new(llm_client: Arc<dyn LLMClient>, filesystem: Arc<CortexFilesystem>) -> Self {
        Self { llm_client, filesystem }
    }

    /// Extract memories from session messages using LLM
    pub async fn extract(&self, messages: &[String]) -> Result<ExtractedMemories> {
        if messages.is_empty() {
            return Ok(ExtractedMemories::default());
        }

        let prompt = self.build_extraction_prompt(messages);
        let response = self.llm_client.complete(&prompt).await?;
        self.parse_extraction_response(&response)
    }

    /// Build the extraction prompt
    fn build_extraction_prompt(&self, messages: &[String]) -> String {
        let messages_text = messages.join("\n\n---\n\n");
        
        format!(
            r#"Analyze the following conversation and extract memories in JSON format.

## Instructions

Extract the following types of memories:

1. **Preferences** (user preferences by topic):
   - topic: The topic/subject area
   - preference: The user's stated preference
   - confidence: 0.0-1.0 confidence level

2. **Entities** (people, projects, organizations mentioned):
   - name: Entity name
   - entity_type: "person", "project", "organization", "technology", etc.
   - description: Brief description
   - context: How it was mentioned

3. **Events** (decisions, milestones, important occurrences):
   - title: Event title
   - event_type: "decision", "milestone", "occurrence"
   - summary: Brief summary
   - timestamp: If mentioned

4. **Cases** (problems encountered and solutions found):
   - title: Case title
   - problem: The problem description
   - solution: The solution applied
   - lessons_learned: List of lessons learned

## Conversation

{}

## Output Format

Output a JSON object with the following structure:
```json
{{
  "preferences": [{{"topic": "...", "preference": "...", "confidence": 0.8}}],
  "entities": [{{"name": "...", "entity_type": "...", "description": "...", "context": "..."}}],
  "events": [{{"title": "...", "event_type": "...", "summary": "...", "timestamp": "..."}}],
  "cases": [{{"title": "...", "problem": "...", "solution": "...", "lessons_learned": ["..."]}}]
}}
```

Only output the JSON, no additional text."#,
            messages_text
        )
    }

    /// Parse the LLM response into ExtractedMemories
    fn parse_extraction_response(&self, response: &str) -> Result<ExtractedMemories> {
        // Try to extract JSON from the response
        let json_str = if response.starts_with('{') {
            response.to_string()
        } else {
            // Try to find JSON block
            response
                .find('{')
                .and_then(|start| response.rfind('}').map(|end| &response[start..=end]))
                .map(|s| s.to_string())
                .unwrap_or_default()
        };

        if json_str.is_empty() {
            return Ok(ExtractedMemories::default());
        }

        serde_json::from_str(&json_str).map_err(|e| Error::Other(format!("Failed to parse extraction response: {}", e)))
    }

    /// Save extracted memories to user/agent dimensions
    pub async fn save_memories(&self, memories: &ExtractedMemories) -> Result<()> {
        // Ensure directories exist by writing a placeholder
        // Note: FilesystemOperations doesn't have create_dir, so we rely on write to create parent dirs
        
        // Save preferences
        for (idx, pref) in memories.preferences.iter().enumerate() {
            let uri = format!("cortex://user/preferences/pref_{}.md", idx);
            let content = format!("# {}\n\n{}", pref.topic, pref.preference);
            self.filesystem.write(&uri, &content).await?;
        }

        // Save entities
        for (idx, entity) in memories.entities.iter().enumerate() {
            let uri = format!("cortex://user/entities/entity_{}.md", idx);
            let content = format!(
                "# {}\n\n**Type**: {}\n\n**Description**: {}\n\n**Context**: {}",
                entity.name, entity.entity_type, entity.description, entity.context
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save events
        for (idx, event) in memories.events.iter().enumerate() {
            let uri = format!("cortex://user/events/event_{}.md", idx);
            let timestamp = event.timestamp.as_deref().unwrap_or("N/A");
            let content = format!(
                "# {}\n\n**Type**: {}\n\n**Summary**: {}\n\n**Timestamp**: {}",
                event.title, event.event_type, event.summary, timestamp
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save cases
        for (idx, case) in memories.cases.iter().enumerate() {
            let uri = format!("cortex://agent/cases/case_{}.md", idx);
            let lessons = case.lessons_learned.iter()
                .map(|l| format!("- {}", l))
                .collect::<Vec<_>>()
                .join("\n");
            let content = format!(
                "# {}\n\n## Problem\n\n{}\n\n## Solution\n\n{}\n\n## Lessons Learned\n\n{}",
                case.title, case.problem, case.solution, lessons
            );
            self.filesystem.write(&uri, &content).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extraction_response() {
        let json = r#"{
            "preferences": [{"topic": "language", "preference": "Chinese", "confidence": 0.9}],
            "entities": [{"name": "Alice", "entity_type": "person", "description": "Developer", "context": "Colleague"}],
            "events": [],
            "cases": []
        }"#;

        // Note: This test would need a mock filesystem to work properly
        // For now, we just verify the parsing logic
        let parsed: ExtractedMemories = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.preferences.len(), 1);
        assert_eq!(parsed.preferences[0].topic, "language");
        assert_eq!(parsed.entities.len(), 1);
    }
}
