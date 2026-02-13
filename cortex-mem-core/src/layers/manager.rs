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
    llm_client: Option<Arc<dyn LLMClient>>,
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

    pub fn with_llm(filesystem: Arc<CortexFilesystem>, llm_client: Arc<dyn LLMClient>) -> Self {
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
        
        // 2. Generate L0/L1 (with or without LLM)
        if let Some(llm) = &self.llm_client {
            // ✅ 有 LLM：使用 LLM 生成高质量摘要
            let abstract_text = self.abstract_gen.generate_with_llm(content, llm).await?;
            let abstract_uri = Self::get_layer_uri(uri, ContextLayer::L0Abstract);
            self.filesystem.write(&abstract_uri, &abstract_text).await?;
            
            let overview = self.overview_gen.generate_with_llm(content, llm).await?;
            let overview_uri = Self::get_layer_uri(uri, ContextLayer::L1Overview);
            self.filesystem.write(&overview_uri, &overview).await?;
        } else {
            // ✅ 没有 LLM：使用 fallback 方法（基于规则）
            let abstract_text = self.abstract_gen.generate(content).await?;
            let abstract_uri = Self::get_layer_uri(uri, ContextLayer::L0Abstract);
            self.filesystem.write(&abstract_uri, &abstract_text).await?;
            
            let overview = self.overview_gen.generate(content).await?;
            let overview_uri = Self::get_layer_uri(uri, ContextLayer::L1Overview);
            self.filesystem.write(&overview_uri, &overview).await?;
        }
        
        Ok(())
    }
    
    /// Generate L0/L1 layers for a timeline directory
    /// 
    /// This method generates abstract and overview for an entire timeline directory
    /// by aggregating all messages in the timeline.
    /// 
    /// # Arguments
    /// * `timeline_uri` - URI of the timeline directory (e.g., "cortex://session/abc/timeline")
    /// 
    /// # Example
    /// ```ignore
    /// layer_manager.generate_timeline_layers("cortex://session/abc/timeline").await?;
    /// // Creates:
    /// // - cortex://session/abc/timeline/.abstract.md (L0 - ~100 tokens)
    /// // - cortex://session/abc/timeline/.overview.md (L1 - ~500-2000 tokens)
    /// ```
    pub async fn generate_timeline_layers(&self, timeline_uri: &str) -> Result<()> {
        use tracing::{debug, info};
        
        info!("Generating timeline layers for {}", timeline_uri);
        
        // 1. Read all messages in timeline
        let entries = self.filesystem.list(timeline_uri).await?;
        let mut messages = Vec::new();
        
        for entry in entries {
            // Skip hidden files except layer files
            if entry.name.starts_with('.') {
                continue;
            }
            
            if entry.name.ends_with(".md") && !entry.is_directory {
                match self.filesystem.read(&entry.uri).await {
                    Ok(content) => messages.push((entry.uri, content)),
                    Err(e) => debug!("Failed to read {}: {}", entry.uri, e),
                }
            }
        }
        
        if messages.is_empty() {
            debug!("No messages found in {}", timeline_uri);
            return Ok(());
        }
        
        // 2. Aggregate all content
        let mut all_content = String::new();
        all_content.push_str(&format!("# Timeline: {}\n\n", timeline_uri));
        all_content.push_str(&format!("Total messages: {}\n\n", messages.len()));
        
        for (idx, (_uri, content)) in messages.iter().enumerate() {
            all_content.push_str(&format!("## Message {}\n\n", idx + 1));
            all_content.push_str(content);
            all_content.push_str("\n\n---\n\n");
        }
        
        // 3. Generate L0 abstract (timeline-level)
        let abstract_text = if let Some(llm) = &self.llm_client {
            debug!("Generating L0 abstract with LLM");
            self.abstract_gen.generate_with_llm(&all_content, llm).await?
        } else {
            debug!("Generating L0 abstract with fallback method");
            self.abstract_gen.generate(&all_content).await?
        };
        
        let abstract_uri = format!("{}/.abstract.md", timeline_uri);
        self.filesystem.write(&abstract_uri, &abstract_text).await?;
        info!("Generated L0 abstract: {}", abstract_uri);
        
        // 4. Generate L1 overview (timeline-level)
        let overview = if let Some(llm) = &self.llm_client {
            debug!("Generating L1 overview with LLM");
            self.overview_gen.generate_with_llm(&all_content, llm).await?
        } else {
            debug!("Generating L1 overview with fallback method");
            self.overview_gen.generate(&all_content).await?
        };
        
        let overview_uri = format!("{}/.overview.md", timeline_uri);
        self.filesystem.write(&overview_uri, &overview).await?;
        info!("Generated L1 overview: {}", overview_uri);
        
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

// 核心功能测试已迁移至 cortex-mem-tools/tests/core_functionality_tests.rs
