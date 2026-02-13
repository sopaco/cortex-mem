// Search Tools - OpenViking style intelligent search

use crate::{Result, types::*, MemoryOperations};
use cortex_mem_core::{RetrievalEngine, RetrievalOptions, ContextLayer, FilesystemOperations};

/// 查询类型枚举
#[derive(Debug, Clone, Copy)]
enum QueryType {
    Exact,      // 精确查询：日期、ID、专有名词
    Semantic,   // 语义查询：概念、主题、问题
    Mixed,      // 混合查询
}

impl MemoryOperations {
    /// Intelligent search with multiple engines and tiered results
    pub async fn search(&self, args: SearchArgs) -> Result<SearchResponse> {
        // Normalize scope before searching
        let normalized_args = SearchArgs {
            scope: args.scope.as_deref().map(|s| Self::normalize_scope(Some(s))),
            ..args
        };
        
        // 1. Choose retrieval engine based on args.engine
        let raw_results = match normalized_args.engine.as_deref() {
            #[cfg(feature = "vector-search")]
            Some("vector") => self.vector_search(&normalized_args).await?,
            #[cfg(feature = "vector-search")]
            Some("hybrid") => self.hybrid_search(&normalized_args).await?,
            _ => self.keyword_search(&normalized_args).await?,
        };
        
        // 2. Enrich results with requested layers
        let enriched_results = self.enrich_results(
            raw_results,
            &normalized_args.return_layers.clone().unwrap_or(vec!["L0".to_string()])
        ).await?;
        
        let total = enriched_results.len();
        
        Ok(SearchResponse {
            query: normalized_args.query.clone(),
            results: enriched_results,
            total,
            engine_used: normalized_args.engine.unwrap_or("keyword".to_string()),
        })
    }
    
    /// Simple find - quick search returning only L0 abstracts
    pub async fn find(&self, args: FindArgs) -> Result<FindResponse> {
        // Normalize scope - if invalid, default to threads
        let normalized_scope = Self::normalize_scope(args.scope.as_deref());
        
        let search_args = SearchArgs {
            query: args.query.clone(),
            engine: Some("keyword".to_string()),
            recursive: Some(true),
            return_layers: Some(vec!["L0".to_string()]),
            scope: Some(normalized_scope),
            limit: args.limit,
        };
        
        let search_response = self.search(search_args).await?;
        
        let results = search_response.results.into_iter().map(|r| FindResult {
            uri: r.uri,
            abstract_text: r.abstract_text.unwrap_or_default(),
        }).collect();
        
        Ok(FindResponse {
            query: args.query,
            results,
            total: search_response.total,
        })
    }
    
    /// Normalize scope parameter to ensure it's a valid cortex URI
    fn normalize_scope(scope: Option<&str>) -> String {
        match scope {
            None => "cortex://threads".to_string(),
            Some(s) => {
                // If already a valid cortex URI with known dimension, use as-is
                if s.starts_with("cortex://") {
                    let dimension = s.strip_prefix("cortex://")
                        .and_then(|rest| rest.split('/').next())
                        .unwrap_or("");
                    
                    match dimension {
                        "agents" | "users" | "threads" | "global" => s.to_string(),
                        // Invalid dimension, map common aliases to valid ones
                        "system" | "assistant" | "bot" => "cortex://threads".to_string(),
                        "user" => "cortex://users".to_string(),
                        "agent" => "cortex://agents".to_string(),
                        // Unknown dimension, default to threads
                        _ => "cortex://threads".to_string(),
                    }
                } else {
                    // Not a cortex URI, assume it's a relative path under threads
                    format!("cortex://threads/{}", s.trim_start_matches('/'))
                }
            }
        }
    }
    
    // ==================== Internal Methods ====================
    
    /// 分析查询类型
    fn analyze_query_type(query: &str) -> QueryType {
        let query_lower = query.to_lowercase();
        
        // 规则1: 包含日期格式 -> Exact
        // 匹配格式: YYYY-MM-DD, YYYY/MM/DD, YYYY.MM.DD
        if query.chars().filter(|c| c.is_numeric()).count() >= 6 {
            // 简单检测：如果包含至少6个数字，可能是日期
            if query.contains('-') || query.contains('/') || query.contains('.') {
                return QueryType::Exact;
            }
        }
        
        // 规则2: 包含引号 -> Exact
        if query.contains('"') || query.contains('\'') {
            return QueryType::Exact;
        }
        
        // 规则3: 包含问号/疑问词 -> Semantic
        let question_words = ["什么", "如何", "为什么", "怎么", "哪里", "谁", 
                              "what", "how", "why", "where", "who", "when"];
        if query.contains('?') || question_words.iter().any(|w| query_lower.contains(w)) {
            return QueryType::Semantic;
        }
        
        // 规则4: 包含"关于"/"讨论"等主题词 -> Semantic
        let topic_words = ["关于", "讨论", "决定", "计划", "想法", 
                          "about", "discussion", "decision", "plan", "idea"];
        if topic_words.iter().any(|w| query_lower.contains(w)) {
            return QueryType::Semantic;
        }
        
        // 规则5: 查询很短(≤5个字符)且无空格 -> Exact (可能是ID或关键词)
        if query.len() <= 5 && !query.contains(' ') {
            return QueryType::Exact;
        }
        
        // 默认: Mixed
        QueryType::Mixed
    }
    
    /// Keyword search using RetrievalEngine
    async fn keyword_search(&self, args: &SearchArgs) -> Result<Vec<RawSearchResult>> {
        let engine = RetrievalEngine::new(
            self.filesystem.clone(),
            self.layer_manager.clone()
        );
        
        let options = RetrievalOptions {
            top_k: args.limit.unwrap_or(10),
            ..Default::default()
        };
        
        let scope = args.scope.as_deref().unwrap_or("cortex://threads");
        let result = engine.search(&args.query, scope, options).await?;
        
        Ok(result.results.into_iter().map(|r| RawSearchResult {
            uri: r.uri,
            score: r.score,
        }).collect())
    }
    
    /// Vector search using VectorSearchEngine
    #[cfg(feature = "vector-search")]
    async fn vector_search(&self, args: &SearchArgs) -> Result<Vec<RawSearchResult>> {
        use cortex_mem_core::search::SearchResult as CoreSearchResult;
        use crate::ToolsError;
        
        let engine = self.vector_engine.as_ref()
            .ok_or(ToolsError::Custom("Vector search not enabled".to_string()))?;
        
        let search_options = cortex_mem_core::search::SearchOptions {
            limit: args.limit.unwrap_or(10),
            threshold: 0.5,
            root_uri: args.scope.clone(),
            recursive: args.recursive.unwrap_or(true),
        };
        
        let results: Vec<CoreSearchResult> = if args.recursive.unwrap_or(true) {
            engine.recursive_search(
                &args.query,
                args.scope.as_deref().unwrap_or("cortex://threads"),
                &search_options
            ).await?
        } else {
            engine.semantic_search(&args.query, &search_options).await?
        };
        
        Ok(results.into_iter().map(|r| RawSearchResult {
            uri: r.uri,
            score: r.score,
        }).collect())
    }
    
    /// Hybrid search combining keyword and vector search (with adaptive weighting)
    #[cfg(feature = "vector-search")]
    async fn hybrid_search(&self, args: &SearchArgs) -> Result<Vec<RawSearchResult>> {
        // Get results from both engines
        let keyword_results = self.keyword_search(args).await?;
        let vector_results = self.vector_search(args).await?;
        
        // Analyze query type to determine weights
        let query_type = Self::analyze_query_type(&args.query);
        let (keyword_weight, vector_weight) = match query_type {
            QueryType::Exact => (0.8, 0.2),    // 精确查询优先关键词匹配
            QueryType::Semantic => (0.2, 0.8), // 语义查询优先向量相似度
            QueryType::Mixed => (0.5, 0.5),    // 混合查询平衡权重
        };
        
        // Merge and deduplicate with adaptive weights
        let mut combined: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        
        for result in keyword_results {
            combined.insert(result.uri.clone(), result.score * keyword_weight);
        }
        
        for result in vector_results {
            combined.entry(result.uri.clone())
                .and_modify(|score| *score += result.score * vector_weight)
                .or_insert(result.score * vector_weight);
        }
        
        let mut results: Vec<RawSearchResult> = combined.into_iter()
            .map(|(uri, score)| RawSearchResult { uri, score })
            .collect();
        
        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // Limit results
        results.truncate(args.limit.unwrap_or(10));
        
        Ok(results)
    }
    
    /// Enrich raw results with requested layers
    async fn enrich_results(
        &self,
        raw_results: Vec<RawSearchResult>,
        return_layers: &[String],
    ) -> Result<Vec<SearchResult>> {
        let mut enriched = Vec::new();
        
        for raw in raw_results {
            let mut result = SearchResult {
                uri: raw.uri.clone(),
                score: raw.score,
                abstract_text: None,
                overview_text: None,
                content: None,
            };
            
            // Load layers as requested
            if return_layers.contains(&"L0".to_string()) {
                result.abstract_text = self.layer_manager
                    .load(&raw.uri, ContextLayer::L0Abstract)
                    .await
                    .ok();
            }
            if return_layers.contains(&"L1".to_string()) {
                result.overview_text = self.layer_manager
                    .load(&raw.uri, ContextLayer::L1Overview)
                    .await
                    .ok();
            }
            if return_layers.contains(&"L2".to_string()) {
                result.content = self.filesystem.read(&raw.uri).await.ok();
            }
            
            enriched.push(result);
        }
        
        Ok(enriched)
    }
}
