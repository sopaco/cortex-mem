use crate::{
    ContextLayer, CortexFilesystem, FilesystemOperations, LayerManager, Result,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::intent::{Intent, IntentAnalyzer};
use super::relevance::RelevanceCalculator;

/// Retrieval engine for finding relevant memories
pub struct RetrievalEngine {
    filesystem: Arc<CortexFilesystem>,
    _layer_manager: Arc<LayerManager>,
    intent_analyzer: IntentAnalyzer,
    relevance_calc: RelevanceCalculator,
}

/// Retrieval options
#[derive(Debug, Clone)]
pub struct RetrievalOptions {
    pub top_k: usize,
    pub min_score: f32,
    pub load_details: bool,
    pub max_candidates: usize,
}

impl Default for RetrievalOptions {
    fn default() -> Self {
        Self {
            top_k: 5,
            min_score: 0.3,
            load_details: false,
            max_candidates: 20,
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub uri: String,
    pub score: f32,
    pub snippet: String,
    pub layer: ContextLayer,
}

/// Retrieval result with trace
#[derive(Debug)]
pub struct RetrievalResult {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub trace: RetrievalTrace,
}

/// Retrieval trace for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalTrace {
    pub query: String,
    pub steps: Vec<TraceStep>,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub step_type: StepType,
    pub description: String,
    pub candidates_count: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepType {
    IntentAnalysis,
    L0Scan,
    L1Exploration,
    ResultAggregation,
}

impl RetrievalEngine {
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        layer_manager: Arc<LayerManager>,
    ) -> Self {
        Self {
            filesystem,
            _layer_manager: layer_manager,
            intent_analyzer: IntentAnalyzer::new(),
            relevance_calc: RelevanceCalculator::new(),
        }
    }
    
    /// Search for relevant memories
    pub async fn search(
        &self,
        query: &str,
        scope: &str,
        options: RetrievalOptions,
    ) -> Result<RetrievalResult> {
        let start = std::time::Instant::now();
        let mut trace = RetrievalTrace {
            query: query.to_string(),
            steps: Vec::new(),
            total_duration_ms: 0,
        };
        
        // Step 1: Intent Analysis
        let step_start = std::time::Instant::now();
        let intent = self.intent_analyzer.analyze(query).await?;
        trace.steps.push(TraceStep {
            step_type: StepType::IntentAnalysis,
            description: format!("Keywords: {:?}", intent.keywords),
            candidates_count: intent.keywords.len(),
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
        
        // Step 2: L0 Scan - Find candidate directories
        let step_start = std::time::Instant::now();
        let candidates = self.scan_l0(scope, &intent, options.max_candidates).await?;
        trace.steps.push(TraceStep {
            step_type: StepType::L0Scan,
            description: format!("Scanned {} directories", candidates.len()),
            candidates_count: candidates.len(),
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
        
        // Step 3: L1 Exploration - Detailed search in candidates
        let step_start = std::time::Instant::now();
        let mut results = Vec::new();
        for candidate in candidates {
            let matches = self.explore_directory(&candidate, &intent).await?;
            results.extend(matches);
        }
        trace.steps.push(TraceStep {
            step_type: StepType::L1Exploration,
            description: format!("Found {} matches", results.len()),
            candidates_count: results.len(),
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
        
        // Step 4: Result Aggregation
        let step_start = std::time::Instant::now();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.retain(|r| r.score >= options.min_score);
        results.truncate(options.top_k);
        trace.steps.push(TraceStep {
            step_type: StepType::ResultAggregation,
            description: format!("Top {} results", results.len()),
            candidates_count: results.len(),
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
        
        trace.total_duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(RetrievalResult {
            query: query.to_string(),
            results,
            trace,
        })
    }
    
    /// Scan L0 layer to find candidate directories
    async fn scan_l0(&self, scope: &str, intent: &Intent, max_candidates: usize) -> Result<Vec<String>> {
        let entries = self.filesystem.list(scope).await?;
        
        let mut candidates = Vec::new();
        
        for entry in entries {
            if !entry.is_directory {
                continue;
            }
            
            // Try to load L0 abstract
            let abstract_uri = format!("{}/.abstract.md", entry.uri);
            if let Ok(abstract_text) = self.filesystem.read(&abstract_uri).await {
                let score = self.relevance_calc.calculate(&abstract_text, intent);
                
                if score > 0.2 {
                    candidates.push((entry.uri.clone(), score));
                }
            } else {
                // If no abstract, still include the directory for exploration
                // This allows searching in timeline directories without abstracts
                candidates.push((entry.uri.clone(), 0.5));
            }
        }
        
        // Sort by score and take top candidates
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(max_candidates);
        
        Ok(candidates.into_iter().map(|(uri, _)| uri).collect())
    }
    
    /// Explore a directory for matching files
    fn explore_directory<'a>(&'a self, dir_uri: &'a str, intent: &'a Intent) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.list(dir_uri).await?;
            let mut results = Vec::new();
            
            for entry in entries {
                if entry.is_directory {
                    // Recursively explore subdirectories (e.g., timeline/2026-02/04)
                    let sub_results = self.explore_directory(&entry.uri, intent).await?;
                    results.extend(sub_results);
                    continue;
                }
                
                if entry.name.starts_with('.') {
                    continue; // Skip metadata files
                }
                
                // Load L1 or L2 content
                if let Ok(content) = self.filesystem.read(&entry.uri).await {
                    let score = self.relevance_calc.calculate(&content, intent);
                    
                    if score > 0.3 {
                        results.push(SearchResult {
                            uri: entry.uri.clone(),
                            score,
                            snippet: Self::create_snippet(&content, &intent.keywords),
                            layer: ContextLayer::L2Detail,
                        });
                    }
                }
            }
            
            Ok(results)
        })
    }
    
    /// Create a snippet highlighting keywords
    fn create_snippet(content: &str, keywords: &[String]) -> String {
        // Find first occurrence of any keyword
        let content_lower = content.to_lowercase();
        
        for keyword in keywords {
            if let Some(pos) = content_lower.find(&keyword.to_lowercase()) {
                // Use char indices to avoid breaking UTF-8 boundaries
                let chars: Vec<char> = content.chars().collect();
                let pos_chars = content[..pos].chars().count();
                
                // Extract context around keyword (in characters, not bytes)
                let start = pos_chars.saturating_sub(50);
                let end = (pos_chars + keyword.chars().count() + 50).min(chars.len());
                
                let snippet_chars: String = chars[start..end].iter().collect();
                
                let mut snippet = snippet_chars;
                if start > 0 {
                    snippet = format!("...{}", snippet);
                }
                if end < chars.len() {
                    snippet = format!("{}...", snippet);
                }
                
                return snippet;
            }
        }
        
        // Fallback: first 100 chars
        let chars: Vec<char> = content.chars().collect();
        if chars.len() > 100 {
            let snippet_chars: String = chars[..97].iter().collect();
            format!("{}...", snippet_chars)
        } else {
            content.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_retrieval_engine() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
        fs.initialize().await.unwrap();
        
        let layer_manager = Arc::new(LayerManager::new(fs.clone()));
        let engine = RetrievalEngine::new(fs.clone(), layer_manager.clone());
        
        // Create some test memories with proper structure
        // First, create a memory about OAuth
        let uri1 = "cortex://threads/test/session1/msg1.md";
        let content1 = "# OAuth Implementation\n\nOAuth 2.0 is a secure authentication protocol used for API security.";
        fs.write(uri1, content1).await.unwrap();
        
        // Generate L0 abstract for the session directory
        let abstract1 = "OAuth 2.0 authentication and security implementation";
        fs.write("cortex://threads/test/session1/.abstract.md", abstract1).await.unwrap();
        
        // Create another memory about database
        let uri2 = "cortex://threads/test/session2/msg2.md";
        let content2 = "# Database Setup\n\nPostgreSQL database configuration and schema design.";
        fs.write(uri2, content2).await.unwrap();
        
        // Generate L0 abstract for the second session directory
        let abstract2 = "Database setup and PostgreSQL configuration";
        fs.write("cortex://threads/test/session2/.abstract.md", abstract2).await.unwrap();
        
        // Search for OAuth with relaxed scoring
        let result = engine.search(
            "OAuth authentication security",
            "cortex://threads/test",
            RetrievalOptions {
                top_k: 5,
                min_score: 0.2,  // Lower threshold for testing
                load_details: false,
                max_candidates: 20,
            },
        ).await.unwrap();
        
        // Debug output
        println!("Search results: {} found", result.results.len());
        for (i, r) in result.results.iter().enumerate() {
            println!("  {}: {} (score: {})", i, r.uri, r.score);
        }
        println!("Trace steps:");
        for step in &result.trace.steps {
            println!("  {:?}: {} candidates", step.step_type, step.candidates_count);
        }
        
        assert!(!result.results.is_empty(), "Should find at least one result");
        assert!(result.results[0].uri.contains("msg1"), "First result should be OAuth message");
        assert!(result.trace.steps.len() >= 3, "Should have at least 3 trace steps");
    }
}
