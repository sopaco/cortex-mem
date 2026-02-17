use crate::{CortexFilesystem, FilesystemOperations, MessageStorage, ParticipantManager, Result};
use crate::llm::LLMClient;
use crate::session::extraction::MemoryExtractor;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Closed,
    Archived,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub thread_id: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub message_count: usize,
    pub participants: Vec<String>, // Participant IDs
    pub tags: Vec<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

impl SessionMetadata {
    /// Create new session metadata
    pub fn new(thread_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            thread_id: thread_id.into(),
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
            closed_at: None,
            message_count: 0,
            participants: Vec::new(),
            tags: Vec::new(),
            title: None,
            description: None,
        }
    }
    
    /// Mark session as closed
    pub fn close(&mut self) {
        self.status = SessionStatus::Closed;
        self.closed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
    
    /// Mark session as archived
    pub fn archive(&mut self) {
        self.status = SessionStatus::Archived;
        self.updated_at = Utc::now();
    }
    
    /// Update message count
    pub fn update_message_count(&mut self, count: usize) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }
    
    /// Add a participant
    pub fn add_participant(&mut self, participant_id: impl Into<String>) {
        let id = participant_id.into();
        if !self.participants.contains(&id) {
            self.participants.push(id);
            self.updated_at = Utc::now();
        }
    }
    
    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let t = tag.into();
        if !self.tags.contains(&t) {
            self.tags.push(t);
            self.updated_at = Utc::now();
        }
    }
    
    /// Set title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
        self.updated_at = Utc::now();
    }
    
    /// Convert to markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Session: {}\n\n", self.thread_id));
        
        if let Some(ref title) = self.title {
            md.push_str(&format!("**Title**: {}\n\n", title));
        }
        
        md.push_str(&format!("**Status**: {:?}\n", self.status));
        md.push_str(&format!("**Created**: {}\n", self.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
        md.push_str(&format!("**Updated**: {}\n", self.updated_at.format("%Y-%m-%d %H:%M:%S UTC")));
        
        if let Some(closed_at) = self.closed_at {
            md.push_str(&format!("**Closed**: {}\n", closed_at.format("%Y-%m-%d %H:%M:%S UTC")));
        }
        
        md.push_str(&format!("**Messages**: {}\n", self.message_count));
        md.push_str(&format!("**Participants**: {}\n", self.participants.len()));
        
        if !self.tags.is_empty() {
            md.push_str(&format!("**Tags**: {}\n", self.tags.join(", ")));
        }
        
        if let Some(ref description) = self.description {
            md.push_str(&format!("\n## Description\n\n{}\n", description));
        }
        
        md.push_str("\n## Participants\n\n");
        for participant in &self.participants {
            md.push_str(&format!("- {}\n", participant));
        }
        
        md
    }
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub auto_extract_on_close: bool,
    pub max_messages_per_session: Option<usize>,
    pub auto_archive_after_days: Option<i64>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_extract_on_close: true,
            max_messages_per_session: None,
            auto_archive_after_days: Some(30),
        }
    }
}

/// Statistics for memory extraction
#[derive(Debug, Clone, Default)]
pub struct ExtractionStats {
    pub preferences: usize,
    pub entities: usize,
    pub events: usize,
    pub cases: usize,
}

/// Session manager
pub struct SessionManager {
    filesystem: Arc<CortexFilesystem>,
    message_storage: MessageStorage,
    participant_manager: ParticipantManager,
    config: SessionConfig,
    llm_client: Option<Arc<dyn LLMClient>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(filesystem: Arc<CortexFilesystem>, config: SessionConfig) -> Self {
        let message_storage = MessageStorage::new(filesystem.clone());
        let participant_manager = ParticipantManager::new();
        
        Self {
            filesystem,
            message_storage,
            participant_manager,
            config,
            llm_client: None,
        }
    }
    
    /// Create a new session manager with LLM client for memory extraction
    pub fn new_with_llm(
        filesystem: Arc<CortexFilesystem>, 
        config: SessionConfig,
        llm_client: Arc<dyn LLMClient>
    ) -> Self {
        let message_storage = MessageStorage::new(filesystem.clone());
        let participant_manager = ParticipantManager::new();
        
        Self {
            filesystem,
            message_storage,
            participant_manager,
            config,
            llm_client: Some(llm_client),
        }
    }
    
    /// Create a new session
    pub async fn create_session(&self, thread_id: &str) -> Result<SessionMetadata> {
        let metadata = SessionMetadata::new(thread_id);
        
        // Save metadata to filesystem
        let metadata_uri = format!("cortex://session/{}/.session.json", thread_id);
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        self.filesystem.write(&metadata_uri, &metadata_json).await?;
        
        Ok(metadata)
    }
    
    /// Load session metadata
    pub async fn load_session(&self, thread_id: &str) -> Result<SessionMetadata> {
        let metadata_uri = format!("cortex://session/{}/.session.json", thread_id);
        let metadata_json = self.filesystem.read(&metadata_uri).await?;
        let metadata: SessionMetadata = serde_json::from_str(&metadata_json)?;
        Ok(metadata)
    }
    
    /// Update session metadata
    pub async fn update_session(&self, metadata: &SessionMetadata) -> Result<()> {
        let metadata_uri = format!("cortex://session/{}/.session.json", metadata.thread_id);
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        self.filesystem.write(&metadata_uri, &metadata_json).await?;
        Ok(())
    }
    
    /// Close a session
    pub async fn close_session(&mut self, thread_id: &str) -> Result<SessionMetadata> {
        let mut metadata = self.load_session(thread_id).await?;
        metadata.close();
        self.update_session(&metadata).await?;
        
        // Trigger memory extraction if auto_extract_on_close is enabled and LLM client is available
        if self.config.auto_extract_on_close {
            if let Some(ref llm_client) = self.llm_client {
                info!("Auto-extracting memories for session: {}", thread_id);
                
                match self.extract_and_save_memories(thread_id, llm_client.clone()).await {
                    Ok(stats) => {
                        info!(
                            "Memory extraction completed for session {}: {} preferences, {} entities, {} events, {} cases",
                            thread_id, stats.preferences, stats.entities, stats.events, stats.cases
                        );
                    }
                    Err(e) => {
                        warn!("Failed to extract memories for session {}: {}", thread_id, e);
                    }
                }
            } else {
                warn!("Memory extraction skipped for session {}: LLM client not configured", thread_id);
            }
        }
        
        Ok(metadata)
    }
    
    /// Extract and save memories from a session
    async fn extract_and_save_memories(
        &self, 
        thread_id: &str, 
        llm_client: Arc<dyn LLMClient>
    ) -> Result<ExtractionStats> {
        // Get all message URIs from the session timeline
        let message_uris = self.message_storage.list_messages(thread_id).await?;
        
        if message_uris.is_empty() {
            info!("No messages found in session {}, skipping extraction", thread_id);
            return Ok(ExtractionStats::default());
        }
        
        // Read message contents
        let mut messages = Vec::new();
        for uri in &message_uris {
            match self.filesystem.read(uri).await {
                Ok(content) => messages.push(content),
                Err(e) => warn!("Failed to read message {}: {}", uri, e),
            }
        }
        
        // Extract memories using LLM
        let extractor = MemoryExtractor::new(llm_client, self.filesystem.clone());
        let extracted = extractor.extract(&messages).await?;
        
        let stats = ExtractionStats {
            preferences: extracted.preferences.len(),
            entities: extracted.entities.len(),
            events: extracted.events.len(),
            cases: extracted.cases.len(),
        };
        
        // Save extracted memories
        extractor.save_memories(&extracted).await?;
        
        Ok(stats)
    }
    
    /// Archive a session
    pub async fn archive_session(&self, thread_id: &str) -> Result<SessionMetadata> {
        let mut metadata = self.load_session(thread_id).await?;
        metadata.archive();
        self.update_session(&metadata).await?;
        Ok(metadata)
    }
    
    /// Delete a session
    pub async fn delete_session(&self, thread_id: &str) -> Result<()> {
        let session_uri = format!("cortex://session/{}", thread_id);
        self.filesystem.delete(&session_uri).await
    }
    
    /// Check if session exists
    pub async fn session_exists(&self, thread_id: &str) -> Result<bool> {
        let metadata_uri = format!("cortex://session/{}/.session.json", thread_id);
        self.filesystem.exists(&metadata_uri).await
    }
    
    /// Get message storage
    pub fn message_storage(&self) -> &MessageStorage {
        &self.message_storage
    }
    
    /// Get participant manager
    pub fn participant_manager(&mut self) -> &mut ParticipantManager {
        &mut self.participant_manager
    }
}

// 核心功能测试已迁移至 cortex-mem-tools/tests/core_functionality_tests.rs
