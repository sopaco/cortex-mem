// Storage Tools - Store content with automatic layer generation

use crate::{Result, types::*, MemoryOperations};
use cortex_mem_core::{Message, MessageRole, FilesystemOperations};
use std::collections::HashMap;
use chrono::Utc;

impl MemoryOperations {
    /// Store content with automatic L0/L1 layer generation
    pub async fn store(&self, args: StoreArgs) -> Result<StoreResponse> {
        // Determine storage scope: user, session, or agent
        let scope = match args.scope.as_str() {
            "user" | "session" | "agent" => args.scope.as_str(),
            _ => "session", // Default to session
        };
        
        // Build URI based on scope
        let uri = match scope {
            "user" => {
                // cortex://user/{user_id}/memories/YYYY-MM/DD/HH_MM_SS_id.md
                let user_id = args.user_id.as_deref().unwrap_or("default");
                let now = Utc::now();
                let year_month = now.format("%Y-%m").to_string();
                let day = now.format("%d").to_string();
                let filename = format!(
                    "{}_{}.md",
                    now.format("%H_%M_%S"),
                    uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown")
                );
                format!("cortex://user/{}/memories/{}/{}/{}", user_id, year_month, day, filename)
            },
            "agent" => {
                // cortex://agent/{agent_id}/memories/YYYY-MM/DD/HH_MM_SS_id.md
                let agent_id = args.agent_id.as_deref()
                    .or_else(|| if args.thread_id.is_empty() { None } else { Some(&args.thread_id) })
                    .unwrap_or("default");
                let now = Utc::now();
                let year_month = now.format("%Y-%m").to_string();
                let day = now.format("%d").to_string();
                let filename = format!(
                    "{}_{}.md",
                    now.format("%H_%M_%S"),
                    uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown")
                );
                format!("cortex://agent/{}/memories/{}/{}/{}", agent_id, year_month, day, filename)
            },
            "session" => {
                // cortex://session/{thread_id}/timeline/YYYY-MM/DD/HH_MM_SS_id.md
                let thread_id = if args.thread_id.is_empty() {
                    "default".to_string()
                } else {
                    args.thread_id.clone()
                };
                
                let sm = self.session_manager.read().await;
                
                // Ensure session exists
                if !sm.session_exists(&thread_id).await? {
                    drop(sm);
                    let sm_write = self.session_manager.write().await;
                    sm_write.create_session(&thread_id).await?;
                    drop(sm_write);
                }
                
                let sm = self.session_manager.read().await;
                
                // Create and save message
                let message = Message::new(MessageRole::User, &args.content);
                sm.message_storage().save_message(&thread_id, &message).await?
            },
            _ => unreachable!(),
        };
        
        // For user and agent scope, directly write to filesystem
        if scope == "user" || scope == "agent" {
            self.filesystem.write(&uri, &args.content).await?;
        }
        
        // Auto-generate layers if requested
        let layers_generated = HashMap::new();
        if args.auto_generate_layers.unwrap_or(true) {
            // Use layer_manager to generate all layers
            if let Err(e) = self.layer_manager.generate_all_layers(&uri, &args.content).await {
                tracing::warn!("Failed to generate layers for {}: {}", uri, e);
            }
        }
        
        Ok(StoreResponse {
            uri,
            layers_generated,
            success: true,
        })
    }
}
