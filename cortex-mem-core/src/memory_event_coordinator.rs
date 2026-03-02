//! Memory Event Coordinator Module
//!
//! Central coordinator that handles all memory events and orchestrates
//! the flow between different components.

use crate::cascade_layer_updater::CascadeLayerUpdater;
use crate::embedding::EmbeddingClient;
use crate::filesystem::{CortexFilesystem, FilesystemOperations};
use crate::incremental_memory_updater::IncrementalMemoryUpdater;
use crate::llm::LLMClient;
use crate::memory_events::{ChangeType, DeleteReason, EventStats, MemoryEvent};
use crate::memory_index::MemoryScope;
use crate::memory_index_manager::MemoryIndexManager;
use crate::session::extraction::ExtractedMemories;
use crate::vector_store::QdrantVectorStore;
use crate::vector_sync_manager::VectorSyncManager;
use crate::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Memory Event Coordinator
///
/// Central hub that coordinates all memory operations:
/// - Receives events from various sources
/// - Dispatches to appropriate handlers
/// - Ensures consistency across components
pub struct MemoryEventCoordinator {
    filesystem: Arc<CortexFilesystem>,
    llm_client: Arc<dyn LLMClient>,
    index_manager: Arc<MemoryIndexManager>,
    memory_updater: Arc<IncrementalMemoryUpdater>,
    layer_updater: Arc<CascadeLayerUpdater>,
    vector_sync: Arc<VectorSyncManager>,
    stats: Arc<RwLock<EventStats>>,
}

impl MemoryEventCoordinator {
    /// Create a new memory event coordinator
    /// 
    /// Returns (coordinator, event_sender, event_receiver)
    /// - coordinator: the coordinator instance
    /// - event_sender: use this to send events to the coordinator
    /// - event_receiver: pass this to coordinator.start() to begin processing
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        llm_client: Arc<dyn LLMClient>,
        embedding_client: Arc<EmbeddingClient>,
        vector_store: Arc<QdrantVectorStore>,
    ) -> (Self, mpsc::UnboundedSender<MemoryEvent>, mpsc::UnboundedReceiver<MemoryEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let index_manager = Arc::new(MemoryIndexManager::new(filesystem.clone()));
        
        // Create memory updater with event sender
        let memory_updater = Arc::new(IncrementalMemoryUpdater::new(
            filesystem.clone(),
            index_manager.clone(),
            llm_client.clone(),
            event_tx.clone(),
        ));
        
        // Create layer updater with event sender
        let layer_updater = Arc::new(CascadeLayerUpdater::new(
            filesystem.clone(),
            llm_client.clone(),
            event_tx.clone(),
        ));
        
        // Create vector sync manager
        let vector_sync = Arc::new(VectorSyncManager::new(
            filesystem.clone(),
            embedding_client,
            vector_store,
        ));
        
        let coordinator = Self {
            filesystem,
            llm_client,
            index_manager,
            memory_updater,
            layer_updater,
            vector_sync,
            stats: Arc::new(RwLock::new(EventStats::default())),
        };
        
        (coordinator, event_tx, event_rx)
    }

    /// Start the event processing loop
    /// 
    /// Returns a boxed future that can be spawned on a tokio runtime.
    pub fn start(self, mut event_rx: mpsc::UnboundedReceiver<MemoryEvent>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
        Box::pin(async move {
            info!("Memory Event Coordinator started");
            
            while let Some(event) = event_rx.recv().await {
                if let Err(e) = self.handle_event(event).await {
                    error!("Event handling failed: {}", e);
                }
            }
            
            warn!("Memory Event Coordinator stopped");
        })
    }

    /// Handle a single event
    async fn handle_event(&self, event: MemoryEvent) -> Result<()> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.record(&event);
        }
        
        debug!("Handling event: {}", event);
        
        match event {
            MemoryEvent::MemoryCreated {
                scope,
                owner_id,
                memory_id,
                memory_type,
                key,
                source_session,
                file_uri,
            } => {
                self.on_memory_created(&scope, &owner_id, &memory_id, &memory_type, &key, &source_session, &file_uri).await?;
            }
            
            MemoryEvent::MemoryUpdated {
                scope,
                owner_id,
                memory_id,
                memory_type,
                key,
                source_session,
                file_uri,
                old_content_hash,
                new_content_hash,
            } => {
                self.on_memory_updated(&scope, &owner_id, &memory_id, &memory_type, &key, &source_session, &file_uri, &old_content_hash, &new_content_hash).await?;
            }
            
            MemoryEvent::MemoryDeleted {
                scope,
                owner_id,
                memory_id,
                memory_type,
                file_uri,
                reason,
            } => {
                self.on_memory_deleted(&scope, &owner_id, &memory_id, &memory_type, &file_uri, &reason).await?;
            }
            
            MemoryEvent::MemoryAccessed {
                scope,
                owner_id,
                memory_id,
                context,
            } => {
                self.on_memory_accessed(&scope, &owner_id, &memory_id, &context).await?;
            }
            
            MemoryEvent::LayersUpdated {
                scope,
                owner_id,
                directory_uri,
                layers,
            } => {
                self.on_layers_updated(&scope, &owner_id, &directory_uri, &layers).await?;
            }
            
            MemoryEvent::SessionClosed {
                session_id,
                user_id,
                agent_id,
            } => {
                self.on_session_closed(&session_id, &user_id, &agent_id).await?;
            }
            
            MemoryEvent::LayerUpdateNeeded {
                scope,
                owner_id,
                directory_uri,
                change_type,
                changed_file,
            } => {
                self.on_layer_update_needed(&scope, &owner_id, &directory_uri, &change_type, &changed_file).await?;
            }
            
            MemoryEvent::VectorSyncNeeded {
                file_uri,
                change_type,
            } => {
                self.on_vector_sync_needed(&file_uri, &change_type).await?;
            }
        }
        
        Ok(())
    }

    /// Handle memory created event
    async fn on_memory_created(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        memory_type: &crate::memory_index::MemoryType,
        _key: &str,
        _source_session: &str,
        file_uri: &str,
    ) -> Result<()> {
        debug!(
            "Memory created: {} ({:?}) in {:?}/{}",
            memory_id, memory_type, scope, owner_id
        );
        
        // Trigger layer cascade update
        self.layer_updater
            .on_memory_changed(scope.clone(), owner_id.to_string(), file_uri.to_string(), ChangeType::Add)
            .await?;
        
        // Trigger vector sync
        self.vector_sync
            .sync_file_change(file_uri, ChangeType::Add)
            .await?;
        
        Ok(())
    }

    /// Handle memory updated event
    async fn on_memory_updated(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        memory_type: &crate::memory_index::MemoryType,
        _key: &str,
        _source_session: &str,
        file_uri: &str,
        _old_content_hash: &str,
        _new_content_hash: &str,
    ) -> Result<()> {
        debug!(
            "Memory updated: {} ({:?}) in {:?}/{}",
            memory_id, memory_type, scope, owner_id
        );
        
        // Trigger layer cascade update
        self.layer_updater
            .on_memory_changed(scope.clone(), owner_id.to_string(), file_uri.to_string(), ChangeType::Update)
            .await?;
        
        // Trigger vector sync
        self.vector_sync
            .sync_file_change(file_uri, ChangeType::Update)
            .await?;
        
        Ok(())
    }

    /// Handle memory deleted event
    async fn on_memory_deleted(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        memory_type: &crate::memory_index::MemoryType,
        file_uri: &str,
        reason: &DeleteReason,
    ) -> Result<()> {
        debug!(
            "Memory deleted: {} ({:?}) in {:?}/{}, reason: {:?}",
            memory_id, memory_type, scope, owner_id, reason
        );
        
        // Trigger layer cascade update
        self.layer_updater
            .on_memory_changed(scope.clone(), owner_id.to_string(), file_uri.to_string(), ChangeType::Delete)
            .await?;
        
        // Trigger vector deletion
        self.vector_sync
            .sync_file_change(file_uri, ChangeType::Delete)
            .await?;
        
        Ok(())
    }

    /// Handle memory accessed event
    async fn on_memory_accessed(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        context: &str,
    ) -> Result<()> {
        debug!(
            "Memory accessed: {} in {:?}/{}, context: {}",
            memory_id, scope, owner_id, context
        );
        
        // Record access in index
        self.index_manager.record_access(scope, owner_id, memory_id).await?;
        
        Ok(())
    }

    /// Handle layers updated event
    async fn on_layers_updated(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        directory_uri: &str,
        layers: &[crate::ContextLayer],
    ) -> Result<()> {
        debug!(
            "Layers updated for {} in {:?}/{}: {:?}",
            directory_uri, scope, owner_id, layers
        );
        
        // Sync layer files to vector database
        self.vector_sync.sync_layer_files(directory_uri).await?;
        
        Ok(())
    }

    /// Handle session closed event (the main trigger for memory extraction)
    async fn on_session_closed(
        &self,
        session_id: &str,
        user_id: &str,
        agent_id: &str,
    ) -> Result<()> {
        info!("Processing session closed: {}", session_id);
        
        // 1. Extract memories from the session
        let extracted = self.extract_memories_from_session(session_id).await?;
        
        // 2. Update user memories
        if !extracted.is_empty() {
            let user_result = self.memory_updater
                .update_memories(user_id, agent_id, session_id, &extracted)
                .await?;
            
            info!(
                "User memory update for session {}: {} created, {} updated",
                session_id, user_result.created, user_result.updated
            );
        }
        
        // 3. Update timeline layers
        self.layer_updater.update_timeline_layers(session_id).await?;
        
        // 4. Sync session to vectors
        let timeline_uri = format!("cortex://session/{}/timeline", session_id);
        self.vector_sync.sync_directory(&timeline_uri).await?;
        
        info!("Session {} processing complete", session_id);
        
        Ok(())
    }

    /// Handle layer update needed event
    async fn on_layer_update_needed(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        directory_uri: &str,
        change_type: &ChangeType,
        changed_file: &str,
    ) -> Result<()> {
        debug!(
            "Layer update needed for {} due to {:?} on {}",
            directory_uri, change_type, changed_file
        );
        
        // Update directory layers
        self.layer_updater
            .on_memory_changed(scope.clone(), owner_id.to_string(), changed_file.to_string(), change_type.clone())
            .await?;
        
        Ok(())
    }

    /// Handle vector sync needed event
    async fn on_vector_sync_needed(
        &self,
        file_uri: &str,
        change_type: &ChangeType,
    ) -> Result<()> {
        debug!("Vector sync needed for {}: {:?}", file_uri, change_type);
        
        self.vector_sync.sync_file_change(file_uri, change_type.clone()).await?;
        
        Ok(())
    }

    /// Extract memories from a session using LLM
    async fn extract_memories_from_session(&self, session_id: &str) -> Result<ExtractedMemories> {
        // Collect all messages from the session
        let timeline_uri = format!("cortex://session/{}/timeline", session_id);
        
        let mut messages = Vec::new();
        self.collect_messages_recursive(&timeline_uri, &mut messages).await?;
        
        if messages.is_empty() {
            debug!("No messages found in session {}", session_id);
            return Ok(ExtractedMemories::default());
        }
        
        // Build extraction prompt
        let prompt = self.build_extraction_prompt(&messages);
        
        // Call LLM for extraction
        let response = self.llm_client.complete(&prompt).await?;
        
        // Parse response
        let extracted = self.parse_extraction_response(&response);
        
        info!(
            "Extracted {} memories from session {}",
            extracted.preferences.len() + extracted.entities.len() + extracted.events.len() + extracted.cases.len(),
            session_id
        );
        
        Ok(extracted)
    }

    /// Recursively collect messages from timeline
    fn collect_messages_recursive<'a>(
        &'a self,
        uri: &'a str,
        messages: &'a mut Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.list(uri).await?;
            
            for entry in entries {
                if entry.name.starts_with('.') {
                    continue;
                }
                
                if entry.is_directory {
                    self.collect_messages_recursive(&entry.uri, messages).await?;
                } else if entry.name.ends_with(".md") {
                    if let Ok(content) = self.filesystem.read(&entry.uri).await {
                        messages.push(content);
                    }
                }
            }
            
            Ok(())
        })
    }

    /// Build the extraction prompt
    fn build_extraction_prompt(&self, messages: &[String]) -> String {
        let messages_text = messages.join("\n\n---\n\n");
        
        format!(
            r#"Analyze the following conversation and extract memories in JSON format.

## Instructions

Extract the following types of memories:

1. **Personal Info** (user's personal information):
   - category: "age", "occupation", "education", "location", etc.
   - content: The specific information
   - confidence: 0.0-1.0 confidence level

2. **Work History** (user's work experience):
   - company: Company name
   - role: Job title/role
   - duration: Time period (optional)
   - description: Brief description
   - confidence: 0.0-1.0 confidence level

3. **Preferences** (user preferences by topic):
   - topic: The topic/subject area
   - preference: The user's stated preference
   - confidence: 0.0-1.0 confidence level

4. **Relationships** (people user mentions):
   - person: Person's name
   - relation_type: "family", "colleague", "friend", etc.
   - context: How they're related
   - confidence: 0.0-1.0 confidence level

5. **Goals** (user's goals and aspirations):
   - goal: The specific goal
   - category: "career", "personal", "health", "learning", etc.
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
  "personal_info": [{{ "category": "...", "content": "...", "confidence": 0.9 }}],
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

    /// Parse the LLM extraction response
    fn parse_extraction_response(&self, response: &str) -> ExtractedMemories {
        // Try to extract JSON from the response
        let json_str = if response.starts_with('{') {
            response.to_string()
        } else {
            response
                .find('{')
                .and_then(|start| response.rfind('}').map(|end| &response[start..=end]))
                .map(|s| s.to_string())
                .unwrap_or_default()
        };
        
        if json_str.is_empty() {
            return ExtractedMemories::default();
        }
        
        serde_json::from_str(&json_str).unwrap_or_default()
    }

    /// Get current event statistics
    pub async fn get_stats(&self) -> EventStats {
        self.stats.read().await.clone()
    }

    /// Force a full update for a scope
    pub async fn force_full_update(&self, scope: &MemoryScope, owner_id: &str) -> Result<()> {
        info!("Forcing full update for {:?}/{}", scope, owner_id);
        
        // Update all layers
        self.layer_updater.update_all_layers(scope, owner_id).await?;
        
        // Sync to vectors
        let root_uri = match scope {
            MemoryScope::User => format!("cortex://user/{}", owner_id),
            MemoryScope::Agent => format!("cortex://agent/{}", owner_id),
            MemoryScope::Session => format!("cortex://session/{}", owner_id),
            MemoryScope::Resources => "cortex://resources".to_string(),
        };
        
        self.vector_sync.sync_directory(&root_uri).await?;
        
        Ok(())
    }

    /// Delete all memories for a session
    pub async fn delete_session_memories(&self, session_id: &str, user_id: &str, agent_id: &str) -> Result<()> {
        info!("Deleting all memories for session {}", session_id);
        
        // Delete from index
        let deleted_user = self.index_manager
            .delete_memories_from_session(&MemoryScope::User, user_id, session_id)
            .await?;
        
        let deleted_agent = self.index_manager
            .delete_memories_from_session(&MemoryScope::Agent, agent_id, session_id)
            .await?;
        
        // Delete vectors
        self.vector_sync.delete_session_vectors(session_id).await?;
        
        info!(
            "Deleted {} user memories and {} agent memories for session {}",
            deleted_user.len(),
            deleted_agent.len(),
            session_id
        );
        
        Ok(())
    }
}
