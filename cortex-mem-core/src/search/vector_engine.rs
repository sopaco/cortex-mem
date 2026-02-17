use crate::{
    ContextLayer, FilesystemOperations, Result,
    embedding::EmbeddingClient,
    filesystem::CortexFilesystem,
    llm::LLMClient,
    vector_store::{QdrantVectorStore, VectorStore, uri_to_vector_id},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

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

/// Query intent analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryIntent {
    /// Original query
    pub original_query: String,
    /// Rewritten/expanded query for better retrieval
    pub rewritten_query: Option<String>,
    /// Extracted keywords
    pub keywords: Vec<String>,
    /// Detected intent type
    pub intent_type: QueryIntentType,
}

/// Types of query intents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryIntentType {
    /// Factual question
    Factual,
    /// Searching for specific content
    Search,
    /// Comparing or relating concepts
    Relational,
    /// Looking for recent information
    Temporal,
    /// General/broad query
    General,
}

/// Vector search engine with L0/L1/L2 layered search support
pub struct VectorSearchEngine {
    qdrant: Arc<QdrantVectorStore>,
    embedding: Arc<EmbeddingClient>,
    filesystem: Arc<CortexFilesystem>,
    /// Optional LLM client for query rewriting
    llm_client: Option<Arc<dyn LLMClient>>,
}

impl VectorSearchEngine {
    /// Create a new vector search engine
    pub fn new(
        qdrant: Arc<QdrantVectorStore>,
        embedding: Arc<EmbeddingClient>,
        filesystem: Arc<CortexFilesystem>,
    ) -> Self {
        Self {
            qdrant,
            embedding,
            filesystem,
            llm_client: None,
        }
    }

    /// Create a new vector search engine with LLM support for query rewriting
    pub fn with_llm(
        qdrant: Arc<QdrantVectorStore>,
        embedding: Arc<EmbeddingClient>,
        filesystem: Arc<CortexFilesystem>,
        llm_client: Arc<dyn LLMClient>,
    ) -> Self {
        Self {
            qdrant,
            embedding,
            filesystem,
            llm_client: Some(llm_client),
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
        let scored = self
            .qdrant
            .as_ref()
            .search_with_threshold(&query_vec, &filters, options.limit, Some(options.threshold))
            .await?;

        // 3. Enrich results with content
        let mut results = Vec::new();
        for scored_mem in scored {
            let snippet = if scored_mem.memory.content.chars().count() > 200 {
                format!(
                    "{}...",
                    scored_mem
                        .memory
                        .content
                        .chars()
                        .take(200)
                        .collect::<String>()
                )
            } else {
                scored_mem.memory.content.clone()
            };

            // Use metadata.uri if available, otherwise fall back to id
            let uri = scored_mem
                .memory
                .metadata
                .uri
                .clone()
                .unwrap_or_else(|| scored_mem.memory.id.clone());

            results.push(SearchResult {
                uri,
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
        // Analyze and potentially rewrite the query
        let intent = self.analyze_intent(query).await?;
        let search_query = intent.rewritten_query.as_deref().unwrap_or(query);

        if intent.rewritten_query.is_some() {
            info!("Query rewritten: '{}' -> '{}'", query, search_query);
        }
        info!("Query intent: {:?}, keywords: {:?}", intent.intent_type, intent.keywords);

        // Generate query embedding once (use rewritten query if available)
        let query_vec = self.embedding.embed(search_query).await?;

        // Stage 1: L0 fast positioning - search .abstract.md
        info!("Stage 1: Scanning L0 abstract layer");
        let mut l0_filters = crate::types::Filters::with_layer("L0");
        
        // Add URI prefix filter for scope-based searching
        if let Some(scope) = &options.root_uri {
            l0_filters.uri_prefix = Some(scope.clone());
        }

        let l0_results = self
            .qdrant
            .search_with_threshold(&query_vec, &l0_filters, options.limit * 3, Some(0.5))
            .await?;
        
        // Apply URI prefix filter (application-level filtering for reliability)
        let scope_prefix = options.root_uri.as_ref();
        let l0_results: Vec<_> = l0_results.into_iter().filter(|result| {
            if let Some(prefix) = scope_prefix {
                if let Some(uri) = &result.memory.metadata.uri {
                    return uri.starts_with(prefix);
                }
            }
            true
        }).collect();

        if l0_results.is_empty() {
            warn!("No L0 results found, falling back to full search");
            return self.semantic_search(query, options).await;
        }

        info!("Found {} L0 candidates after scope filtering", l0_results.len());

        // Stage 2: L1 deep exploration - search .overview.md in candidate directories
        info!("Stage 2: Exploring L1 overview layer");
        let mut candidates = Vec::new(); // (dir_uri, l0_score, l1_score, is_timeline)

        for l0_result in l0_results {
            // Get L0 file URI from metadata
            let l0_uri = l0_result
                .memory
                .metadata
                .uri
                .clone()
                .unwrap_or_else(|| l0_result.memory.id.clone());
            
            // Extract directory URI from L0 file URI
            // L0 file: cortex://session/xxx/timeline/.abstract.md
            // Directory: cortex://session/xxx/timeline
            let (dir_uri, is_timeline) = Self::extract_directory_from_l0_uri(&l0_uri);
            
            // Generate L1 ID from directory URI (not file URI!)
            let l1_id = uri_to_vector_id(&dir_uri, ContextLayer::L1Overview);

            // Try to get L1 layer, but don't discard if missing
            let l1_score = if let Ok(Some(l1_memory)) = self.qdrant.get(&l1_id).await {
                Self::cosine_similarity(&query_vec, &l1_memory.embedding)
            } else {
                // L1 not found - use L0 score as approximation (weighted lower)
                warn!("L1 layer not found for {}, using L0 score as fallback", dir_uri);
                l0_result.score * 0.8 // Slightly reduce score when L1 is missing
            };
            
            // Only add if combined threshold is likely to be met
            if l0_result.score >= options.threshold * 0.5 || l1_score >= options.threshold * 0.5 {
                candidates.push((dir_uri, l0_result.score, l1_score, is_timeline));
            }
        }

        info!("Found {} candidates after L1 stage", candidates.len());

        // Stage 3: L2 precise matching - search actual message content
        info!("Stage 3: Searching L2 detail layer");
        let mut final_results = Vec::new();

        for (dir_uri, l0_score, l1_score, is_timeline) in candidates {
            if is_timeline {
                // For timeline directories, list individual messages
                if let Ok(entries) = self.filesystem.list(&dir_uri).await {
                    for entry in entries {
                        // Skip directories and hidden files (but allow .abstract.md and .overview.md for metadata)
                        if entry.is_directory
                            || !entry.name.ends_with(".md")
                            || (entry.name.starts_with('.') && !entry.name.ends_with(".abstract.md") && !entry.name.ends_with(".overview.md"))
                        {
                            continue;
                        }

                        let l2_id = uri_to_vector_id(&entry.uri, ContextLayer::L2Detail);
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
            } else {
                // For non-timeline directories (user/agent memories), the L0 URI points to the actual file
                // Try to get L2 content directly
                let l2_id = uri_to_vector_id(&dir_uri, ContextLayer::L2Detail);
                if let Ok(Some(l2_memory)) = self.qdrant.get(&l2_id).await {
                    let l2_score = Self::cosine_similarity(&query_vec, &l2_memory.embedding);
                    let combined_score = l0_score * 0.2 + l1_score * 0.3 + l2_score * 0.5;
                    
                    if combined_score >= options.threshold {
                        final_results.push(SearchResult {
                            uri: dir_uri.clone(),
                            score: combined_score,
                            snippet: Self::extract_snippet(&l2_memory.content, query),
                            content: Some(l2_memory.content),
                        });
                    }
                } else {
                    // L2 not indexed, but L0/L1 were good matches - include with lower score
                    let combined_score = l0_score * 0.4 + l1_score * 0.6;
                    if combined_score >= options.threshold {
                        // Try to read content from filesystem
                        if let Ok(content) = self.filesystem.read(&dir_uri).await {
                            final_results.push(SearchResult {
                                uri: dir_uri,
                                score: combined_score,
                                snippet: Self::extract_snippet(&content, query),
                                content: Some(content),
                            });
                        }
                    }
                }
            }
        }

        // Sort and truncate
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        final_results.truncate(options.limit);

        info!(
            "Layered search completed: {} final results",
            final_results.len()
        );
        Ok(final_results)
    }
    
    /// Extract directory URI from L0 metadata URI
    /// 
    /// Since we now store directory URI in metadata.uri during indexing,
    /// this function is simplified to handle both old and new formats.
    /// 
    /// Returns (directory_uri, is_timeline)
    fn extract_directory_from_l0_uri(l0_uri: &str) -> (String, bool) {
        // New format: metadata.uri is already the directory URI
        // e.g., "cortex://session/abc/timeline" for timeline
        // e.g., "cortex://user/preferences" for user memories
        
        // Check if this looks like a directory URI (no file extension)
        let is_directory = !l0_uri.ends_with(".md") || l0_uri.contains("/.abstract.md") || l0_uri.contains("/.overview.md");
        
        if is_directory {
            // Handle .abstract.md suffix (old format or layer file path)
            if l0_uri.ends_with("/.abstract.md") {
                let dir = &l0_uri[..l0_uri.len() - 13]; // Remove "/.abstract.md"
                return (dir.to_string(), dir.contains("/timeline"));
            }
            if l0_uri.ends_with("/.overview.md") {
                let dir = &l0_uri[..l0_uri.len() - 13]; // Remove "/.overview.md"
                return (dir.to_string(), dir.contains("/timeline"));
            }
            // Already a directory URI
            return (l0_uri.to_string(), l0_uri.contains("/timeline"));
        }
        
        // It's a file URI, extract parent directory
        if let Some(pos) = l0_uri.rfind('/') {
            let dir = &l0_uri[..pos];
            return (dir.to_string(), dir.contains("/timeline"));
        }
        
        (l0_uri.to_string(), false)
    }

    /// Calculate cosine similarity between two vectors
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

        if let Some(byte_pos_in_lower) = content_lower.find(&query_lower) {
            // Calculate character position in content_lower
            let char_pos = content_lower[..byte_pos_in_lower].chars().count();
            
            // Since content and content_lower have the same number of characters
            // (case conversion doesn't change char count), we can use the same char_pos
            // to locate the position in original content
            let query_char_len = query.chars().count();
            
            // Calculate start and end in char indices
            let start_char = char_pos.saturating_sub(100);
            let end_char = (char_pos + query_char_len + 100).min(content.chars().count());
            
            // Extract snippet using char indices from original content
            let snippet: String = content.chars()
                .skip(start_char)
                .take(end_char - start_char)
                .collect();

            if start_char > 0 {
                format!("...{}", snippet)
            } else {
                snippet
            }
        } else {
            // Return first 200 chars if no match
            if content.chars().count() > 200 {
                format!("{}...", content.chars().take(200).collect::<String>())
            } else {
                content.to_string()
            }
        }
    }

    /// Analyze query intent and optionally rewrite/expand the query
    /// 
    /// If LLM client is available, uses it for intelligent query rewriting.
    /// Otherwise, falls back to simple keyword extraction.
    async fn analyze_intent(&self, query: &str) -> Result<QueryIntent> {
        // Simple keyword extraction (always performed)
        let keywords: Vec<String> = query
            .split_whitespace()
            .filter(|w| w.len() > 2) // Filter out very short words
            .map(|s| s.to_lowercase())
            .collect();

        // Determine basic intent type from query patterns
        let intent_type = Self::detect_intent_type(query);

        // If LLM client is available, attempt query rewriting
        if let Some(llm) = &self.llm_client {
            match self.rewrite_query_with_llm(llm.as_ref(), query).await {
                Ok(rewritten) => {
                    return Ok(QueryIntent {
                        original_query: query.to_string(),
                        rewritten_query: Some(rewritten),
                        keywords,
                        intent_type,
                    });
                }
                Err(e) => {
                    warn!("Query rewrite failed, using original query: {}", e);
                }
            }
        }

        Ok(QueryIntent {
            original_query: query.to_string(),
            rewritten_query: None,
            keywords,
            intent_type,
        })
    }

    /// Detect intent type from query patterns
    fn detect_intent_type(query: &str) -> QueryIntentType {
        let lower = query.to_lowercase();
        
        // Temporal patterns
        if lower.contains("when") || lower.contains("recent") || lower.contains("latest") 
           || lower.contains("yesterday") || lower.contains("last") || lower.contains("ago") {
            return QueryIntentType::Temporal;
        }
        
        // Factual patterns
        if lower.starts_with("what is") || lower.starts_with("who is") 
           || lower.starts_with("how to") || lower.starts_with("define") {
            return QueryIntentType::Factual;
        }
        
        // Relational patterns
        if lower.contains(" vs ") || lower.contains(" versus ") 
           || lower.contains("compared to") || lower.contains("difference between")
           || lower.contains("related to") || lower.contains("connected with") {
            return QueryIntentType::Relational;
        }
        
        // Search patterns
        if lower.starts_with("find") || lower.starts_with("search") 
           || lower.starts_with("show me") || lower.starts_with("list") {
            return QueryIntentType::Search;
        }
        
        QueryIntentType::General
    }

    /// Rewrite query using LLM for better semantic matching
    async fn rewrite_query_with_llm(&self, llm: &dyn LLMClient, query: &str) -> Result<String> {
        let prompt = format!(
            r#"You are a query rewriting assistant for a semantic search system. 
Your task is to rewrite the user's query to improve retrieval accuracy.

Rules:
1. Expand abbreviations and clarify ambiguous terms
2. Add relevant synonyms or related terms that might appear in documents
3. Keep the original meaning - do NOT change the user's intent
4. If the query is already clear and specific, return it unchanged
5. Keep the rewritten query concise (max 50 words)
6. Return ONLY the rewritten query, no explanations

Original query: {}

Rewritten query:"#,
            query
        );

        let response = llm.complete(&prompt).await?;
        
        // Clean up the response
        let rewritten = response
            .trim()
            .lines()
            .next()
            .unwrap_or(query)
            .to_string();

        // If the rewrite is too different or empty, return original
        if rewritten.is_empty() || rewritten.len() > query.len() * 3 {
            return Ok(query.to_string());
        }

        Ok(rewritten)
    }
}