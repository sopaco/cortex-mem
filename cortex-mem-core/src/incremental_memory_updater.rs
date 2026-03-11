//! Incremental Memory Updater Module
//!
//! Handles incremental updates to memories with version tracking.
//! Supports create, update, delete operations with proper deduplication.
//!
//! ## Generic Processing
//!
//! All memory types share the same update flow via the `MemoryItem` trait:
//! `find_existing → format_content → hash → should_update? → create / update`
//! This eliminates per-type boilerplate and keeps each memory type as a thin
//! trait implementation.

use crate::filesystem::{CortexFilesystem, FilesystemOperations};
use crate::llm::LLMClient;
use crate::memory_index::{MemoryMetadata, MemoryScope, MemoryType, MemoryUpdateResult};
use crate::memory_index_manager::MemoryIndexManager;
use crate::memory_events::{DeleteReason, MemoryEvent};
use crate::session::extraction::{
    CaseMemory, EntityMemory, EventMemory, ExtractedMemories, GoalMemory,
    PersonalInfoMemory, PreferenceMemory, RelationshipMemory, WorkHistoryMemory,
};
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};

// ────────────────────────────────────────────────────────────────────────────
//  MemoryItem trait — the single abstraction that replaces 8 process_xxx fns
// ────────────────────────────────────────────────────────────────────────────

/// Trait implemented by every extracted memory type.
///
/// This is the key abstraction that allows `IncrementalMemoryUpdater` to handle
/// all memory types through a single generic `process_items` method.
pub trait MemoryItem {
    /// The primary key used for matching existing memories (e.g. topic, name, person)
    fn key(&self) -> String;

    /// Which `MemoryType` this item maps to
    fn memory_type(&self) -> MemoryType;

    /// Confidence score (0.0–1.0)
    fn confidence(&self) -> f32;

    /// Render the memory as Markdown content
    fn format_content(&self) -> String;

    /// ID prefix used when creating a new memory (e.g. "pref", "entity")
    fn id_prefix(&self) -> &'static str;

    /// Sub-directory under the scope root where files are stored (e.g. "preferences")
    fn file_dir(&self) -> &'static str;
}

// ── Implementations ─────────────────────────────────────────────────────────

impl MemoryItem for PreferenceMemory {
    fn key(&self) -> String { self.topic.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Preference }
    fn confidence(&self) -> f32 { self.confidence }
    fn id_prefix(&self) -> &'static str { "pref" }
    fn file_dir(&self) -> &'static str { "preferences" }
    fn format_content(&self) -> String {
        format!(
            "# {}\n\n{}\n\n**Confidence**: {:.2}",
            self.topic, self.preference, self.confidence
        )
    }
}

impl MemoryItem for EntityMemory {
    fn key(&self) -> String { self.name.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Entity }
    fn confidence(&self) -> f32 { 0.9 }
    fn id_prefix(&self) -> &'static str { "entity" }
    fn file_dir(&self) -> &'static str { "entities" }
    fn format_content(&self) -> String {
        format!(
            "# {}\n\n**Type**: {}\n\n**Description**: {}\n\n**Context**: {}",
            self.name, self.entity_type, self.description, self.context
        )
    }
}

impl MemoryItem for EventMemory {
    fn key(&self) -> String { self.title.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Event }
    fn confidence(&self) -> f32 { 0.8 }
    fn id_prefix(&self) -> &'static str { "event" }
    fn file_dir(&self) -> &'static str { "events" }
    fn format_content(&self) -> String {
        let timestamp = self.timestamp.as_deref().unwrap_or("N/A");
        format!(
            "# {}\n\n**Type**: {}\n\n**Summary**: {}\n\n**Timestamp**: {}",
            self.title, self.event_type, self.summary, timestamp
        )
    }
}

impl MemoryItem for CaseMemory {
    fn key(&self) -> String { self.title.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Case }
    fn confidence(&self) -> f32 { 0.9 }
    fn id_prefix(&self) -> &'static str { "case" }
    fn file_dir(&self) -> &'static str { "cases" }
    fn format_content(&self) -> String {
        let lessons = self
            .lessons_learned
            .iter()
            .map(|l| format!("- {}", l))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "# {}\n\n## Problem\n\n{}\n\n## Solution\n\n{}\n\n## Lessons Learned\n\n{}",
            self.title, self.problem, self.solution, lessons
        )
    }
}

impl MemoryItem for PersonalInfoMemory {
    fn key(&self) -> String { self.category.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::PersonalInfo }
    fn confidence(&self) -> f32 { self.confidence }
    fn id_prefix(&self) -> &'static str { "info" }
    fn file_dir(&self) -> &'static str { "personal_info" }
    fn format_content(&self) -> String {
        format!(
            "# {}\n\n{}\n\n**Confidence**: {:.2}",
            self.category, self.content, self.confidence
        )
    }
}

impl MemoryItem for WorkHistoryMemory {
    fn key(&self) -> String { format!("{}_{}", self.company, self.role) }
    fn memory_type(&self) -> MemoryType { MemoryType::WorkHistory }
    fn confidence(&self) -> f32 { self.confidence }
    fn id_prefix(&self) -> &'static str { "work" }
    fn file_dir(&self) -> &'static str { "work_history" }
    fn format_content(&self) -> String {
        let duration = self.duration.as_deref().unwrap_or("N/A");
        format!(
            "# {} - {}\n\n**Duration**: {}\n\n**Description**: {}\n\n**Confidence**: {:.2}",
            self.company, self.role, duration, self.description, self.confidence
        )
    }
}

impl MemoryItem for RelationshipMemory {
    fn key(&self) -> String { self.person.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Relationship }
    fn confidence(&self) -> f32 { self.confidence }
    fn id_prefix(&self) -> &'static str { "rel" }
    fn file_dir(&self) -> &'static str { "relationships" }
    fn format_content(&self) -> String {
        format!(
            "# {}\n\n**Type**: {}\n\n**Context**: {}\n\n**Confidence**: {:.2}",
            self.person, self.relation_type, self.context, self.confidence
        )
    }
}

impl MemoryItem for GoalMemory {
    fn key(&self) -> String { self.goal.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Goal }
    fn confidence(&self) -> f32 { self.confidence }
    fn id_prefix(&self) -> &'static str { "goal" }
    fn file_dir(&self) -> &'static str { "goals" }
    fn format_content(&self) -> String {
        let timeline = self.timeline.as_deref().unwrap_or("未指定");
        format!(
            "# {}\n\n**Category**: {}\n\n**Timeline**: {}\n\n**Confidence**: {:.2}",
            self.goal, self.category, timeline, self.confidence
        )
    }
}

// ────────────────────────────────────────────────────────────────────────────
//  IncrementalMemoryUpdater
// ────────────────────────────────────────────────────────────────────────────

/// Incremental Memory Updater
///
/// Handles incremental updates to user and agent memories.
/// Emits events for each operation to trigger cascading updates.
pub struct IncrementalMemoryUpdater {
    filesystem: Arc<CortexFilesystem>,
    index_manager: Arc<MemoryIndexManager>,
    /// LLM client for future content comparison and merge features
    #[allow(dead_code)]
    llm_client: Arc<dyn LLMClient>,
    event_tx: mpsc::UnboundedSender<MemoryEvent>,
}

impl IncrementalMemoryUpdater {
    /// Create a new incremental memory updater
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        index_manager: Arc<MemoryIndexManager>,
        llm_client: Arc<dyn LLMClient>,
        event_tx: mpsc::UnboundedSender<MemoryEvent>,
    ) -> Self {
        Self {
            filesystem,
            index_manager,
            llm_client,
            event_tx,
        }
    }

    /// Update memories from extracted session data
    ///
    /// This is the main entry point for memory updates during session close.
    /// It handles creation, update, and deletion with proper event emission.
    pub async fn update_memories(
        &self,
        user_id: &str,
        agent_id: &str,
        session_id: &str,
        extracted: &ExtractedMemories,
    ) -> Result<MemoryUpdateResult> {
        let mut result = MemoryUpdateResult::default();

        // Process user-scoped memory types
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.preferences).await?;
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.entities).await?;
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.events).await?;
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.personal_info).await?;
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.work_history).await?;
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.relationships).await?;
        self.process_items(&mut result, &MemoryScope::User, user_id, session_id, &extracted.goals).await?;

        // Process agent-scoped memory types
        self.process_items(&mut result, &MemoryScope::Agent, agent_id, session_id, &extracted.cases).await?;

        // Record session extraction summary
        self.index_manager.record_session_extraction(
            &MemoryScope::User,
            user_id,
            session_id,
            result.created_ids.clone(),
            result.updated_ids.clone(),
        ).await?;

        info!(
            "Memory update complete for session {}: {} created, {} updated, {} deleted",
            session_id, result.created, result.updated, result.deleted
        );

        Ok(result)
    }

    // ────────────────────────────────────────────────────────────────────────
    //  Generic processing — the heart of the deduplication
    // ────────────────────────────────────────────────────────────────────────

    /// Process a slice of `MemoryItem` values through the standard pipeline:
    /// find-existing → compare → create / update.
    async fn process_items<T: MemoryItem>(
        &self,
        result: &mut MemoryUpdateResult,
        scope: &MemoryScope,
        owner_id: &str,
        session_id: &str,
        items: &[T],
    ) -> Result<()> {
        for item in items {
            let key = item.key();
            let memory_type = item.memory_type();
            let confidence = item.confidence();
            let content = item.format_content();
            let content_hash = MemoryIndexManager::calculate_content_hash(&content);
            let content_summary = MemoryIndexManager::generate_content_summary(&content, 200);

            let existing = self
                .index_manager
                .find_matching_memory(scope, owner_id, &memory_type, &key)
                .await?;

            match existing {
                Some(existing_meta) => {
                    if self.should_update(&existing_meta, confidence, &content_hash, &content_summary).await? {
                        self.do_update_memory(
                            result, scope, owner_id, session_id,
                            existing_meta, content, content_hash, content_summary, confidence,
                        ).await?;
                    }
                }
                None => {
                    self.do_create_memory(
                        result, scope, owner_id, session_id,
                        item, content, content_hash, content_summary,
                    ).await?;
                }
            }
        }
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    //  Create / Update / Delete — scope-agnostic helpers
    // ────────────────────────────────────────────────────────────────────────

    /// Create a new memory (works for any scope)
    async fn do_create_memory<T: MemoryItem>(
        &self,
        result: &mut MemoryUpdateResult,
        scope: &MemoryScope,
        owner_id: &str,
        session_id: &str,
        item: &T,
        content: String,
        content_hash: String,
        content_summary: String,
    ) -> Result<()> {
        let memory_id = format!(
            "{}_{}",
            item.id_prefix(),
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );
        let file_path = format!("{}/{}.md", item.file_dir(), memory_id);
        // MemoryScope implements Display as lowercase ("user", "agent", ...)
        let file_uri = format!("cortex://{}/{}/{}", scope, owner_id, file_path);

        // Write content
        let timestamped_content = Self::add_timestamp(&content);
        self.filesystem.write(&file_uri, &timestamped_content).await?;

        // Create metadata
        let metadata = MemoryMetadata::new(
            memory_id.clone(),
            file_path,
            item.memory_type(),
            item.key(),
            content_hash,
            session_id,
            item.confidence(),
            content_summary,
        );

        // Update index
        self.index_manager.upsert_memory(scope, owner_id, metadata).await?;

        // Emit event
        let _ = self.event_tx.send(MemoryEvent::MemoryCreated {
            scope: scope.clone(),
            owner_id: owner_id.to_string(),
            memory_id: memory_id.clone(),
            memory_type: item.memory_type(),
            key: item.key(),
            source_session: session_id.to_string(),
            file_uri,
        });

        result.created += 1;
        result.created_ids.push(memory_id);

        Ok(())
    }

    /// Update an existing memory (works for any scope)
    async fn do_update_memory(
        &self,
        result: &mut MemoryUpdateResult,
        scope: &MemoryScope,
        owner_id: &str,
        session_id: &str,
        existing: MemoryMetadata,
        content: String,
        content_hash: String,
        content_summary: String,
        confidence: f32,
    ) -> Result<()> {
        // MemoryScope implements Display as lowercase ("user", "agent", ...)
        let file_uri = format!("cortex://{}/{}/{}", scope, owner_id, existing.file);
        let memory_id = existing.id.clone();
        let old_hash = existing.content_hash.clone();
        let new_hash = content_hash.clone();

        // Write updated content
        let timestamped_content = Self::add_timestamp(&content);
        self.filesystem.write(&file_uri, &timestamped_content).await?;

        // Update metadata
        let mut updated_meta = existing.clone();
        updated_meta.update(content_hash, session_id, confidence, content_summary);

        // Update index
        self.index_manager.upsert_memory(scope, owner_id, updated_meta).await?;

        // Emit event
        let _ = self.event_tx.send(MemoryEvent::MemoryUpdated {
            scope: scope.clone(),
            owner_id: owner_id.to_string(),
            memory_id: memory_id.clone(),
            memory_type: existing.memory_type.clone(),
            key: existing.key.clone(),
            source_session: session_id.to_string(),
            file_uri: file_uri.clone(),
            old_content_hash: old_hash,
            new_content_hash: new_hash,
        });

        result.updated += 1;
        result.updated_ids.push(memory_id.clone());

        debug!("Updated memory {} for {}/{}", memory_id, scope, owner_id);
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    //  Decision helpers
    // ────────────────────────────────────────────────────────────────────────

    /// Check if an existing memory should be updated
    async fn should_update(
        &self,
        existing: &MemoryMetadata,
        new_confidence: f32,
        new_hash: &str,
        new_summary: &str,
    ) -> Result<bool> {
        // Update if new confidence is significantly higher
        if new_confidence > existing.confidence + 0.1 {
            return Ok(true);
        }

        // Update if content changed
        if MemoryIndexManager::content_changed(
            &existing.content_hash,
            new_hash,
            &existing.content_summary,
            new_summary,
        ) {
            return Ok(true);
        }

        Ok(false)
    }

    // ────────────────────────────────────────────────────────────────────────
    //  Utility
    // ────────────────────────────────────────────────────────────────────────

    fn add_timestamp(content: &str) -> String {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        format!("{}\n\n**Added**: {}", content, timestamp)
    }

    /// Delete a memory
    pub async fn delete_memory(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        reason: DeleteReason,
    ) -> Result<bool> {
        // Get metadata first
        let index = self.index_manager.load_index(scope.clone(), owner_id.to_string()).await?;

        if let Some(metadata) = index.memories.get(memory_id).cloned() {
            // MemoryScope implements Display as lowercase ("user", "agent", ...)
            let file_uri = format!("cortex://{}/{}/{}", scope, owner_id, metadata.file);

            // Delete file
            if self.filesystem.exists(&file_uri).await? {
                self.filesystem.delete(&file_uri).await?;
            }

            // Remove from index
            self.index_manager.remove_memory(scope, owner_id, memory_id).await?;

            // Emit event
            let _ = self.event_tx.send(MemoryEvent::MemoryDeleted {
                scope: scope.clone(),
                owner_id: owner_id.to_string(),
                memory_id: memory_id.to_string(),
                memory_type: metadata.memory_type,
                file_uri,
                reason,
            });

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
