use crate::{Error, FileEntry, FileMetadata, MemoryMetadata, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::uri::UriParser;

/// Trait for filesystem operations
#[async_trait]
pub trait FilesystemOperations: Send + Sync {
    /// List directory contents
    async fn list(&self, uri: &str) -> Result<Vec<FileEntry>>;
    
    /// Read file content
    async fn read(&self, uri: &str) -> Result<String>;
    
    /// Write file content
    async fn write(&self, uri: &str, content: &str) -> Result<()>;
    
    /// Delete file or directory
    async fn delete(&self, uri: &str) -> Result<()>;
    
    /// Check if file/directory exists
    async fn exists(&self, uri: &str) -> Result<bool>;
    
    /// Get file metadata
    async fn metadata(&self, uri: &str) -> Result<FileMetadata>;
}

/// Cortex filesystem implementation
pub struct CortexFilesystem {
    root: PathBuf,
    tenant_id: Option<String>,
}

impl CortexFilesystem {
    /// Create a new CortexFilesystem with the given root directory (no tenant isolation)
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            tenant_id: None,
        }
    }
    
    /// Create a new CortexFilesystem with tenant isolation
    pub fn with_tenant(root: impl AsRef<Path>, tenant_id: impl Into<String>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            tenant_id: Some(tenant_id.into()),
        }
    }
    
    /// Get the root path
    pub fn root_path(&self) -> &Path {
        &self.root
    }
    
    /// Get the tenant ID
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }
    
    /// Initialize the filesystem structure
    pub async fn initialize(&self) -> Result<()> {
        // Get the base directory (with or without tenant)
        let base_dir = if let Some(tenant_id) = &self.tenant_id {
            // For tenant: /root/tenants/{tenant_id}/ (without extra cortex subfolder)
            self.root.join("tenants").join(tenant_id)
        } else {
            // For non-tenant: /root/
            self.root.clone()
        };
        
        // Create root directory
        fs::create_dir_all(&base_dir).await?;
        
        // Create dimension directories (OpenViking style: resources, user, agent, session)
        for dimension in &["resources", "user", "agent", "session"] {
            let dir = base_dir.join(dimension);
            fs::create_dir_all(dir).await?;
        }
        
        Ok(())
    }
    
    /// Get file path from URI (with tenant isolation)
    fn uri_to_path(&self, uri: &str) -> Result<PathBuf> {
        let parsed_uri = UriParser::parse(uri)?;
        
        // If tenant_id exists, add tenant prefix (without extra cortex subfolder)
        let path = if let Some(tenant_id) = &self.tenant_id {
            // /root/tenants/{tenant_id}/{path}
            let tenant_base = self.root.join("tenants").join(tenant_id);
            parsed_uri.to_file_path(&tenant_base)
        } else {
            // /root/{path}
            parsed_uri.to_file_path(&self.root)
        };
        
        Ok(path)
    }
    
    /// Load metadata from .metadata.json
    #[allow(dead_code)]
    async fn load_metadata(&self, dir_path: &Path) -> Result<Option<MemoryMetadata>> {
        let metadata_path = dir_path.join(".metadata.json");
        if !metadata_path.try_exists()? {
            return Ok(None);
        }
        
        let content = fs::read_to_string(metadata_path).await?;
        let metadata: MemoryMetadata = serde_json::from_str(&content)?;
        Ok(Some(metadata))
    }
    
    /// Save metadata to .metadata.json
    #[allow(dead_code)]
    async fn save_metadata(&self, dir_path: &Path, metadata: &MemoryMetadata) -> Result<()> {
        let metadata_path = dir_path.join(".metadata.json");
        let content = serde_json::to_string_pretty(metadata)?;
        fs::write(metadata_path, content).await?;
        Ok(())
    }
}

#[async_trait]
impl FilesystemOperations for CortexFilesystem {
    async fn list(&self, uri: &str) -> Result<Vec<FileEntry>> {
        let path = self.uri_to_path(uri)?;
        
        if !path.try_exists()? {
            return Err(Error::NotFound {
                uri: uri.to_string(),
            });
        }
        
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&path).await?;
        
        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files except .abstract.md and .overview.md
            if name.starts_with('.') 
                && name != ".abstract.md" 
                && name != ".overview.md" 
            {
                continue;
            }
            
            let entry_uri = format!("{}/{}", uri.trim_end_matches('/'), name);
            
            entries.push(FileEntry {
                uri: entry_uri,
                name,
                is_directory: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata.modified()
                    .map(|t| t.into())
                    .unwrap_or_else(|_| Utc::now()),
            });
        }
        
        Ok(entries)
    }
    
    async fn read(&self, uri: &str) -> Result<String> {
        let path = self.uri_to_path(uri)?;
        
        if !path.try_exists()? {
            return Err(Error::NotFound {
                uri: uri.to_string(),
            });
        }
        
        let content = fs::read_to_string(&path).await?;
        Ok(content)
    }
    
    async fn write(&self, uri: &str, content: &str) -> Result<()> {
        let path = self.uri_to_path(uri)?;
        
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(&path, content).await?;
        Ok(())
    }
    
    async fn delete(&self, uri: &str) -> Result<()> {
        let path = self.uri_to_path(uri)?;
        
        if !path.try_exists()? {
            return Err(Error::NotFound {
                uri: uri.to_string(),
            });
        }
        
        if path.is_dir() {
            fs::remove_dir_all(&path).await?;
        } else {
            fs::remove_file(&path).await?;
        }
        
        Ok(())
    }
    
    async fn exists(&self, uri: &str) -> Result<bool> {
        let path = self.uri_to_path(uri)?;
        Ok(path.try_exists().unwrap_or(false))
    }
    
    async fn metadata(&self, uri: &str) -> Result<FileMetadata> {
        let path = self.uri_to_path(uri)?;
        
        if !path.try_exists()? {
            return Err(Error::NotFound {
                uri: uri.to_string(),
            });
        }
        
        let metadata = fs::metadata(&path).await?;
        
        Ok(FileMetadata {
            created_at: metadata.created()
                .map(|t| t.into())
                .unwrap_or_else(|_| Utc::now()),
            updated_at: metadata.modified()
                .map(|t| t.into())
                .unwrap_or_else(|_| Utc::now()),
            size: metadata.len(),
            is_directory: metadata.is_dir(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_filesystem_init() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        
        fs.initialize().await.unwrap();
        
        // OpenViking style dimensions
        assert!(temp_dir.path().join("resources").exists());
        assert!(temp_dir.path().join("user").exists());
        assert!(temp_dir.path().join("agent").exists());
        assert!(temp_dir.path().join("session").exists());
    }
    
    #[tokio::test]
    async fn test_filesystem_with_tenant() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::with_tenant(temp_dir.path(), "agent-a");
        
        fs.initialize().await.unwrap();
        
        // Tenant directories should exist
        let tenant_base = temp_dir.path().join("tenants/agent-a/cortex");
        assert!(tenant_base.join("resources").exists());
        assert!(tenant_base.join("user").exists());
        assert!(tenant_base.join("agent").exists());
        assert!(tenant_base.join("session").exists());
    }
    
    #[tokio::test]
    async fn test_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        fs.initialize().await.unwrap();
        
        let uri = "cortex://session/test123/messages/msg001.md";
        let content = "# Test Message\n\nHello World!";
        
        fs.write(uri, content).await.unwrap();
        
        let read_content = fs.read(uri).await.unwrap();
        assert_eq!(read_content, content);
    }
    
    #[tokio::test]
    async fn test_tenant_isolation() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create two tenant filesystems
        let fs_a = CortexFilesystem::with_tenant(temp_dir.path(), "agent-a");
        let fs_b = CortexFilesystem::with_tenant(temp_dir.path(), "agent-b");
        
        fs_a.initialize().await.unwrap();
        fs_b.initialize().await.unwrap();
        
        // Write to tenant A
        let uri = "cortex://user/memories/entities/Alice.md";
        fs_a.write(uri, "Agent A's memory about Alice").await.unwrap();
        
        // Write to tenant B
        fs_b.write(uri, "Agent B's memory about Alice").await.unwrap();
        
        // Read from each tenant
        let content_a = fs_a.read(uri).await.unwrap();
        let content_b = fs_b.read(uri).await.unwrap();
        
        // They should be different
        assert_eq!(content_a, "Agent A's memory about Alice");
        assert_eq!(content_b, "Agent B's memory about Alice");
        assert_ne!(content_a, content_b);
    }
    
    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        fs.initialize().await.unwrap();
        
        // Write some files
        fs.write("cortex://session/test/msg1.md", "content1").await.unwrap();
        fs.write("cortex://session/test/msg2.md", "content2").await.unwrap();
        
        let entries = fs.list("cortex://session/test").await.unwrap();
        assert_eq!(entries.len(), 2);
    }
    
    #[tokio::test]
    async fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        fs.initialize().await.unwrap();
        
        let uri = "cortex://session/test/file.md";
        assert!(!fs.exists(uri).await.unwrap());
        
        fs.write(uri, "content").await.unwrap();
        assert!(fs.exists(uri).await.unwrap());
    }
}
