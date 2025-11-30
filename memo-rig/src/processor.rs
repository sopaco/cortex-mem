use std::sync::Arc;
use tracing::error;

use memo_core::{
    memory::MemoryManager,
    types::{MemoryMetadata, MemoryResult, Message},
    Result,
};

/// A processor responsible for passively learning from conversations.
/// This component should be used by the application/framework layer after each
/// conversation turn to automatically update memories in the background.
pub struct ConversationProcessor {
    memory_manager: Arc<MemoryManager>,
}

impl ConversationProcessor {
    /// Creates a new `ConversationProcessor`.
    ///
    /// # Arguments
    ///
    /// * `memory_manager` - An `Arc` wrapped `MemoryManager` from `memo-core`.
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }

    /// Processes a conversation turn, allowing the memory system to learn from it.
    ///
    /// This method invokes the core `add_memory` function, which triggers the
    /// "extract-retrieve-reason-act" pipeline to intelligently update the knowledge base.
    ///
    /// # Arguments
    ///
    /// * `messages` - A slice of `memo_core::types::Message` representing the conversation turn.
    /// * `metadata` - Metadata associated with the memory, such as `user_id` or `agent_id`.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<MemoryResult>` which details the actions
    /// (`Create`, `Update`, `Delete`, etc.) performed by the memory system.
    pub async fn process_turn(
        &self,
        messages: &[Message],
        metadata: MemoryMetadata,
    ) -> Result<Vec<MemoryResult>> {
        match self.memory_manager.add_memory(messages, metadata).await {
            Ok(results) => Ok(results),
            Err(e) => {
                error!("Failed to process conversation turn for memory: {}", e);
                Err(e)
            }
        }
    }
}
