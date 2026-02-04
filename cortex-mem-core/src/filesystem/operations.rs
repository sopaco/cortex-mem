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
}

impl CortexFilesystem {
    /// Create a new CortexFilesystem with the given root directory
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
    
    /// Get the root path
    pub fn root_path(&self) -> &Path {
        &self.root
    }
    
    /// Initialize the filesystem structure
    pub async fn initialize(&self) -> Result<()> {
        // Create root directory
        fs::create_dir_all(&self.root).await?;
        
        // Create dimension directories
        for dimension in &["agents", "users", "threads", "global"] {
            let dir = self.root.join(dimension);
            fs::create_dir_all(dir).await?;
        }
        
        Ok(())
    }
    
    /// Get file path from URI
    fn uri_to_path(&self, uri: &str) -> Result<PathBuf> {
        let parsed_uri = UriParser::parse(uri)?;
        Ok(parsed_uri.to_file_path(&self.root))
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
        
        assert!(temp_dir.path().join("agents").exists());
        assert!(temp_dir.path().join("users").exists());
        assert!(temp_dir.path().join("threads").exists());
        assert!(temp_dir.path().join("global").exists());
    }
    
    #[tokio::test]
    async fn test_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        fs.initialize().await.unwrap();
        
        let uri = "cortex://threads/test123/messages/msg001.md";
        let content = "# Test Message\n\nHello World!";
        
        fs.write(uri, content).await.unwrap();
        
        let read_content = fs.read(uri).await.unwrap();
        assert_eq!(read_content, content);
    }
    
    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        fs.initialize().await.unwrap();
        
        // Write some files
        fs.write("cortex://threads/test/msg1.md", "content1").await.unwrap();
        fs.write("cortex://threads/test/msg2.md", "content2").await.unwrap();
        
        let entries = fs.list("cortex://threads/test").await.unwrap();
        assert_eq!(entries.len(), 2);
    }
    
    #[tokio::test]
    async fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let fs = CortexFilesystem::new(temp_dir.path());
        fs.initialize().await.unwrap();
        
        let uri = "cortex://threads/test/file.md";
        assert!(!fs.exists(uri).await.unwrap());
        
        fs.write(uri, "content").await.unwrap();
        assert!(fs.exists(uri).await.unwrap());
    }
}
