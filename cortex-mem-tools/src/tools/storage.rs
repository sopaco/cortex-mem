// Storage Tools - Store content with automatic layer generation

use crate::{Result, types::*, MemoryOperations};
use cortex_mem_core::{Message, MessageRole};
use std::collections::HashMap;

impl MemoryOperations {
    /// Store content with automatic L0/L1 layer generation
    pub async fn store(&self, args: StoreArgs) -> Result<StoreResponse> {
        let sm = self.session_manager.read().await;
        
        // Ensure session exists
        if !sm.session_exists(&args.thread_id).await? {
            drop(sm);
            let sm_write = self.session_manager.write().await;
            sm_write.create_session(&args.thread_id).await?;
            drop(sm_write);
            // Re-acquire read lock
        }
        
        let sm = self.session_manager.read().await;
        
        // Create and save message
        let message = Message::new(MessageRole::User, &args.content);
        let message_uri = sm.message_storage().save_message(&args.thread_id, &message).await?;
        
        // Auto-generate layers if requested
        let layers_generated = HashMap::new();
        if args.auto_generate_layers.unwrap_or(true) {
            // Use layer_manager to generate all layers
            if let Err(e) = self.layer_manager.generate_all_layers(&message_uri, &args.content).await {
                tracing::warn!("Failed to generate layers: {}", e);
            }
        }
        
        Ok(StoreResponse {
            uri: message_uri,
            layers_generated,
            success: true,
        })
    }
}
