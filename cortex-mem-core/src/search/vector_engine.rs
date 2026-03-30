use crate::{
    ContextLayer, FilesystemOperations, Result,
    embedding::EmbeddingClient,
    filesystem::CortexFilesystem,
    llm::LLMClient,
    memory_events::MemoryEvent,
    memory_index::MemoryScope,
    memory_index_manager::MemoryIndexManager,
    vector_store::{QdrantVectorStore, VectorStore, uri_to_vector_id},
};
use crate::llm::prompts::Prompts;
use super::{EnhancedQueryIntent, QueryIntentType, TimeConstraint};
use super::weight_model;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
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
    /// Optional event sender for MemoryAccessed events (drives forgetting mechanism)
    memory_event_tx: Option<mpsc::UnboundedSender<MemoryEvent>>,
    /// Optional index manager for archived-memory filtering
    index_manager: Option<Arc<MemoryIndexManager>>,
    /// Whether to call the LLM for intent analysis before each search.
    /// When `false`, the raw query is used directly (skips rewriting/threshold tuning).
    /// Default: `true`.
    enable_intent_analysis: bool,
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
            memory_event_tx: None,
            index_manager: None,
            enable_intent_analysis: true,
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
            memory_event_tx: None,
            index_manager: None,
            enable_intent_analysis: true,
        }
    }

    /// Control whether LLM intent analysis is performed before each search.
    ///
    /// Set to `false` to skip the LLM round-trip and use the raw query directly.
    /// Reduces search latency from ~15-25s to <500ms at the cost of no query rewriting.
    pub fn with_intent_analysis(mut self, enabled: bool) -> Self {
        self.enable_intent_analysis = enabled;
        self
    }

    /// Set the memory event sender for access tracking (enables forgetting mechanism)
    pub fn with_memory_event_tx(mut self, tx: mpsc::UnboundedSender<MemoryEvent>) -> Self {
        self.memory_event_tx = Some(tx);
        self
    }

    /// Set the memory index manager for archived-memory filtering
    ///
    /// When configured, search results whose corresponding `MemoryMetadata.archived == true`
    /// will be removed from the result set, preventing stale/forgotten memories from
    /// surfacing in semantic search.
    pub fn with_index_manager(mut self, index_manager: Arc<MemoryIndexManager>) -> Self {
        self.index_manager = Some(index_manager);
        self
    }

    /// Filter out archived memories from a result list.
    ///
    /// Loads the index for each unique (scope, owner_id) combination found in the
    /// results and removes any item whose memory ID is marked as archived.
    /// Results whose URIs cannot be parsed are kept (conservative approach).
    async fn filter_archived(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let im = match &self.index_manager {
            Some(im) => im,
            None => return results,
        };

        // Build a cache of (scope, owner_id) → MemoryIndex to avoid repeated I/O
        let mut index_cache: std::collections::HashMap<
            (MemoryScope, String),
            crate::memory_index::MemoryIndex,
        > = std::collections::HashMap::new();

        let total_before = results.len();
        let mut filtered = Vec::with_capacity(total_before);

        for result in results {
            let keep = match Self::parse_scope_owner_from_uri(&result.uri) {
                None => true, // Cannot parse URI → keep conservatively
                Some((scope, owner_id, memory_id)) => {
                    let key = (scope.clone(), owner_id.clone());
                    let index = if let Some(idx) = index_cache.get(&key) {
                        idx
                    } else {
                        match im.load_index(scope.clone(), owner_id.clone()).await {
                            Ok(idx) => {
                                index_cache.insert(key.clone(), idx);
                                index_cache.get(&key).unwrap()
                            }
                            Err(e) => {
                                warn!("Failed to load index for {}/{}: {}", scope, owner_id, e);
                                filtered.push(result);
                                continue;
                            }
                        }
                    };

                    !index
                        .memories
                        .get(&memory_id)
                        .map(|m| m.archived)
                        .unwrap_or(false)
                }
            };

            if keep {
                filtered.push(result);
            } else {
                debug!("Filtered archived memory: {}", result.uri);
            }
        }

        let archived_count = total_before - filtered.len();
        if archived_count > 0 {
            info!(
                "Filtered {}/{} archived memories from search results",
                archived_count, total_before
            );
        }

        filtered
    }
    ///
    /// Extracts scope/owner from URI and sends events asynchronously.
    /// Failures are logged but do not affect search results.
    fn emit_access_events(&self, results: &[SearchResult], query: &str) {
        let tx = match &self.memory_event_tx {
            Some(tx) => tx,
            None => return,
        };

        for result in results {
            // Parse URI: cortex://{scope}/{owner_id}/...
            if let Some(parsed) = Self::parse_scope_owner_from_uri(&result.uri) {
                let (scope, owner_id, memory_id) = parsed;
                let _ = tx.send(MemoryEvent::MemoryAccessed {
                    scope,
                    owner_id,
                    memory_id,
                    context: query.to_string(),
                });
            }
        }
    }

    /// Parse scope, owner_id and memory_id from a cortex:// URI.
    ///
    /// URI format: `cortex://{scope}/{owner_id}/{type_dir}/{memory_file}.md`
    ///
    /// The returned `memory_id` is the **file name stem** of the last path segment
    /// (e.g. `"pref_abc123"` from `"cortex://user/u1/preferences/pref_abc123.md"`).
    ///
    /// This matches `MemoryMetadata.id` because `IncrementalMemoryUpdater` generates
    /// the ID first and then writes the file as `{id}.md`.  The invariant is:
    ///   `MemoryMetadata.id == file_stem(MemoryMetadata.file)`
    ///
    /// If the URI cannot be parsed the caller should keep the result (conservative approach).
    fn parse_scope_owner_from_uri(uri: &str) -> Option<(MemoryScope, String, String)> {
        let stripped = uri.strip_prefix("cortex://")?;
        let parts: Vec<&str> = stripped.splitn(4, '/').collect();
        if parts.len() < 3 {
            return None;
        }

        let scope = match parts[0] {
            "user" => MemoryScope::User,
            "agent" => MemoryScope::Agent,
            "session" => MemoryScope::Session,
            "resources" => MemoryScope::Resources,
            _ => return None,
        };
        let owner_id = parts[1].to_string();
        // Use the file name stem as memory_id hint (e.g., "pref_abc123" from "preferences/pref_abc123.md")
        let memory_id = if parts.len() == 4 {
            parts[3]
                .rsplit('/')
                .next()
                .unwrap_or(parts[3])
                .trim_end_matches(".md")
                .to_string()
        } else {
            parts[2].trim_end_matches(".md").to_string()
        };

        Some((scope, owner_id, memory_id))
    }

    /// Semantic search using vector similarity
    pub async fn semantic_search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let intent = self.analyze_intent(query).await?;
        let query_text = if intent.rewritten_query.trim().is_empty() {
            query
        } else {
            &intent.rewritten_query
        };

        let query_vec = self.embedding.embed(query_text).await?;

        let mut filters = crate::types::Filters::default();
        if let Some(scope) = &options.root_uri {
            filters.uri_prefix = Some(scope.clone());
        }

        let scored = self
            .qdrant
            .as_ref()
            .search_with_threshold(
                &query_vec,
                &filters,
                options.limit.saturating_mul(3).max(options.limit),
                Some(options.threshold),
            )
            .await?;

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

        let mut results = Vec::new();
        for scored_mem in scored {
            let raw_uri = scored_mem
                .memory
                .metadata
                .uri
                .clone()
                .unwrap_or_else(|| scored_mem.memory.id.clone());
            let canonical_uri = Self::canonicalize_uri(&raw_uri);
            let mut score = scored_mem.score;

            match scored_mem.memory.metadata.layer.as_str() {
                "L2" => score += 0.08,
                "L1" => score -= 0.04,
                "L0" => score -= 0.08,
                _ => {}
            }

            results.push(SearchResult {
                uri: canonical_uri,
                score,
                snippet: Self::extract_snippet(&scored_mem.memory.content, query_text),
                content: Some(scored_mem.memory.content),
            });
        }

        Self::rerank_results(&mut results, &intent);
        Self::dedup_results(&mut results);
        results.truncate(options.limit);

        let results = self.filter_archived(results).await;
        self.emit_access_events(&results, &intent.original_query);

        Ok(results)
    }

    /// Layered semantic search - utilizes L0/L1/L2 three-layer architecture    ///
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
        let weights = weight_model::weights_for_intent(&intent.intent_type).normalize();
        info!(
            "Layer weights: L0={:.2}, L1={:.2}, L2={:.2} (intent={:?})",
            weights.l0, weights.l1, weights.l2, intent.intent_type
        );

        info!("Stage 2: Exploring L1 overview layer");
        let mut candidates = Vec::new();

        for l0_result in l0_results {
            let l0_uri = l0_result
                .memory
                .metadata
                .uri
                .clone()
                .unwrap_or_else(|| l0_result.memory.id.clone());

            let (dir_uri, _is_timeline) = Self::extract_directory_from_l0_uri(&l0_uri);
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
                candidates.push((dir_uri, l0_result.score, l1_score));
            }
        }

        info!("Found {} candidates after L1 stage", candidates.len());
        info!("Stage 3: Searching L2 detail layer");
        let mut final_results = Vec::new();

        for (dir_uri, l0_score, l1_score) in candidates {
            let mut stage3_targets = self.collect_stage3_targets(&dir_uri).await;
            if stage3_targets.is_empty() {
                stage3_targets.push(dir_uri.clone());
            }

            for target_uri in stage3_targets {
                let l2_id = uri_to_vector_id(&target_uri, ContextLayer::L2Detail);
                if let Ok(Some(l2_memory)) = self.qdrant.get(&l2_id).await {
                    let l2_score = Self::cosine_similarity(query_vec, &l2_memory.embedding);
                    let combined_score =
                        l0_score * weights.l0 + l1_score * weights.l1 + l2_score * weights.l2;

                    if combined_score >= options.threshold {
                        final_results.push(SearchResult {
                            uri: Self::canonicalize_uri(&target_uri),
                            score: combined_score,
                            snippet: Self::extract_snippet(&l2_memory.content, &intent.rewritten_query),
                            content: Some(l2_memory.content),
                        });
                    }
                } else {
                    let combined_score = l0_score * 0.4 + l1_score * 0.6;
                    if combined_score >= options.threshold {
                        if let Ok(content) = self.filesystem.read(&target_uri).await {
                            final_results.push(SearchResult {
                                uri: Self::canonicalize_uri(&target_uri),
                                score: combined_score,
                                snippet: Self::extract_snippet(&content, &intent.rewritten_query),
                                content: Some(content),
                            });
                        }
                    }
                }
            }
        }

        Self::rerank_results(&mut final_results, intent);
        Self::dedup_results(&mut final_results);
        final_results.truncate(options.limit);

        let final_results = self.filter_archived(final_results).await;

        info!(
            "Layered search completed: {} final results",
            final_results.len()
        );

        self.emit_access_events(&final_results, &intent.original_query);

        Ok(final_results)
    }

    /// 统一意图分析（优先使用 LLM 单次调用，LLM 不可用时使用最小 fallback）
    async fn analyze_intent(&self, query: &str) -> Result<EnhancedQueryIntent> {
        if self.enable_intent_analysis {
            if let Some(llm) = &self.llm_client {
                match self.analyze_intent_with_llm(llm.as_ref(), query).await {
                    Ok(intent) => return Ok(intent),
                    Err(e) => warn!("LLM intent analysis failed, using fallback: {}", e),
                }
            }
        } else {
            debug!("Intent analysis disabled, using heuristic fallback directly");
        }

        debug!("Using heuristic fallback intent analysis");
        Ok(Self::fallback_intent(query))
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
            // 实体查询：L0 摘要可能丢失实体细节，用更低阈值确保覆盖
            // LoCoMo Cat 1 (factual) 事实类也需要更低阈值，避免遗漏
            QueryIntentType::EntityLookup => {
                info!("EntityLookup: using lowered L0 threshold 0.28");
                0.28
            }
            QueryIntentType::Factual => {
                info!("Factual query: threshold 0.32");
                0.32
            }
            QueryIntentType::Temporal => {
                info!("Temporal query: threshold 0.38");
                0.38
            }
            QueryIntentType::Search | QueryIntentType::Relational => {
                info!("Search/Relational query: threshold 0.35");
                0.35
            }
            QueryIntentType::General => {
                info!("General query: default threshold 0.45");
                0.45
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

    async fn collect_stage3_targets(&self, dir_uri: &str) -> Vec<String> {
        let mut stack = vec![dir_uri.to_string()];
        let mut visited = std::collections::HashSet::new();
        let mut targets = Vec::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            let Ok(entries) = self.filesystem.list(&current).await else {
                continue;
            };

            for entry in entries {
                if entry.is_directory {
                    if !entry.name.starts_with('.') {
                        stack.push(entry.uri);
                    }
                    continue;
                }

                if entry.name.starts_with('.') || !entry.name.ends_with(".md") {
                    continue;
                }

                targets.push(entry.uri);
            }
        }

        targets
    }

    fn fallback_intent(query: &str) -> EnhancedQueryIntent {
        let query_lower = query.to_lowercase();
        let keywords = Self::fallback_keywords(query);
        let entities = Self::fallback_entities(query);
        let intent_type = if Self::contains_any(
            &query_lower,
            &["when", "date", "time", "before", "after", "昨天", "什么时候", "哪天"],
        ) {
            QueryIntentType::Temporal
        } else if Self::contains_any(
            &query_lower,
            &[
                "relationship",
                "friend",
                "partner",
                "family",
                "support",
                "married",
                "with",
                "between",
                "关系",
                "支持",
            ],
        ) {
            QueryIntentType::Relational
        } else if Self::contains_any(
            &query_lower,
            &[
                "list", "find", "search", "show", "summarize",
                "activities", "hobbies", "hobby", "partake", "sports", "interests",
                "哪些", "列出", "查找",
            ],
        ) {
            QueryIntentType::Search
        } else if Self::contains_any(
            &query_lower,
            &["who is", "what is", "identity", "background", "profile", "是谁", "身份"],
        ) {
            QueryIntentType::EntityLookup
        } else {
            QueryIntentType::Factual
        };

        let rewritten_query = Self::rewrite_query_for_intent(query, &intent_type, &keywords, &entities);

        EnhancedQueryIntent {
            original_query: query.to_string(),
            rewritten_query,
            keywords,
            entities,
            intent_type,
            time_constraint: None,
        }
    }

    fn fallback_keywords(query: &str) -> Vec<String> {
        let mut keywords: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric() && c != '+' && c != '#')
            .filter(|s| s.chars().count() > 2)
            .map(|s| s.to_lowercase())
            .filter(|s| {
                !matches!(
                    s.as_str(),
                    "what"
                        | "when"
                        | "where"
                        | "which"
                        | "who"
                        | "tell"
                        | "about"
                        | "does"
                        | "did"
                        | "with"
                        | "from"
                        | "that"
                        | "this"
                )
            })
            .collect();

        keywords.sort();
        keywords.dedup();
        keywords
    }

    fn fallback_entities(query: &str) -> Vec<String> {
        let mut entities = Vec::new();
        for token in query.split_whitespace() {
            let cleaned = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
            if cleaned.chars().count() < 2 {
                continue;
            }
            if cleaned.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                entities.push(cleaned.to_string());
            }
        }
        entities.sort();
        entities.dedup();
        entities
    }

    fn rewrite_query_for_intent(
        query: &str,
        intent_type: &QueryIntentType,
        keywords: &[String],
        entities: &[String],
    ) -> String {
        let mut terms: Vec<String> = Vec::new();
        terms.push(query.to_string());
        terms.extend(entities.iter().cloned());
        terms.extend(keywords.iter().take(6).cloned());

        match intent_type {
            QueryIntentType::EntityLookup => terms.extend(
                ["identity", "background", "profile", "personal info"]
                    .into_iter()
                    .map(str::to_string),
            ),
            QueryIntentType::Factual => terms.extend(
                ["fact", "details", "context"]
                    .into_iter()
                    .map(str::to_string),
            ),
            QueryIntentType::Temporal => terms.extend(
                ["timeline", "event", "date", "time"]
                    .into_iter()
                    .map(str::to_string),
            ),
            QueryIntentType::Relational => terms.extend(
                ["relationship", "friend", "support", "connection"]
                    .into_iter()
                    .map(str::to_string),
            ),
            QueryIntentType::Search => terms.extend(
                ["search", "relevant", "memory"]
                    .into_iter()
                    .map(str::to_string),
            ),
            QueryIntentType::General => {}
        }

        terms.sort();
        terms.dedup();
        terms.join(" ").chars().take(200).collect()
    }

    fn contains_any(text: &str, needles: &[&str]) -> bool {
        needles.iter().any(|needle| text.contains(needle))
    }

    fn canonicalize_uri(uri: &str) -> String {
        if let Some(stripped) = uri.strip_suffix("/.abstract.md") {
            stripped.to_string()
        } else if let Some(stripped) = uri.strip_suffix("/.overview.md") {
            stripped.to_string()
        } else {
            uri.to_string()
        }
    }

    fn is_summary_uri(uri: &str) -> bool {
        uri.ends_with("/.abstract.md") || uri.ends_with("/.overview.md")
    }

    fn is_leaf_uri(uri: &str) -> bool {
        uri.ends_with(".md") && !Self::is_summary_uri(uri)
    }

    fn collection_summary_penalty(path_lower: &str) -> f32 {
        if path_lower.ends_with("/entities")
            || path_lower.ends_with("/events")
            || path_lower.ends_with("/goals")
            || path_lower.ends_with("/relationships")
        {
            -0.14
        } else if path_lower.contains("/entities/") {
            -0.08
        } else {
            0.0
        }
    }

    fn intent_path_bonus(intent_type: &QueryIntentType, path_lower: &str) -> f32 {
        match intent_type {
            QueryIntentType::EntityLookup => {
                if path_lower.contains("/personal_info/") {
                    0.30
                } else if path_lower.contains("/events/") {
                    0.12
                } else if path_lower.contains("/entities/") {
                    -0.14
                } else if path_lower.contains("/preferences/") {
                    -0.20
                } else if path_lower.contains("/relationships/") {
                    -0.26
                } else if path_lower.contains("/timeline/") {
                    -0.06
                } else {
                    0.0
                }
            }
            QueryIntentType::Factual => {
                if path_lower.contains("/events/") || path_lower.contains("/personal_info/") {
                    0.08
                } else if path_lower.contains("/goals/") || path_lower.contains("/preferences/") {
                    0.03
                } else {
                    0.0
                }
            }
            QueryIntentType::Temporal => {
                if path_lower.contains("/events/") || path_lower.contains("/timeline/") {
                    0.06
                } else if path_lower.contains("/goals/") || path_lower.contains("/preferences/") {
                    -0.04
                } else {
                    0.0
                }
            }
            QueryIntentType::Relational => {
                if path_lower.contains("/relationships/") {
                    0.16
                } else if path_lower.contains("/personal_info/") {
                    0.04
                } else if path_lower.contains("/entities/") {
                    -0.10
                } else if path_lower.contains("/preferences/") {
                    -0.12
                } else if path_lower.contains("/goals/") {
                    -0.08
                } else {
                    0.0
                }
            }
            QueryIntentType::Search | QueryIntentType::General => 0.0,
        }
    }

    fn rerank_results(results: &mut Vec<SearchResult>, intent: &EnhancedQueryIntent) {
        for result in results.iter_mut() {
            let uri_lower = result.uri.to_lowercase();
            let snippet_lower = result.snippet.to_lowercase();
            let keyword_hits = intent
                .keywords
                .iter()
                .filter(|keyword| snippet_lower.contains(keyword.as_str()))
                .count() as f32;
            let entity_hits = intent
                .entities
                .iter()
                .filter(|entity| snippet_lower.contains(&entity.to_lowercase()))
                .count() as f32;

            let mut bonus = if Self::is_leaf_uri(&result.uri) { 0.12 } else { -0.12 };
            if Self::is_summary_uri(&result.uri) {
                bonus -= 0.12;
            }
            bonus += keyword_hits.min(4.0) * 0.03;
            bonus += entity_hits.min(3.0) * 0.05;
            bonus += Self::collection_summary_penalty(&uri_lower);
            bonus += Self::intent_path_bonus(&intent.intent_type, &uri_lower);

            match intent.intent_type {
                QueryIntentType::Temporal => {
                    if snippet_lower.contains("date")
                        || snippet_lower.contains("time")
                        || snippet_lower.contains("yesterday")
                        || snippet_lower.contains("last ")
                    {
                        bonus += 0.10;
                    }
                }
                QueryIntentType::Relational => {
                    if uri_lower.contains("/relationships/") {
                        bonus += 0.06;
                    }
                }
                QueryIntentType::EntityLookup | QueryIntentType::Factual => {}
                QueryIntentType::Search | QueryIntentType::General => {}
            }

            result.score += bonus;
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    fn dedup_results(results: &mut Vec<SearchResult>) {
        let mut merged: std::collections::HashMap<String, SearchResult> =
            std::collections::HashMap::new();

        for mut result in std::mem::take(results) {
            result.uri = Self::canonicalize_uri(&result.uri);
            match merged.get_mut(&result.uri) {
                Some(existing) => {
                    if result.score > existing.score {
                        *existing = result;
                    } else if existing.content.is_none() {
                        existing.content = result.content;
                    }
                }
                None => {
                    merged.insert(result.uri.clone(), result);
                }
            }
        }

        *results = merged.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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
