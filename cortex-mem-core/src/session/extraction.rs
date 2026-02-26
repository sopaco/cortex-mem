//! Session memory extraction module
//!
//! Implements OpenViking-style memory extraction from sessions:
//! - Extract user preferences
//! - Extract entities (people, projects)
//! - Extract events/decisions
//! - Extract agent cases (problem + solution)

use crate::{CortexFilesystem, Error, Result, llm::LLMClient};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Extracted memory from session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemories {
    /// User preferences extracted
    #[serde(default)]
    pub preferences: Vec<PreferenceMemory>,
    /// Entities mentioned (people, projects)
    #[serde(default)]
    pub entities: Vec<EntityMemory>,
    /// Events/decisions
    #[serde(default)]
    pub events: Vec<EventMemory>,
    /// Agent cases (problem + solution)
    #[serde(default)]
    pub cases: Vec<CaseMemory>,
    /// Personal information (age, occupation, education, etc.)
    #[serde(default)]
    pub personal_info: Vec<PersonalInfoMemory>,
    /// Work history (companies, roles, durations)
    #[serde(default)]
    pub work_history: Vec<WorkHistoryMemory>,
    /// Relationships (family, friends, colleagues)
    #[serde(default)]
    pub relationships: Vec<RelationshipMemory>,
    /// Goals (career goals, personal goals)
    #[serde(default)]
    pub goals: Vec<GoalMemory>,
}

impl Default for ExtractedMemories {
    fn default() -> Self {
        Self {
            preferences: Vec::new(),
            entities: Vec::new(),
            events: Vec::new(),
            cases: Vec::new(),
            personal_info: Vec::new(),
            work_history: Vec::new(),
            relationships: Vec::new(),
            goals: Vec::new(),
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

/// Personal information memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalInfoMemory {
    pub category: String, // e.g., "age", "occupation", "education", "location"
    pub content: String,
    pub confidence: f32,
}

/// Work history memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkHistoryMemory {
    pub company: String,
    pub role: String,
    pub duration: Option<String>,
    pub description: String,
    pub confidence: f32,
}

/// Relationship memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMemory {
    pub person: String,
    pub relation_type: String, // e.g., "family", "colleague", "friend"
    pub context: String,
    pub confidence: f32,
}

/// Goal memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalMemory {
    pub goal: String,
    pub category: String, // e.g., "career", "personal", "health", "learning"
    pub timeline: Option<String>,
    pub confidence: f32,
}

/// Memory extractor for session commit
pub struct MemoryExtractor {
    llm_client: Arc<dyn LLMClient>,
    filesystem: Arc<CortexFilesystem>,
    user_id: String,
    agent_id: String,
}

impl MemoryExtractor {
    /// Create a new memory extractor
    pub fn new(
        llm_client: Arc<dyn LLMClient>,
        filesystem: Arc<CortexFilesystem>,
        user_id: String,
        agent_id: String,
    ) -> Self {
        Self {
            llm_client,
            filesystem,
            user_id,
            agent_id,
        }
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

1. **Personal Info** (user's personal information):
   - category: "age", "occupation", "education", "location", "nationality", etc.
   - content: The specific information
   - confidence: 0.0-1.0 confidence level

2. **Work History** (user's work experience):
   - company: Company name
   - role: Job title/role
   - duration: Time period (optional)
   - description: Brief description of role/responsibilities
   - confidence: 0.0-1.0 confidence level

3. **Preferences** (user preferences by topic):
   - topic: The topic/subject area
   - preference: The user's stated preference
   - confidence: 0.0-1.0 confidence level

4. **Relationships** (people user mentions):
   - person: Person's name
   - relation_type: "family", "colleague", "friend", "mentor", etc.
   - context: How they're related/context
   - confidence: 0.0-1.0 confidence level

5. **Goals** (user's goals and aspirations):
   - goal: The specific goal
   - category: "career", "personal", "health", "learning", "financial", etc.
   - timeline: When they want to achieve it (optional)
   - confidence: 0.0-1.0 confidence level

6. **Entities** (people, projects, organizations mentioned):
   - name: Entity name
   - entity_type: "person", "project", "organization", "technology", etc.
   - description: Brief description
   - context: How it was mentioned

7. **Events** (decisions, milestones, important occurrences):
   - title: Event title
   - event_type: "decision", "milestone", "occurrence"
   - summary: Brief summary
   - timestamp: If mentioned

8. **Cases** (problems encountered and solutions found):
   - title: Case title
   - problem: The problem encountered
   - solution: How it was solved
   - lessons_learned: Array of lessons learned

## Response Format

Return ONLY a JSON object with this structure:

{{
  "personal_info": [{{ "category": "age", "content": "30岁", "confidence": 0.9 }}],
  "work_history": [{{ "company": "...", "role": "...", "duration": "...", "description": "...", "confidence": 0.9 }}],
  "preferences": [{{ "topic": "...", "preference": "...", "confidence": 0.9 }}],
  "relationships": [{{ "person": "...", "relation_type": "...", "context": "...", "confidence": 0.9 }}],
  "goals": [{{ "goal": "...", "category": "...", "timeline": "...", "confidence": 0.9 }}],
  "entities": [{{ "name": "...", "entity_type": "...", "description": "...", "context": "..." }}],
  "events": [{{ "title": "...", "event_type": "...", "summary": "...", "timestamp": "..." }}],
  "cases": [{{ "title": "...", "problem": "...", "solution": "...", "lessons_learned": ["..."] }}]
}}

Only include memories that are clearly stated in the conversation. Set empty arrays for categories with no data.

## Conversation

{}

## Response

Return ONLY the JSON object. No additional text before or after."#,
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

        serde_json::from_str(&json_str)
            .map_err(|e| Error::Other(format!("Failed to parse extraction response: {}", e)))
    }

    /// Save extracted memories to user/agent dimensions
    pub async fn save_memories(&self, memories: &ExtractedMemories) -> Result<()> {
        use crate::FilesystemOperations;

        // 🔧 确保基础维度目录存在（否则工具访问会失败）
        let user_base_dir = format!("cortex://user/{}", self.user_id);
        let agent_base_dir = format!("cortex://agent/{}", self.agent_id);

        // 创建用户和agent基础目录（如果不存在）
        // 通过写入一个临时文件再删除来确保目录被创建
        let user_marker = format!("{}/.marker", user_base_dir);
        let agent_marker = format!("{}/.marker", agent_base_dir);
        let _ = self.filesystem.write(&user_marker, "").await;
        let _ = self.filesystem.write(&agent_marker, "").await;
        let _ = self.filesystem.delete(&user_marker).await;
        let _ = self.filesystem.delete(&agent_marker).await;

        // 🔧 改进：读取已有文件，去重后追加，而不是覆盖

        // Save preferences with deduplication
        let prefs_dir = format!("cortex://user/{}/preferences", self.user_id);
        let existing_prefs = self.load_existing_memories(&prefs_dir).await?;
        let new_prefs = self.deduplicate_preferences(&memories.preferences, &existing_prefs);
        let start_idx = existing_prefs.len();

        for (idx, pref) in new_prefs.iter().enumerate() {
            let uri = format!("{}/pref_{}.md", prefs_dir, start_idx + idx);
            let content = format!(
                "# {}\n\n{}\n\n**Added**: {}\n**Confidence**: {:.2}",
                pref.topic,
                pref.preference,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                pref.confidence
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save entities with deduplication
        let entities_dir = format!("cortex://user/{}/entities", self.user_id);
        let existing_entities = self.load_existing_memories(&entities_dir).await?;
        let new_entities = self.deduplicate_entities(&memories.entities, &existing_entities);
        let start_idx = existing_entities.len();

        for (idx, entity) in new_entities.iter().enumerate() {
            let uri = format!("{}/entity_{}.md", entities_dir, start_idx + idx);
            let content = format!(
                "# {}\n\n**Type**: {}\n\n**Description**: {}\n\n**Context**: {}\n\n**Added**: {}",
                entity.name,
                entity.entity_type,
                entity.description,
                entity.context,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save events with deduplication
        let events_dir = format!("cortex://user/{}/events", self.user_id);
        let existing_events = self.load_existing_memories(&events_dir).await?;
        let new_events = self.deduplicate_events(&memories.events, &existing_events);
        let start_idx = existing_events.len();

        for (idx, event) in new_events.iter().enumerate() {
            let uri = format!("{}/event_{}.md", events_dir, start_idx + idx);
            let timestamp = event.timestamp.as_deref().unwrap_or("N/A");
            let content = format!(
                "# {}\n\n**Type**: {}\n\n**Summary**: {}\n\n**Timestamp**: {}\n\n**Added**: {}",
                event.title,
                event.event_type,
                event.summary,
                timestamp,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save cases
        let cases_dir = format!("cortex://agent/{}/cases", self.agent_id);
        let existing_cases = self.load_existing_memories(&cases_dir).await?;
        let start_idx = existing_cases.len();

        for (idx, case) in memories.cases.iter().enumerate() {
            let uri = format!("{}/case_{}.md", cases_dir, start_idx + idx);
            let lessons = case
                .lessons_learned
                .iter()
                .map(|l| format!("- {}", l))
                .collect::<Vec<_>>()
                .join("\n");
            let content = format!(
                "# {}\n\n## Problem\n\n{}\n\n## Solution\n\n{}\n\n## Lessons Learned\n\n{}\n\n**Added**: {}",
                case.title,
                case.problem,
                case.solution,
                lessons,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save personal info with deduplication
        let personal_info_dir = format!("cortex://user/{}/personal_info", self.user_id);
        let existing_personal_info = self.load_existing_memories(&personal_info_dir).await?;
        let new_personal_info =
            self.deduplicate_personal_info(&memories.personal_info, &existing_personal_info);
        let start_idx = existing_personal_info.len();

        for (idx, info) in new_personal_info.iter().enumerate() {
            let uri = format!("{}/info_{}.md", personal_info_dir, start_idx + idx);
            let content = format!(
                "# {}\n\n{}\n\n**Added**: {}\n**Confidence**: {:.2}",
                info.category,
                info.content,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                info.confidence
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save work history with deduplication
        let work_history_dir = format!("cortex://user/{}/work_history", self.user_id);
        let existing_work_history = self.load_existing_memories(&work_history_dir).await?;
        let new_work_history =
            self.deduplicate_work_history(&memories.work_history, &existing_work_history);
        let start_idx = existing_work_history.len();

        for (idx, work) in new_work_history.iter().enumerate() {
            let uri = format!("{}/work_{}.md", work_history_dir, start_idx + idx);
            let duration = work.duration.as_deref().unwrap_or("N/A");
            let content = format!(
                "# {} - {}\n\n**Duration**: {}\n\n**Description**: {}\n\n**Added**: {}\n**Confidence**: {:.2}",
                work.company,
                work.role,
                duration,
                work.description,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                work.confidence
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save relationships with deduplication
        let relationships_dir = format!("cortex://user/{}/relationships", self.user_id);
        let existing_relationships = self.load_existing_memories(&relationships_dir).await?;
        let new_relationships =
            self.deduplicate_relationships(&memories.relationships, &existing_relationships);
        let start_idx = existing_relationships.len();

        for (idx, rel) in new_relationships.iter().enumerate() {
            let uri = format!("{}/rel_{}.md", relationships_dir, start_idx + idx);
            let content = format!(
                "# {}\n\n**Type**: {}\n\n**Context**: {}\n\n**Added**: {}\n**Confidence**: {:.2}",
                rel.person,
                rel.relation_type,
                rel.context,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                rel.confidence
            );
            self.filesystem.write(&uri, &content).await?;
        }

        // Save goals with deduplication
        let goals_dir = format!("cortex://user/{}/goals", self.user_id);
        let existing_goals = self.load_existing_memories(&goals_dir).await?;
        let new_goals = self.deduplicate_goals(&memories.goals, &existing_goals);
        let start_idx = existing_goals.len();

        for (idx, goal) in new_goals.iter().enumerate() {
            let uri = format!("{}/goal_{}.md", goals_dir, start_idx + idx);
            let timeline = goal.timeline.as_deref().unwrap_or("未指定");
            let content = format!(
                "# {}\n\n**Category**: {}\n\n**Timeline**: {}\n\n**Added**: {}\n**Confidence**: {:.2}",
                goal.goal,
                goal.category,
                timeline,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                goal.confidence
            );
            self.filesystem.write(&uri, &content).await?;
        }

        Ok(())
    }

    /// Load existing memory files from a directory
    async fn load_existing_memories(&self, dir_uri: &str) -> Result<Vec<String>> {
        use crate::FilesystemOperations;

        match self.filesystem.list(dir_uri).await {
            Ok(entries) => {
                let mut contents = Vec::new();
                for entry in entries {
                    if entry.name.ends_with(".md") && !entry.name.starts_with('.') {
                        if let Ok(content) = self.filesystem.read(&entry.uri).await {
                            contents.push(content);
                        }
                    }
                }
                Ok(contents)
            }
            Err(_) => Ok(Vec::new()), // Directory doesn't exist yet
        }
    }

    /// Deduplicate preferences against existing ones
    fn deduplicate_preferences(
        &self,
        new_prefs: &[PreferenceMemory],
        existing_contents: &[String],
    ) -> Vec<PreferenceMemory> {
        new_prefs
            .iter()
            .filter(|pref| {
                // 🔧 改进：检查完整内容的相似度，而不仅仅是topic匹配
                let pref_full_content = format!("{} {}", pref.topic, pref.preference);
                let is_duplicate = existing_contents.iter().any(|existing| {
                    // 检查完整内容的相似度（而非简单的子串匹配）
                    Self::calculate_similarity(&pref_full_content, existing) > 0.8
                });
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Deduplicate entities against existing ones
    fn deduplicate_entities(
        &self,
        new_entities: &[EntityMemory],
        existing_contents: &[String],
    ) -> Vec<EntityMemory> {
        new_entities
            .iter()
            .filter(|entity| {
                // 🔧 改进：检查name+description的组合相似度
                let entity_full_content = format!("{} {}", entity.name, entity.description);
                let is_duplicate = existing_contents.iter().any(|existing| {
                    Self::calculate_similarity(&entity_full_content, existing) > 0.8
                });
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Deduplicate events against existing ones
    fn deduplicate_events(
        &self,
        new_events: &[EventMemory],
        existing_contents: &[String],
    ) -> Vec<EventMemory> {
        new_events
            .iter()
            .filter(|event| {
                // 🔧 改进：检查title+summary的组合相似度
                let event_full_content = format!("{} {}", event.title, event.summary);
                let is_duplicate = existing_contents.iter().any(|existing| {
                    Self::calculate_similarity(&event_full_content, existing) > 0.8
                });
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Deduplicate personal info against existing ones
    fn deduplicate_personal_info(
        &self,
        new_info: &[PersonalInfoMemory],
        existing_contents: &[String],
    ) -> Vec<PersonalInfoMemory> {
        new_info
            .iter()
            .filter(|info| {
                // 🔧 改进：检查category+content的组合相似度
                let info_full_content = format!("{} {}", info.category, info.content);
                let is_duplicate = existing_contents
                    .iter()
                    .any(|existing| Self::calculate_similarity(&info_full_content, existing) > 0.8);
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Deduplicate work history against existing ones
    fn deduplicate_work_history(
        &self,
        new_work: &[WorkHistoryMemory],
        existing_contents: &[String],
    ) -> Vec<WorkHistoryMemory> {
        new_work
            .iter()
            .filter(|work| {
                // 🔧 改进：检查company+role+description的组合相似度
                let work_full_content =
                    format!("{} {} {}", work.company, work.role, work.description);
                let is_duplicate = existing_contents
                    .iter()
                    .any(|existing| Self::calculate_similarity(&work_full_content, existing) > 0.8);
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Deduplicate relationships against existing ones
    fn deduplicate_relationships(
        &self,
        new_rels: &[RelationshipMemory],
        existing_contents: &[String],
    ) -> Vec<RelationshipMemory> {
        new_rels
            .iter()
            .filter(|rel| {
                // 🔧 改进：检查person+relation_type+context的组合相似度
                let rel_full_content =
                    format!("{} {} {}", rel.person, rel.relation_type, rel.context);
                let is_duplicate = existing_contents
                    .iter()
                    .any(|existing| Self::calculate_similarity(&rel_full_content, existing) > 0.8);
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Deduplicate goals against existing ones
    fn deduplicate_goals(
        &self,
        new_goals: &[GoalMemory],
        existing_contents: &[String],
    ) -> Vec<GoalMemory> {
        new_goals
            .iter()
            .filter(|goal| {
                // 🔧 改进：检查goal+category的组合相似度
                let goal_full_content = format!("{} {}", goal.goal, goal.category);
                let is_duplicate = existing_contents
                    .iter()
                    .any(|existing| Self::calculate_similarity(&goal_full_content, existing) > 0.8);
                !is_duplicate
            })
            .cloned()
            .collect()
    }

    /// Calculate similarity between two strings
    fn calculate_similarity(a: &str, b: &str) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let mut max_match = 0;
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 || b_len == 0 {
            return 0.0;
        }

        let min_len = a_len.min(b_len);

        // 滑动窗口检查相似度
        for window_size in (1..=min_len).rev() {
            for i in 0..=a_len.saturating_sub(window_size) {
                let window_a: String = a_chars[i..(i + window_size).min(a_len)].iter().collect();
                // 在 b 中查找这个窗口
                for j in 0..=b_len.saturating_sub(window_size) {
                    let window_b: String =
                        b_chars[j..(j + window_size).min(b_len)].iter().collect();
                    if window_a == window_b {
                        max_match = max_match.max(window_size);
                        break;
                    }
                }
                if max_match == window_size {
                    break;
                }
            }
        }

        max_match as f64 / a_len.max(b_len) as f64
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
