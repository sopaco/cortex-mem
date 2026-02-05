use crate::{errors::*, types::*};
use cortex_mem_core::{
    CortexFilesystem, SessionManager, SessionConfig, FilesystemOperations,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// High-level memory operations
/// 
/// Provides convenient wrappers around cortex-mem-core functionality
pub struct MemoryOperations {
    filesystem: Arc<CortexFilesystem>,
    session_manager: Arc<RwLock<SessionManager>>,
}

impl MemoryOperations {
    /// Create new memory operations
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        session_manager: Arc<RwLock<SessionManager>>,
    ) -> Self {
        Self {
            filesystem,
            session_manager,
        }
    }

    /// Create from data directory
    pub async fn from_data_dir(data_dir: &str) -> Result<Self> {
        let filesystem = Arc::new(CortexFilesystem::new(data_dir));
        filesystem.initialize().await?;

        let config = SessionConfig::default();
        let session_manager = SessionManager::new(filesystem.clone(), config);
        let session_manager = Arc::new(RwLock::new(session_manager));

        Ok(Self {
            filesystem,
            session_manager,
        })
    }

    /// Add a message to a session
    pub async fn add_message(
        &self,
        thread_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String> {
        let sm = self.session_manager.read().await;
        
        // Ensure session exists
        if !sm.session_exists(thread_id).await? {
            drop(sm);
            let mut sm = self.session_manager.write().await;
            sm.create_session(thread_id).await?;
            drop(sm);
        }
        
        // Add message using MessageStorage
        let sm = self.session_manager.read().await;
        
        // Create message
        let message = cortex_mem_core::Message::new(
            match role {
                "user" => cortex_mem_core::MessageRole::User,
                "assistant" => cortex_mem_core::MessageRole::Assistant,
                "system" => cortex_mem_core::MessageRole::System,
                _ => cortex_mem_core::MessageRole::User,
            },
            content
        );
        
        let message_uri = sm.message_storage().save_message(thread_id, &message).await?;
        
        // Extract message ID from URI
        let message_id = message_uri.rsplit('/').next()
            .unwrap_or("unknown")
            .to_string();
        
        info!("Added message {} to session {}", message_id, thread_id);
        Ok(message_id)
    }

    /// Search memories
    pub async fn search(
        &self,
        query: &str,
        thread_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryInfo>> {
        use cortex_mem_core::retrieval::{RetrievalEngine, RetrievalOptions};
        
        let layer_manager = Arc::new(cortex_mem_core::LayerManager::new(self.filesystem.clone()));
        let engine = RetrievalEngine::new(self.filesystem.clone(), layer_manager);

        let scope = if let Some(thread_id) = thread_id {
            format!("cortex://threads/{}", thread_id)
        } else {
            "cortex://threads".to_string()
        };

        let mut options = RetrievalOptions::default();
        options.top_k = limit;

        let result = engine.search(query, &scope, options).await?;

        let memories: Vec<MemoryInfo> = result.results.into_iter().map(|candidate| {
            MemoryInfo {
                uri: candidate.uri.clone(),
                content: candidate.snippet.clone(),
                score: Some(candidate.score),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        }).collect();

        Ok(memories)
    }

    /// List sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        // List all thread directories
        let entries = self.filesystem.list("cortex://threads").await?;
        
        let mut session_infos = Vec::new();
        for entry in entries {
            if entry.is_directory {
                let thread_id = entry.name;
                if let Ok(metadata) = self.session_manager.read().await.load_session(&thread_id).await {
                    let status_str = match metadata.status {
                        cortex_mem_core::session::manager::SessionStatus::Active => "active",
                        cortex_mem_core::session::manager::SessionStatus::Closed => "closed",
                        cortex_mem_core::session::manager::SessionStatus::Archived => "archived",
                    };
                    
                    session_infos.push(SessionInfo {
                        thread_id: metadata.thread_id,
                        status: status_str.to_string(),
                        message_count: 0,
                        created_at: metadata.created_at,
                        updated_at: metadata.updated_at,
                    });
                }
            }
        }

        Ok(session_infos)
    }

    /// Get session by thread_id
    pub async fn get_session(&self, thread_id: &str) -> Result<SessionInfo> {
        let sm = self.session_manager.read().await;
        let metadata = sm.load_session(thread_id).await?;

        let status_str = match metadata.status {
            cortex_mem_core::session::manager::SessionStatus::Active => "active",
            cortex_mem_core::session::manager::SessionStatus::Closed => "closed",
            cortex_mem_core::session::manager::SessionStatus::Archived => "archived",
        };

        Ok(SessionInfo {
            thread_id: metadata.thread_id,
            status: status_str.to_string(),
            message_count: 0,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
        })
    }

    /// Close session
    pub async fn close_session(&self, thread_id: &str) -> Result<()> {
        let mut sm = self.session_manager.write().await;
        sm.close_session(thread_id).await?;
        info!("Closed session: {}", thread_id);
        Ok(())
    }

    /// Read file from filesystem
    pub async fn read_file(&self, uri: &str) -> Result<String> {
        let content = self.filesystem.read(uri).await?;
        Ok(content)
    }

    /// List files in directory
    pub async fn list_files(&self, uri: &str) -> Result<Vec<String>> {
        let entries = self.filesystem.list(uri).await?;
        let uris = entries.into_iter().map(|e| e.uri).collect();
        Ok(uris)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_operations() {
        let tmpdir = tempfile::tempdir().unwrap();
        let ops = MemoryOperations::from_data_dir(tmpdir.path().to_str().unwrap())
            .await
            .unwrap();

        // Add message
        let msg_id = ops.add_message("test-session", "user", "Hello").await.unwrap();
        assert!(!msg_id.is_empty());

        // Search
        let results = ops.search("Hello", Some("test-session"), 10).await.unwrap();
        assert!(!results.is_empty());

        // List sessions
        let sessions = ops.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
    }
}
