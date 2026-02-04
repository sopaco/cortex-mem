use crate::{ContextLayer, CortexFilesystem, FilesystemOperations, Result};
use crate::llm::LLMClient;
use std::sync::Arc;

use super::generator::{AbstractGenerator, OverviewGenerator};

/// Layer Manager
/// 
/// Manages the three-layer memory architecture (L0/L1/L2)
pub struct LayerManager {
    filesystem: Arc<CortexFilesystem>,
    abstract_gen: AbstractGenerator,
    overview_gen: OverviewGenerator,
    llm_client: Option<Arc<LLMClient>>,
}

impl LayerManager {
    pub fn new(filesystem: Arc<CortexFilesystem>) -> Self {
        Self {
            filesystem,
            abstract_gen: AbstractGenerator::new(),
            overview_gen: OverviewGenerator::new(),
            llm_client: None,
        }
    }

    pub fn with_llm(filesystem: Arc<CortexFilesystem>, llm_client: Arc<LLMClient>) -> Self {
        Self {
            filesystem,
            abstract_gen: AbstractGenerator::new(),
            overview_gen: OverviewGenerator::new(),
            llm_client: Some(llm_client),
        }
    }
    
    /// Load content for a specific layer
    pub async fn load(&self, uri: &str, layer: ContextLayer) -> Result<String> {
        match layer {
            ContextLayer::L0Abstract => self.load_abstract(uri).await,
            ContextLayer::L1Overview => self.load_overview(uri).await,
            ContextLayer::L2Detail => self.load_detail(uri).await,
        }
    }
    
    /// Load L0 abstract layer
    async fn load_abstract(&self, uri: &str) -> Result<String> {
        let abstract_uri = Self::get_layer_uri(uri, ContextLayer::L0Abstract);
        
        // If exists, read it
        if self.filesystem.exists(&abstract_uri).await? {
            return self.filesystem.read(&abstract_uri).await;
        }
        
        // Otherwise, generate from L2
        let detail = self.load_detail(uri).await?;
        let abstract_text = self.abstract_gen.generate(&detail).await?;
        
        // Save for future use
        self.filesystem.write(&abstract_uri, &abstract_text).await?;
        
        Ok(abstract_text)
    }
    
    /// Load L1 overview layer
    async fn load_overview(&self, uri: &str) -> Result<String> {
        let overview_uri = Self::get_layer_uri(uri, ContextLayer::L1Overview);
        
        if self.filesystem.exists(&overview_uri).await? {
            return self.filesystem.read(&overview_uri).await;
        }
        
        let detail = self.load_detail(uri).await?;
        let overview = self.overview_gen.generate(&detail).await?;
        
        self.filesystem.write(&overview_uri, &overview).await?;
        
        Ok(overview)
    }
    
    /// Load L2 detail layer (original content)
    async fn load_detail(&self, uri: &str) -> Result<String> {
        self.filesystem.read(uri).await
    }
    
    /// Generate all layers for a new memory
    pub async fn generate_all_layers(&self, uri: &str, content: &str) -> Result<()> {
        // 1. Write L2 (detail)
        self.filesystem.write(uri, content).await?;
        
        // Only generate L0/L1 if LLM client is available
        if let Some(llm) = &self.llm_client {
            // 2. Generate and write L0 (abstract)
            let abstract_text = self.abstract_gen.generate_with_llm(content, llm).await?;
            let abstract_uri = Self::get_layer_uri(uri, ContextLayer::L0Abstract);
            self.filesystem.write(&abstract_uri, &abstract_text).await?;
            
            // 3. Generate and write L1 (overview)
            let overview = self.overview_gen.generate_with_llm(content, llm).await?;
            let overview_uri = Self::get_layer_uri(uri, ContextLayer::L1Overview);
            self.filesystem.write(&overview_uri, &overview).await?;
        }
        
        Ok(())
    }
    
    /// Get layer URI for a base URI
    fn get_layer_uri(base_uri: &str, layer: ContextLayer) -> String {
        match layer {
            ContextLayer::L0Abstract => {
                // Get directory part and append .abstract.md
                let dir = base_uri.rsplit_once('/').map(|(dir, _)| dir).unwrap_or(base_uri);
                format!("{}/.abstract.md", dir)
            }
            ContextLayer::L1Overview => {
                let dir = base_uri.rsplit_once('/').map(|(dir, _)| dir).unwrap_or(base_uri);
                format!("{}/.overview.md", dir)
            }
            ContextLayer::L2Detail => base_uri.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_layer_manager_generate_all() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
        fs.initialize().await.unwrap();
        
        let manager = LayerManager::new(fs.clone());
        
        let uri = "cortex://threads/test/messages/msg1.md";
        let content = "# Test Message\n\nThis is a test about OAuth 2.0.\n\n- Secure\n- Standard";
        
        manager.generate_all_layers(uri, content).await.unwrap();
        
        // Verify L2 exists
        let l2 = manager.load(uri, ContextLayer::L2Detail).await.unwrap();
        assert_eq!(l2, content);
        
        // Verify L0 exists
        let l0 = manager.load(uri, ContextLayer::L0Abstract).await.unwrap();
        assert!(!l0.is_empty());
        assert!(l0.len() <= 200);
        
        // Verify L1 exists
        let l1 = manager.load(uri, ContextLayer::L1Overview).await.unwrap();
        assert!(l1.contains("# Overview"));
    }
    
    #[tokio::test]
    async fn test_lazy_generation() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
        fs.initialize().await.unwrap();
        
        let manager = LayerManager::new(fs.clone());
        
        let uri = "cortex://threads/test/messages/msg2.md";
        let content = "Test content for lazy generation.";
        
        // Write only L2
        fs.write(uri, content).await.unwrap();
        
        // L0 should be generated on-demand
        let l0 = manager.load(uri, ContextLayer::L0Abstract).await.unwrap();
        assert!(!l0.is_empty());
        
        // L0 file should now exist
        let abstract_uri = "cortex://threads/test/messages/.abstract.md";
        assert!(fs.exists(abstract_uri).await.unwrap());
    }
}
