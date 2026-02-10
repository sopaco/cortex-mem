use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Memory importance level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum MemoryImportance {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for MemoryImportance {
    fn default() -> Self {
        Self::Medium
    }
}

/// Memory type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Fact,
    Decision,
    Entity,
    Preference,
    Skill,
    Goal,
}

/// Extracted fact from conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFact {
    pub content: String,
    pub subject: Option<String>,
    pub confidence: f32,
    pub importance: MemoryImportance,
    pub source_uris: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl ExtractedFact {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            subject: None,
            confidence: 0.8,
            importance: MemoryImportance::Medium,
            source_uris: Vec::new(),
            timestamp: Utc::now(),
        }
    }
    
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }
    
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_importance(mut self, importance: MemoryImportance) -> Self {
        self.importance = importance;
        self
    }
    
    pub fn add_source(mut self, uri: impl Into<String>) -> Self {
        self.source_uris.push(uri.into());
        self
    }
}

/// Extracted decision from conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDecision {
    pub decision: String,
    pub context: String,
    pub rationale: Option<String>,
    pub confidence: f32,
    pub importance: MemoryImportance,
    pub source_uris: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl ExtractedDecision {
    pub fn new(decision: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            decision: decision.into(),
            context: context.into(),
            rationale: None,
            confidence: 0.8,
            importance: MemoryImportance::Medium,
            source_uris: Vec::new(),
            timestamp: Utc::now(),
        }
    }
    
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }
    
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_importance(mut self, importance: MemoryImportance) -> Self {
        self.importance = importance;
        self
    }
    
    pub fn add_source(mut self, uri: impl Into<String>) -> Self {
        self.source_uris.push(uri.into());
        self
    }
}

/// Extracted entity (person, organization, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub attributes: Vec<(String, String)>,
    pub source_uris: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl ExtractedEntity {
    pub fn new(name: impl Into<String>, entity_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entity_type: entity_type.into(),
            description: None,
            attributes: Vec::new(),
            source_uris: Vec::new(),
            timestamp: Utc::now(),
        }
    }
    
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    pub fn with_confidence(self, _confidence: f64) -> Self {
        // Just return self - ExtractedEntity doesn't store confidence
        self
    }
    
    pub fn add_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }
    
    pub fn add_source(mut self, uri: impl Into<String>) -> Self {
        self.source_uris.push(uri.into());
        self
    }
}

/// Complete extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemories {
    pub facts: Vec<ExtractedFact>,
    pub decisions: Vec<ExtractedDecision>,
    pub entities: Vec<ExtractedEntity>,
    pub extracted_at: DateTime<Utc>,
    pub thread_id: String,
}

impl ExtractedMemories {
    pub fn new(thread_id: impl Into<String>) -> Self {
        Self {
            facts: Vec::new(),
            decisions: Vec::new(),
            entities: Vec::new(),
            extracted_at: Utc::now(),
            thread_id: thread_id.into(),
        }
    }
    
    pub fn add_fact(&mut self, fact: ExtractedFact) {
        self.facts.push(fact);
    }
    
    pub fn add_decision(&mut self, decision: ExtractedDecision) {
        self.decisions.push(decision);
    }
    
    pub fn add_entity(&mut self, entity: ExtractedEntity) {
        self.entities.push(entity);
    }
    
    pub fn total_count(&self) -> usize {
        self.facts.len() + self.decisions.len() + self.entities.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.total_count() == 0
    }
    
    /// Convert to markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Extracted Memories\n\n"));
        md.push_str(&format!("**Thread**: {}\n", self.thread_id));
        md.push_str(&format!("**Extracted**: {}\n", self.extracted_at.format("%Y-%m-%d %H:%M:%S UTC")));
        md.push_str(&format!("**Total**: {} memories\n\n", self.total_count()));
        
        // Facts section
        if !self.facts.is_empty() {
            md.push_str(&format!("## Facts ({})\n\n", self.facts.len()));
            for (i, fact) in self.facts.iter().enumerate() {
                md.push_str(&format!("### Fact {}\n\n", i + 1));
                md.push_str(&format!("{}\n\n", fact.content));
                if let Some(ref subject) = fact.subject {
                    md.push_str(&format!("**Subject**: {}\n", subject));
                }
                md.push_str(&format!("**Confidence**: {:.2}\n", fact.confidence));
                md.push_str(&format!("**Importance**: {:?}\n", fact.importance));
                md.push_str("\n");
            }
        }
        
        // Decisions section
        if !self.decisions.is_empty() {
            md.push_str(&format!("## Decisions ({})\n\n", self.decisions.len()));
            for (i, decision) in self.decisions.iter().enumerate() {
                md.push_str(&format!("### Decision {}\n\n", i + 1));
                md.push_str(&format!("**Decision**: {}\n\n", decision.decision));
                md.push_str(&format!("**Context**: {}\n\n", decision.context));
                if let Some(ref rationale) = decision.rationale {
                    md.push_str(&format!("**Rationale**: {}\n\n", rationale));
                }
                md.push_str(&format!("**Confidence**: {:.2}\n", decision.confidence));
                md.push_str(&format!("**Importance**: {:?}\n", decision.importance));
                md.push_str("\n");
            }
        }
        
        // Entities section
        if !self.entities.is_empty() {
            md.push_str(&format!("## Entities ({})\n\n", self.entities.len()));
            for (i, entity) in self.entities.iter().enumerate() {
                md.push_str(&format!("### Entity {}: {}\n\n", i + 1, entity.name));
                md.push_str(&format!("**Type**: {}\n", entity.entity_type));
                if let Some(ref description) = entity.description {
                    md.push_str(&format!("**Description**: {}\n", description));
                }
                if !entity.attributes.is_empty() {
                    md.push_str("\n**Attributes**:\n\n");
                    for (key, value) in &entity.attributes {
                        md.push_str(&format!("- **{}**: {}\n", key, value));
                    }
                }
                md.push_str("\n");
            }
        }
        
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extracted_fact() {
        let fact = ExtractedFact::new("User prefers dark mode")
            .with_subject("UI Preferences")
            .with_confidence(0.95)
            .with_importance(MemoryImportance::High)
            .add_source("cortex://session/session1/msg001.md");
        
        assert_eq!(fact.content, "User prefers dark mode");
        assert_eq!(fact.subject, Some("UI Preferences".to_string()));
        assert_eq!(fact.confidence, 0.95);
        assert_eq!(fact.importance, MemoryImportance::High);
        assert_eq!(fact.source_uris.len(), 1);
    }
    
    #[test]
    fn test_extracted_decision() {
        let decision = ExtractedDecision::new(
            "Use PostgreSQL for database",
            "Choosing database for project"
        )
        .with_rationale("Better support for JSON and full-text search")
        .with_importance(MemoryImportance::Critical);
        
        assert_eq!(decision.decision, "Use PostgreSQL for database");
        assert!(decision.rationale.is_some());
        assert_eq!(decision.importance, MemoryImportance::Critical);
    }
    
    #[test]
    fn test_extracted_entity() {
        let entity = ExtractedEntity::new("OpenAI", "Organization")
            .with_description("AI research company")
            .add_attribute("Founded", "2015")
            .add_attribute("Product", "GPT");
        
        assert_eq!(entity.name, "OpenAI");
        assert_eq!(entity.entity_type, "Organization");
        assert_eq!(entity.attributes.len(), 2);
    }
    
    #[test]
    fn test_extracted_memories() {
        let mut memories = ExtractedMemories::new("test-thread");
        
        memories.add_fact(ExtractedFact::new("Test fact"));
        memories.add_decision(ExtractedDecision::new("Test decision", "Test context"));
        memories.add_entity(ExtractedEntity::new("Test Entity", "Person"));
        
        assert_eq!(memories.total_count(), 3);
        assert!(!memories.is_empty());
    }
    
    #[test]
    fn test_memories_to_markdown() {
        let mut memories = ExtractedMemories::new("test-thread");
        memories.add_fact(ExtractedFact::new("User likes coding")
            .with_importance(MemoryImportance::High));
        
        let md = memories.to_markdown();
        assert!(md.contains("# Extracted Memories"));
        assert!(md.contains("test-thread"));
        assert!(md.contains("User likes coding"));
        assert!(md.contains("High"));
    }
}
