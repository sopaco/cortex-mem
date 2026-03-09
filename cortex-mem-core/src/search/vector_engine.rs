use crate::{
    ContextLayer, FilesystemOperations, Result,
    embedding::EmbeddingClient,
    filesystem::CortexFilesystem,
    llm::LLMClient,
    vector_store::{QdrantVectorStore, VectorStore, uri_to_vector_id},
};
use crate::llm::prompts::Prompts;
use super::{EnhancedQueryIntent, QueryIntentType, TimeConstraint};
use super::weight_model;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

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
            threshold: 0.6,
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

/// Vector search engine with L0/L1/L2 layered search support
pub struct VectorSearchEngine {
    qdrant: Arc<QdrantVectorStore>,
    embedding: Arc<EmbeddingClient>,
    filesystem: Arc<CortexFilesystem>,
    /// Optional LLM client for intent analysis
    llm_client: Option<Arc<dyn LLMClient>>,
}

impl VectorSearchEngine {
    /// Create a new vector search engine (without LLM, intent analysis uses fallback)
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

    /// Create a new vector search engine with LLM support for intent analysis
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

        // 2. Search in Qdrant with scope filter
        let mut filters = crate::types::Filters::default();
        if let Some(scope) = &options.root_uri {
            filters.uri_prefix = Some(scope.clone());
        }

        let scored = self
            .qdrant
            .as_ref()
            .search_with_threshold(&query_vec, &filters, options.limit, Some(options.threshold))
            .await?;

        // 3. Application-level URI prefix filtering (ensures scope isolation)
        let scope_prefix = options.root_uri.as_ref();
        let scored: Vec<_> = scored
            .into_iter()
            .filter(|result| {
                if let Some(prefix) = scope_prefix {
                    if let Some(uri) = &result.memory.metadata.uri {
                        return uri.starts_with(prefix);
                    }
                    return false;
                }
                true
            })
            .collect();

        // 4. Enrich results with content snippets
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
    /// Three-stage retrieval strategy:
    /// 1. Stage 1 (L0): Fast positioning using .abstract.md files
    /// 2. Stage 2 (L1): Deep exploration using .overview.md files
    /// 3. Stage 3 (L2): Precise matching using full message content
    ///
    /// Dynamic scoring weights based on query intent type
    pub async fn layered_semantic_search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 1. LLM 统一意图分析（单次请求）
        let intent = self.analyze_intent(query).await?;

        info!(
            "Intent analysis: type={:?}, entities={:?}, keywords={:?}, rewritten='{}'",
            intent.intent_type, intent.entities, intent.keywords, intent.rewritten_query
        );

        // 2. 用改写后的查询生成 embedding
        let query_vec = self.embedding.embed(&intent.rewritten_query).await?;

        // 3. 根据意图类型动态调整 L0 阈值
        let adaptive_threshold = Self::adaptive_l0_threshold(&intent.intent_type);

        // Stage 1: L0 fast positioning
        info!(
            "Stage 1: Scanning L0 abstract layer with threshold {}",
            adaptive_threshold
        );
        let mut l0_filters = crate::types::Filters::with_layer("L0");
        if let Some(scope) = &options.root_uri {
            l0_filters.uri_prefix = Some(scope.clone());
        }

        let l0_results = self
            .qdrant
            .search_with_threshold(
                &query_vec,
                &l0_filters,
                options.limit * 3,
                Some(adaptive_threshold),
            )
            .await?;

        // Application-level URI prefix filter
        let scope_prefix = options.root_uri.as_ref();
        let l0_results: Vec<_> = l0_results
            .into_iter()
            .filter(|result| {
                if let Some(prefix) = scope_prefix {
                    if let Some(uri) = &result.memory.metadata.uri {
                        return uri.starts_with(prefix);
                    }
                }
                true
            })
            .collect();

        if l0_results.is_empty() {
            warn!(
                "No L0 results at threshold {}, trying fallback",
                adaptive_threshold
            );

            // Fallback 1: relaxed threshold
            let relaxed_threshold = (adaptive_threshold - 0.2).max(0.4);
            info!(
                "Fallback: retrying L0 with relaxed threshold {}",
                relaxed_threshold
            );
            let relaxed_results = self
                .qdrant
                .search_with_threshold(
                    &query_vec,
                    &l0_filters,
                    options.limit * 5,
                    Some(relaxed_threshold),
                )
                .await?;

            let relaxed_results: Vec<_> = relaxed_results
                .into_iter()
                .filter(|result| {
                    if let Some(prefix) = scope_prefix {
                        if let Some(uri) = &result.memory.metadata.uri {
                            return uri.starts_with(prefix);
                        }
                    }
                    true
                })
                .collect();

            if !relaxed_results.is_empty() {
                info!(
                    "Found {} results with relaxed threshold, continuing layered search",
                    relaxed_results.len()
                );
                return self
                    .continue_layered_search(&query_vec, relaxed_results, options, &intent)
                    .await;
            } else {
                // Fallback 2: full semantic search
                warn!("No L0 results even with relaxed threshold, falling back to semantic search");
                return self.semantic_search(query, options).await;
            }
        }

        info!("Found {} L0 candidates", l0_results.len());
        self.continue_layered_search(&query_vec, l0_results, options, &intent)
            .await
    }

    // ── 私有方法 ──────────────────────────────────────────────────────────────

    /// L1/L2 阶段检索（从 L0 候选集出发，逐层深入）
    async fn continue_layered_search(
        &self,
        query_vec: &[f32],
        l0_results: Vec<crate::types::ScoredMemory>,
        options: &SearchOptions,
        intent: &EnhancedQueryIntent,
    ) -> Result<Vec<SearchResult>> {
        // 动态权重：根据意图类型选择 L0/L1/L2 的贡献比例
        let weights = weight_model::weights_for_intent(&intent.intent_type);
        info!(
            "Layer weights: L0={:.2}, L1={:.2}, L2={:.2} (intent={:?})",
            weights.l0, weights.l1, weights.l2, intent.intent_type
        );

        // Stage 2: L1 deep exploration
        info!("Stage 2: Exploring L1 overview layer");
        let mut candidates = Vec::new(); // (dir_uri, l0_score, l1_score, is_timeline)

        for l0_result in l0_results {
            let l0_uri = l0_result
                .memory
                .metadata
                .uri
                .clone()
                .unwrap_or_else(|| l0_result.memory.id.clone());

            let (dir_uri, is_timeline) = Self::extract_directory_from_l0_uri(&l0_uri);
            let l1_id = uri_to_vector_id(&dir_uri, ContextLayer::L1Overview);

            let l1_score = if let Ok(Some(l1_memory)) = self.qdrant.get(&l1_id).await {
                Self::cosine_similarity(query_vec, &l1_memory.embedding)
            } else {
                warn!(
                    "L1 layer not found for {}, using L0 score as fallback",
                    dir_uri
                );
                l0_result.score * 0.8
            };

            if l0_result.score >= options.threshold * 0.5 || l1_score >= options.threshold * 0.5 {
                candidates.push((dir_uri, l0_result.score, l1_score, is_timeline));
            }
        }

        info!("Found {} candidates after L1 stage", candidates.len());

        // Stage 3: L2 precise matching
        info!("Stage 3: Searching L2 detail layer");
        let mut final_results = Vec::new();

        for (dir_uri, l0_score, l1_score, is_timeline) in candidates {
            if is_timeline {
                if let Ok(entries) = self.filesystem.list(&dir_uri).await {
                    for entry in entries {
                        if entry.is_directory
                            || !entry.name.ends_with(".md")
                            || (entry.name.starts_with('.')
                                && !entry.name.ends_with(".abstract.md")
                                && !entry.name.ends_with(".overview.md"))
                        {
                            continue;
                        }

                        let l2_id = uri_to_vector_id(&entry.uri, ContextLayer::L2Detail);
                        if let Ok(Some(l2_memory)) = self.qdrant.get(&l2_id).await {
                            let l2_score =
                                Self::cosine_similarity(query_vec, &l2_memory.embedding);

                            // 动态权重合并分数
                            let combined_score =
                                l0_score * weights.l0 + l1_score * weights.l1 + l2_score * weights.l2;

                            if combined_score >= options.threshold {
                                final_results.push(SearchResult {
                                    uri: entry.uri,
                                    score: combined_score,
                                    snippet: Self::extract_snippet(&l2_memory.content, &intent.rewritten_query),
                                    content: Some(l2_memory.content),
                                });
                            }
                        }
                    }
                }
            } else {
                let l2_id = uri_to_vector_id(&dir_uri, ContextLayer::L2Detail);
                if let Ok(Some(l2_memory)) = self.qdrant.get(&l2_id).await {
                    let l2_score = Self::cosine_similarity(query_vec, &l2_memory.embedding);
                    let combined_score =
                        l0_score * weights.l0 + l1_score * weights.l1 + l2_score * weights.l2;

                    if combined_score >= options.threshold {
                        final_results.push(SearchResult {
                            uri: dir_uri.clone(),
                            score: combined_score,
                            snippet: Self::extract_snippet(&l2_memory.content, &intent.rewritten_query),
                            content: Some(l2_memory.content),
                        });
                    }
                } else {
                    // L2 未索引，仅凭 L0/L1 加权降级
                    let combined_score = l0_score * 0.4 + l1_score * 0.6;
                    if combined_score >= options.threshold {
                        if let Ok(content) = self.filesystem.read(&dir_uri).await {
                            final_results.push(SearchResult {
                                uri: dir_uri,
                                score: combined_score,
                                snippet: Self::extract_snippet(&content, &intent.rewritten_query),
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

    /// 统一意图分析（优先使用 LLM 单次调用，LLM 不可用时使用最小 fallback）
    async fn analyze_intent(&self, query: &str) -> Result<EnhancedQueryIntent> {
        if let Some(llm) = &self.llm_client {
            match self.analyze_intent_with_llm(llm.as_ref(), query).await {
                Ok(intent) => return Ok(intent),
                Err(e) => warn!("LLM intent analysis failed, using fallback: {}", e),
            }
        }

        // Fallback：LLM 不可用时的基础处理（不含规则判断，仅做基本分词）
        debug!("Using fallback intent analysis (no LLM)");
        Ok(EnhancedQueryIntent {
            original_query: query.to_string(),
            rewritten_query: query.to_string(),
            // 使用 chars 保证 Unicode 安全，过滤掉单字符词
            keywords: query
                .split_whitespace()
                .filter(|w| w.chars().count() > 1)
                .map(|s| s.to_lowercase())
                .collect(),
            entities: vec![],
            intent_type: QueryIntentType::General,
            time_constraint: None,
        })
    }

    /// 使用 LLM 进行单次请求的统一意图分析
    async fn analyze_intent_with_llm(
        &self,
        llm: &dyn LLMClient,
        query: &str,
    ) -> Result<EnhancedQueryIntent> {
        let prompt = Prompts::unified_query_analysis(query);
        let response = llm.complete(&prompt).await?;

        // 提取 JSON（兼容 markdown 代码块包裹）
        let json_str = crate::llm::client::LLMClientImpl::extract_json_from_response_static(&response);

        let val: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            crate::Error::Llm(format!(
                "Intent JSON parse error: {}. Response: {}",
                e, json_str
            ))
        })?;

        let intent_type = match val["intent_type"].as_str().unwrap_or("general") {
            "entity_lookup" => QueryIntentType::EntityLookup,
            "factual" => QueryIntentType::Factual,
            "temporal" => QueryIntentType::Temporal,
            "relational" => QueryIntentType::Relational,
            "search" => QueryIntentType::Search,
            _ => QueryIntentType::General,
        };

        let keywords: Vec<String> = val["keywords"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let entities: Vec<String> = val["entities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // 截断保护：rewritten_query 最多 200 个字符
        let rewritten: String = val["rewritten_query"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(query)
            .chars()
            .take(200)
            .collect();

        let time_constraint = if val["time_constraint"].is_null()
            || val["time_constraint"].is_object()
                && val["time_constraint"]["start"].is_null()
                && val["time_constraint"]["end"].is_null()
        {
            None
        } else {
            Some(TimeConstraint {
                start: val["time_constraint"]["start"]
                    .as_str()
                    .map(String::from),
                end: val["time_constraint"]["end"].as_str().map(String::from),
            })
        };

        Ok(EnhancedQueryIntent {
            original_query: query.to_string(),
            rewritten_query: rewritten,
            keywords,
            entities,
            intent_type,
            time_constraint,
        })
    }

    /// 根据意图类型动态调整 L0 检索阈值
    fn adaptive_l0_threshold(intent_type: &QueryIntentType) -> f32 {
        match intent_type {
            // 实体查询：L0 摘要可能丢失实体，用低阈值确保候选集覆盖
            QueryIntentType::EntityLookup => {
                info!("EntityLookup: using lowered L0 threshold 0.35");
                0.35
            }
            QueryIntentType::Factual => {
                info!("Factual query: threshold 0.4");
                0.4
            }
            QueryIntentType::Temporal => {
                info!("Temporal query: threshold 0.45");
                0.45
            }
            QueryIntentType::Search | QueryIntentType::Relational => {
                info!("Search/Relational query: threshold 0.4");
                0.4
            }
            QueryIntentType::General => {
                info!("General query: default threshold 0.5");
                0.5
            }
        }
    }

    /// Extract directory URI from L0 metadata URI
    fn extract_directory_from_l0_uri(l0_uri: &str) -> (String, bool) {
        let is_directory = !l0_uri.ends_with(".md")
            || l0_uri.contains("/.abstract.md")
            || l0_uri.contains("/.overview.md");

        if is_directory {
            if l0_uri.ends_with("/.abstract.md") {
                // 安全移除后缀（使用字节截断是安全的，因为后缀全是 ASCII）
                let dir = &l0_uri[..l0_uri.len() - "/.abstract.md".len()];
                return (dir.to_string(), dir.contains("/timeline"));
            }
            if l0_uri.ends_with("/.overview.md") {
                let dir = &l0_uri[..l0_uri.len() - "/.overview.md".len()];
                return (dir.to_string(), dir.contains("/timeline"));
            }
            return (l0_uri.to_string(), l0_uri.contains("/timeline"));
        }

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

    /// Extract snippet around query match (Unicode safe, uses chars)
    fn extract_snippet(content: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        if let Some(byte_pos_in_lower) = content_lower.find(&query_lower) {
            let char_pos = content_lower[..byte_pos_in_lower].chars().count();
            let query_char_len = query.chars().count();
            let total_chars = content.chars().count();

            let start_char = char_pos.saturating_sub(100);
            let end_char = (char_pos + query_char_len + 100).min(total_chars);

            let snippet: String = content
                .chars()
                .skip(start_char)
                .take(end_char - start_char)
                .collect();

            if start_char > 0 {
                format!("...{}", snippet)
            } else {
                snippet
            }
        } else {
            // Return first 200 chars if no match found
            if content.chars().count() > 200 {
                format!("{}...", content.chars().take(200).collect::<String>())
            } else {
                content.to_string()
            }
        }
    }
}
