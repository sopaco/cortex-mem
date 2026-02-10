// Tiered Access Tools - OpenViking style L0/L1/L2 access

use crate::{Result, types::*, MemoryOperations};
use cortex_mem_core::{ContextLayer, FilesystemOperations};

impl MemoryOperations {
    /// Get L0 abstract (~100 tokens) - for quick relevance checking
    pub async fn get_abstract(&self, uri: &str) -> Result<AbstractResponse> {
        let abstract_text = self.layer_manager
            .load(uri, ContextLayer::L0Abstract)
            .await?;
        
        Ok(AbstractResponse {
            uri: uri.to_string(),
            abstract_text: abstract_text.clone(),
            layer: "L0".to_string(),
            token_count: abstract_text.split_whitespace().count(),
        })
    }
    
    /// Get L1 overview (~2000 tokens) - for understanding core information
    pub async fn get_overview(&self, uri: &str) -> Result<OverviewResponse> {
        let overview_text = self.layer_manager
            .load(uri, ContextLayer::L1Overview)
            .await?;
        
        Ok(OverviewResponse {
            uri: uri.to_string(),
            overview_text: overview_text.clone(),
            layer: "L1".to_string(),
            token_count: overview_text.split_whitespace().count(),
        })
    }
    
    /// Get L2 complete content - only when detailed information is needed
    pub async fn get_read(&self, uri: &str) -> Result<ReadResponse> {
        let content = self.filesystem.read(uri).await?;
        
        // TODO: Get actual metadata from filesystem
        let metadata = Some(FileMetadata {
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        
        Ok(ReadResponse {
            uri: uri.to_string(),
            content: content.clone(),
            layer: "L2".to_string(),
            token_count: content.split_whitespace().count(),
            metadata,
        })
    }
}
