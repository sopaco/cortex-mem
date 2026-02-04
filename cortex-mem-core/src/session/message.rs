use crate::{CortexFilesystem, FilesystemOperations, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Message role in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>, // Alias for timestamp, for compatibility
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    /// Create a new message
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        let timestamp = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: content.into(),
            timestamp,
            created_at: timestamp,
            metadata: None,
        }
    }
    
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }
    
    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }
    
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// Convert to markdown format
    pub fn to_markdown(&self) -> String {
        let role_emoji = match self.role {
            MessageRole::User => "👤",
            MessageRole::Assistant => "🤖",
            MessageRole::System => "⚙️",
        };
        
        let timestamp = self.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
        
        let mut md = format!(
            "# {} {:?}\n\n**ID**: `{}`  \n**Timestamp**: {}\n\n",
            role_emoji, self.role, self.id, timestamp
        );
        
        md.push_str("## Content\n\n");
        md.push_str(&self.content);
        md.push_str("\n\n");
        
        if let Some(ref metadata) = self.metadata {
            md.push_str("## Metadata\n\n");
            md.push_str("```json\n");
            md.push_str(&serde_json::to_string_pretty(metadata).unwrap_or_default());
            md.push_str("\n```\n");
        }
        
        md
    }
}

/// Message storage interface
pub struct MessageStorage {
    filesystem: Arc<CortexFilesystem>,
}

impl MessageStorage {
    pub fn new(filesystem: Arc<CortexFilesystem>) -> Self {
        Self { filesystem }
    }
    
    /// Save a message to the timeline
    /// 
    /// URI format: cortex://threads/{thread_id}/timeline/{YYYY-MM}/{DD}/{HH_MM_SS}_{message_id}.md
    pub async fn save_message(&self, thread_id: &str, message: &Message) -> Result<String> {
        let timestamp = message.timestamp;
        
        // Build timeline path: YYYY-MM/DD/HH_MM_SS_id.md
        let year_month = timestamp.format("%Y-%m").to_string();
        let day = timestamp.format("%d").to_string();
        let filename = format!(
            "{}_{}.md",
            timestamp.format("%H_%M_%S"),
            &message.id[..8] // Use first 8 chars of UUID
        );
        
        let uri = format!(
            "cortex://threads/{}/timeline/{}/{}/{}",
            thread_id, year_month, day, filename
        );
        
        // Convert message to markdown
        let content = message.to_markdown();
        
        // Write to filesystem
        self.filesystem.write(&uri, &content).await?;
        
        Ok(uri)
    }
    
    /// Load a message from URI
    pub async fn load_message(&self, uri: &str) -> Result<Message> {
        let content = self.filesystem.read(uri).await?;
        
        // Parse markdown to extract message
        // This is a simplified implementation
        // In production, you'd want more robust parsing
        
        let lines: Vec<&str> = content.lines().collect();
        
        // Extract ID
        let id = lines
            .iter()
            .find(|l| l.starts_with("**ID**:"))
            .and_then(|l| l.split('`').nth(1))
            .unwrap_or("unknown")
            .to_string();
        
        // Extract role
        let role = if content.contains("User") {
            MessageRole::User
        } else if content.contains("Assistant") {
            MessageRole::Assistant
        } else {
            MessageRole::System
        };
        
        // Extract content
        let content_start = content.find("## Content").unwrap_or(0) + 12;
        let content_end = content.find("## Metadata").unwrap_or(content.len());
        let message_content = content[content_start..content_end].trim().to_string();
        
        // Extract timestamp from filename or content
        let timestamp = Utc::now(); // Simplified: should parse from content
        
        Ok(Message {
            id,
            role,
            content: message_content,
            timestamp,
            created_at: timestamp,
            metadata: None,
        })
    }
    
    /// List all messages in a thread
    pub async fn list_messages(&self, thread_id: &str) -> Result<Vec<String>> {
        let timeline_uri = format!("cortex://threads/{}/timeline", thread_id);
        
        // Recursively list all .md files in timeline
        let mut messages = Vec::new();
        
        // This would need a recursive directory walk
        // Simplified implementation for now
        let entries = self.filesystem.list(&timeline_uri).await?;
        
        for entry in entries {
            if entry.name.ends_with(".md") && !entry.name.starts_with('.') {
                messages.push(entry.uri);
            }
        }
        
        Ok(messages)
    }
    
    /// Delete a message
    pub async fn delete_message(&self, uri: &str) -> Result<()> {
        self.filesystem.delete(uri).await
    }
    
    /// Batch save messages
    pub async fn batch_save(
        &self,
        thread_id: &str,
        messages: &[Message],
    ) -> Result<Vec<String>> {
        let mut uris = Vec::new();
        
        for message in messages {
            let uri = self.save_message(thread_id, message).await?;
            uris.push(uri);
        }
        
        Ok(uris)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_message_creation() {
        let msg = Message::user("Hello world");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello world");
        assert!(!msg.id.is_empty());
    }
    
    #[tokio::test]
    async fn test_message_to_markdown() {
        let msg = Message::assistant("Test response");
        let md = msg.to_markdown();
        
        assert!(md.contains("🤖"));
        assert!(md.contains("Assistant"));
        assert!(md.contains("Test response"));
    }
    
    #[tokio::test]
    async fn test_message_storage() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
        fs.initialize().await.unwrap();
        
        let storage = MessageStorage::new(fs.clone());
        
        let msg = Message::user("Hello from test");
        let uri = storage.save_message("test-thread", &msg).await.unwrap();
        
        assert!(uri.contains("cortex://threads/test-thread/timeline"));
        assert!(uri.contains(".md"));
        
        // Verify file was created
        let exists = fs.exists(&uri).await.unwrap();
        assert!(exists);
    }
    
    #[tokio::test]
    async fn test_batch_save() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
        fs.initialize().await.unwrap();
        
        let storage = MessageStorage::new(fs.clone());
        
        let messages = vec![
            Message::user("First message"),
            Message::assistant("First response"),
            Message::user("Second message"),
        ];
        
        let uris = storage.batch_save("test-thread", &messages).await.unwrap();
        
        assert_eq!(uris.len(), 3);
        
        for uri in &uris {
            let exists = fs.exists(uri).await.unwrap();
            assert!(exists);
        }
    }
}
