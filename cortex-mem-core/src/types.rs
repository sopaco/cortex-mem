use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core memory structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: MemoryMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Memory metadata for filtering and organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryMetadata {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub role: Option<String>,
    pub memory_type: MemoryType,
    pub hash: String,
    pub importance_score: f32,
    pub entities: Vec<String>,
    pub topics: Vec<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Types of memory supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Conversational memories from user interactions
    Conversational,
    /// Procedural memories about how to do things
    Procedural,
    /// Factual memories about entities and relationships
    Factual,
    /// Semantic memories about concepts and meanings
    Semantic,
    /// Episodic memories about specific events and experiences
    Episodic,
    /// Personal preferences and characteristics
    Personal,
}

impl MemoryType {
    /// Parse a string into a MemoryType enum
    /// Defaults to Conversational for unrecognized types
    pub fn parse(memory_type_str: &str) -> Self {
        match memory_type_str.to_lowercase().as_str() {
            "conversational" => MemoryType::Conversational,
            "procedural" => MemoryType::Procedural,
            "factual" => MemoryType::Factual,
            "semantic" => MemoryType::Semantic,
            "episodic" => MemoryType::Episodic,
            "personal" => MemoryType::Personal,
            _ => MemoryType::Conversational,
        }
    }

    /// Parse a string into a MemoryType enum with Result
    pub fn parse_with_result(memory_type_str: &str) -> Result<Self, String> {
        match memory_type_str.to_lowercase().as_str() {
            "conversational" => Ok(MemoryType::Conversational),
            "procedural" => Ok(MemoryType::Procedural),
            "factual" => Ok(MemoryType::Factual),
            "semantic" => Ok(MemoryType::Semantic),
            "episodic" => Ok(MemoryType::Episodic),
            "personal" => Ok(MemoryType::Personal),
            _ => Err(format!("Invalid memory type: {}", memory_type_str)),
        }
    }
}

/// Memory search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}

/// Memory operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResult {
    pub id: String,
    pub memory: String,
    pub event: MemoryEvent,
    pub actor_id: Option<String>,
    pub role: Option<String>,
    pub previous_memory: Option<String>,
}

/// Types of memory operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryEvent {
    Add,
    Update,
    Delete,
    None,
}

/// Filters for memory search and retrieval
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filters {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub min_importance: Option<f32>,
    pub max_importance: Option<f32>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub entities: Option<Vec<String>>,
    pub topics: Option<Vec<String>>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Message structure for LLM interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
}

/// Memory action determined by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAction {
    pub id: Option<String>,
    pub text: String,
    pub event: MemoryEvent,
    pub old_memory: Option<String>,
}

impl Memory {
    pub fn new(content: String, embedding: Vec<f32>, metadata: MemoryMetadata) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            embedding,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_content(&mut self, content: String, embedding: Vec<f32>) {
        self.content = content;
        self.embedding = embedding;
        self.updated_at = Utc::now();
        self.metadata.hash = Self::compute_hash(&self.content);
    }

    pub fn compute_hash(content: &str) -> String {
        format!("{:x}", md5::compute(content.as_bytes()))
    }
}

impl MemoryMetadata {
    pub fn new(memory_type: MemoryType) -> Self {
        Self {
            user_id: None,
            agent_id: None,
            run_id: None,
            actor_id: None,
            role: None,
            memory_type,
            hash: String::new(),
            importance_score: 0.5, // Default neutral importance
            entities: Vec::new(),
            topics: Vec::new(),
            custom: HashMap::new(),
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_agent_id(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn with_run_id(mut self, run_id: String) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn with_actor_id(mut self, actor_id: String) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    pub fn with_role(mut self, role: String) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_importance_score(mut self, score: f32) -> Self {
        self.importance_score = score.clamp(0.0, 1.0);
        self
    }

    pub fn with_entities(mut self, entities: Vec<String>) -> Self {
        self.entities = entities;
        self
    }

    pub fn with_topics(mut self, topics: Vec<String>) -> Self {
        self.topics = topics;
        self
    }

    pub fn add_entity(&mut self, entity: String) {
        if !self.entities.contains(&entity) {
            self.entities.push(entity);
        }
    }

    pub fn add_topic(&mut self, topic: String) {
        if !self.topics.contains(&topic) {
            self.topics.push(topic);
        }
    }
}

impl Filters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_user(user_id: &str) -> Self {
        Self {
            user_id: Some(user_id.to_string()),
            ..Default::default()
        }
    }

    pub fn for_agent(agent_id: &str) -> Self {
        Self {
            agent_id: Some(agent_id.to_string()),
            ..Default::default()
        }
    }

    pub fn for_run(run_id: &str) -> Self {
        Self {
            run_id: Some(run_id.to_string()),
            ..Default::default()
        }
    }

    pub fn with_memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }
}

impl Message {
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
        }
    }

    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
        }
    }

    pub fn system<S: Into<String>>(content: S) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
        }
    }

    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }
}

// Optimization types
mod optimization;
pub use optimization::*;
