use crate::{
    Result,
    FilesystemOperations,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "vector-search")]
use crate::{
    embedding::EmbeddingClient,
    filesystem::CortexFilesystem,
    vector_store::{QdrantVectorStore, VectorStore},
};

#[cfg(feature = "vector-search")]
use std::sync::Arc;

/// Search options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: usize,
    /// Minimum similarity score (0.0 - 1.0)
    pub threshold: f32,
    /// Root URI to search in
    pub root_uri: Option<String>,
    /// Enable recursive search
    pub recursive: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            threshold: 0.5,
            root_uri: None,
            recursive: true,
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// URI of the result
    pub uri: String,
    /// Similarity score
    pub score: f32,
    /// Content snippet
    pub snippet: String,
    /// Full content (if loaded)
    pub content: Option<String>,
}

/// Vector search engine (requires vector-search feature)
#[cfg(feature = "vector-search")]
pub struct VectorSearchEngine {
    qdrant: Arc<QdrantVectorStore>,
    embedding: Arc<EmbeddingClient>,
    filesystem: Arc<CortexFilesystem>,
}

#[cfg(feature = "vector-search")]
impl VectorSearchEngine {
    pub fn new(
        qdrant: Arc<QdrantVectorStore>,
        embedding: Arc<EmbeddingClient>,
        filesystem: Arc<CortexFilesystem>,
    ) -> Self {
        Self {
            qdrant,
            embedding,
            filesystem,
        }
    }

    /// Semantic search using vector similarity
    pub async fn semantic_search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 1. Generate query embedding
        let query_vec = self.embedding.embed(query).await?;

        // 2. Search in Qdrant
        let filters = crate::types::Filters::default();
        let scored = self.qdrant
            .as_ref()
            .search_with_threshold(&query_vec, &filters, options.limit, Some(options.threshold))
            .await?;

        // 3. Enrich results with content
        let mut results = Vec::new();
        for scored_mem in scored {
            let snippet = if scored_mem.memory.content.len() > 200 {
                format!("{}...", &scored_mem.memory.content[..200])
            } else {
                scored_mem.memory.content.clone()
            };

            results.push(SearchResult {
                uri: scored_mem.memory.id.clone(),
                score: scored_mem.score,
                snippet,
                content: Some(scored_mem.memory.content),
            });
        }

        Ok(results)
    }
    
    /// Layered semantic search - utilizes L0/L1/L2 three-layer architecture
    /// 
    /// This method implements a three-stage retrieval strategy:
    /// 1. Stage 1 (L0): Fast positioning using .abstract.md files
    /// 2. Stage 2 (L1): Deep exploration using .overview.md files  
    /// 3. Stage 3 (L2): Precise matching using full message content
    /// 
    /// Combined scoring: 0.2*L0 + 0.3*L1 + 0.5*L2
    pub async fn layered_semantic_search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        use tracing::{info, warn};
        
        // Generate query embedding once
        let query_vec = self.embedding.embed(query).await?;
        
        // Stage 1: L0 fast positioning - search .abstract.md
        info!("Stage 1: Scanning L0 abstract layer");
        let l0_filters = crate::types::Filters::with_layer("L0");
        
        let l0_results = self.qdrant
            .search_with_threshold(&query_vec, &l0_filters, options.limit * 3, Some(0.6))
            .await?;
        
        if l0_results.is_empty() {
            warn!("No L0 results found, falling back to full search");
            return self.semantic_search(query, options).await;
        }
        
        info!("Found {} L0 candidates", l0_results.len());
        
        // Stage 2: L1 deep exploration - search .overview.md in candidate directories
        info!("Stage 2: Exploring L1 overview layer");
        let mut l1_candidates = Vec::new();
        
        for l0_result in l0_results {
            // Extract base URI (remove #L0 suffix)
            let base_uri = l0_result.memory.id.trim_end_matches("#L0");
            let l1_id = format!("{}#L1", base_uri);
            
            // Try to get L1 layer
            if let Ok(Some(l1_memory)) = self.qdrant.get(&l1_id).await {
                let l1_score = Self::cosine_similarity(&query_vec, &l1_memory.embedding);
                if l1_score >= options.threshold {
                    l1_candidates.push((base_uri.to_string(), l0_result.score, l1_score));
                }
            }
        }
        
        info!("Found {} L1 candidates", l1_candidates.len());
        
        // Stage 3: L2 precise matching - search actual message content
        info!("Stage 3: Searching L2 detail layer");
        let mut final_results = Vec::new();
        
        for (base_uri, l0_score, l1_score) in l1_candidates {
            // Extract timeline URI from base_uri
            // base_uri format: cortex://session/abc/timeline
            let timeline_uri = &base_uri;
            
            // List all files in timeline directory
            if let Ok(entries) = self.filesystem.list(timeline_uri).await {
                for entry in entries {
                    // Skip directories and hidden files
                    if entry.is_directory || !entry.name.ends_with(".md") || entry.name.starts_with('.') {
                        continue;
                    }
                    
                    let l2_id = format!("{}#L2", entry.uri);
                    if let Ok(Some(l2_memory)) = self.qdrant.get(&l2_id).await {
                        let l2_score = Self::cosine_similarity(&query_vec, &l2_memory.embedding);
                        
                        // Combined scoring: 0.2*L0 + 0.3*L1 + 0.5*L2
                        let combined_score = l0_score * 0.2 + l1_score * 0.3 + l2_score * 0.5;
                        
                        if combined_score >= options.threshold {
                            final_results.push(SearchResult {
                                uri: entry.uri,
                                score: combined_score,
                                snippet: Self::extract_snippet(&l2_memory.content, query),
                                content: Some(l2_memory.content),
                            });
                        }
                    }
                }
            }
        }
        
        // Sort and truncate
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        final_results.truncate(options.limit);
        
        info!("Layered search completed: {} final results", final_results.len());
        Ok(final_results)
    }
    
    /// Calculate cosine similarity between two vectors (helper method)
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            0.0
        } else {
            dot_product / (magnitude_a * magnitude_b)
        }
    }
    
    /// Extract snippet around query match
    fn extract_snippet(content: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(100);
            let end = (pos + query.len() + 100).min(content.len());
            let snippet = &content[start..end];
            
            if start > 0 {
                format!("...{}", snippet)
            } else {
                snippet.to_string()
            }
        } else {
            // Return first 200 chars if no match
            if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.to_string()
            }
        }
    }

    /// Recursive directory search (inspired by OpenViking)
    pub async fn recursive_search(
        &self,
        query: &str,
        root_uri: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 1. Analyze intent (future: use LLM for better intent analysis)
        let _intent = self.analyze_intent(query).await?;

        // 2. Initial positioning - find high-score directories
        let initial_results = self.locate_directories(query, root_uri, options).await?;

        // 3. Recursive exploration
        let mut all_results = Vec::new();
        for result in initial_results {
            // If result is a directory, explore it
            if self.is_directory(&result.uri).await? {
                let dir_results = self.explore_directory(&result.uri, query, options).await?;
                all_results.extend(dir_results);
            } else {
                all_results.push(result);
            }
        }

        // 4. Aggregate and sort
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_results.truncate(options.limit);

        Ok(all_results)
    }

    /// Analyze query intent
    async fn analyze_intent(&self, query: &str) -> Result<QueryIntent> {
        // Simple implementation - can be enhanced with LLM
        Ok(QueryIntent {
            query: query.to_string(),
            keywords: query
                .split_whitespace()
                .map(|s| s.to_lowercase())
                .collect(),
        })
    }

    /// Locate high-score directories
    async fn locate_directories(
        &self,
        query: &str,
        _root_uri: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // For now, use semantic search to find relevant files/dirs
        // Future: can be optimized with directory-level embeddings
        self.semantic_search(query, options).await
    }

    /// Explore a directory recursively
    fn explore_directory<'a>(
        &'a self,
        dir_uri: &'a str,
        query: &'a str,
        options: &'a SearchOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.as_ref().list(dir_uri).await?;
            let mut results = Vec::new();

            for entry in entries {
                // Allow .abstract.md and .overview.md, but skip other hidden files
                if entry.name.starts_with('.') && !entry.name.ends_with(".abstract.md") && !entry.name.ends_with(".overview.md") {
                    continue;
                }

                if entry.is_directory && options.recursive {
                    // Recursively search subdirectory
                    let sub_results = self.explore_directory(&entry.uri, query, options).await?;
                    results.extend(sub_results);
                } else if entry.name.ends_with(".md") {
                    // Search in file
                    if let Ok(content) = self.filesystem.as_ref().read(&entry.uri).await {
                        if self.content_matches(query, &content) {
                            let score = self.calculate_relevance(query, &content).await?;
                            if score >= options.threshold {
                                results.push(SearchResult {
                                    uri: entry.uri,
                                    score,
                                    snippet: Self::extract_snippet(&content, query),
                                    content: Some(content),
                                });
                            }
                        }
                    }
                }
            }

            Ok(results)
        })
    }

    /// Check if URI is a directory
    async fn is_directory(&self, uri: &str) -> Result<bool> {
        // Try to list - if successful, it's a directory
        match self.filesystem.as_ref().list(uri).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Simple content matching
    fn content_matches(&self, query: &str, content: &str) -> bool {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        content_lower.contains(&query_lower)
    }

    /// Calculate relevance score
    async fn calculate_relevance(&self, query: &str, content: &str) -> Result<f32> {
        // Generate embeddings and calculate cosine similarity
        let query_vec = self.embedding.embed(query).await?;
        let content_vec = self.embedding.embed(content).await?;

        Ok(Self::cosine_similarity(&query_vec, &content_vec))
    }
}

/// Query intent
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct QueryIntent {
    query: String,
    keywords: Vec<String>,
}

/// Calculate cosine similarity between two vectors (used by VectorSearchEngine and tests)
#[allow(dead_code)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        0.0
    } else {
        dot_product / (magnitude_a * magnitude_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&c, &d) - 0.0).abs() < 0.001);
    }
}
