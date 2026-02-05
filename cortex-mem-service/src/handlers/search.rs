use axum::{
    Router,
    routing::post,
    extract::State,
    Json,
};
use std::sync::Arc;

use crate::{
    error::{Result, AppError},
    models::{ApiResponse, SearchRequest, SearchResultResponse, SearchMode},
    state::AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(search))
}

/// Search endpoint with multiple modes
pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResultResponse>>>> {
    
    
    let limit = req.limit.unwrap_or(10);
    let min_score = req.min_score.unwrap_or(0.0);
    
    let results = match req.mode {
        SearchMode::Filesystem => {
            search_filesystem(&state, &req.query, req.thread.as_deref(), limit, min_score).await?
        }
        
        #[cfg(feature = "vector-search")]
        SearchMode::Vector => {
            search_vector(&state, &req.query, req.thread.as_deref(), limit, min_score).await?
        }
        
        #[cfg(feature = "vector-search")]
        SearchMode::Hybrid => {
            search_hybrid(&state, &req.query, req.thread.as_deref(), limit, min_score).await?
        }
    };

    Ok(Json(ApiResponse::success(results)))
}

/// File system search implementation using RetrievalEngine
async fn search_filesystem(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<Vec<SearchResultResponse>> {
    use cortex_mem_core::{FilesystemOperations, LayerManager, RetrievalEngine, RetrievalOptions};
    
    let search_uri = if let Some(thread) = thread {
        format!("cortex://threads/{}", thread)
    } else {
        "cortex://threads".to_string()
    };

    // Use RetrievalEngine for recursive search with L0/L1/L2 layers
    let layer_manager = Arc::new(LayerManager::new(state.filesystem.clone()));
    let engine = RetrievalEngine::new(state.filesystem.clone(), layer_manager);
    
    let options = RetrievalOptions {
        top_k: limit,
        min_score,
        load_details: true,
        max_candidates: limit * 2,
    };
    
    let result = engine.search(query, &search_uri, options).await
        .map_err(|e| AppError::Internal(format!("Search failed: {}", e)))?;
    
    // Convert to response format
    let results: Vec<SearchResultResponse> = result.results
        .into_iter()
        .map(|r| SearchResultResponse {
            uri: r.uri,
            score: r.score,
            snippet: r.snippet,
            content: None, // Content not loaded by default
            source: "filesystem".to_string(),
        })
        .collect();
    
    Ok(results)
}

/// Vector search implementation (feature-gated)
#[cfg(feature = "vector-search")]
async fn search_vector(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<Vec<SearchResultResponse>> {
    use cortex_mem_core::{Filters, VectorStore};
    
    // Check if vector store is available
    let vector_store = state.vector_store.as_ref()
        .ok_or_else(|| AppError::BadRequest(
            "Vector search not available. Qdrant not configured.".to_string()
        ))?;
    
    // Check if embedding client is available
    let embedding_client = state.embedding_client.as_ref()
        .ok_or_else(|| AppError::BadRequest(
            "Vector search not available. Embedding service not configured.".to_string()
        ))?;
    
    // Generate query embedding
    let query_embedding = embedding_client.embed(query).await
        .map_err(|e| AppError::Internal(format!("Failed to generate embedding: {}", e)))?;
    
    // Build filters
    let mut filters = Filters::default();
    if let Some(thread_id) = thread {
        filters.run_id = Some(thread_id.to_string());
    }
    
    // Search in vector store
    let scored_memories = vector_store
        .search_with_threshold(&query_embedding, &filters, limit, Some(min_score))
        .await
        .map_err(|e| AppError::Internal(format!("Vector search failed: {}", e)))?;
    
    // Convert to response format
    let results: Vec<SearchResultResponse> = scored_memories
        .into_iter()
        .map(|scored| {
            let snippet = if scored.memory.content.len() > 200 {
                format!("{}...", &scored.memory.content[..200])
            } else {
                scored.memory.content.clone()
            };
            
            SearchResultResponse {
                uri: scored.memory.id.clone(),
                score: scored.score,
                snippet,
                content: Some(scored.memory.content),
                source: "vector".to_string(),
            }
        })
        .collect();
    
    Ok(results)
}

/// Hybrid search implementation (feature-gated)
#[cfg(feature = "vector-search")]
async fn search_hybrid(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<Vec<SearchResultResponse>> {
    // Perform both vector and filesystem search
    let vector_results = match search_vector(state, query, thread, limit, min_score).await {
        Ok(results) => results,
        Err(e) => {
            tracing::warn!("Vector search failed in hybrid mode: {:?}, using filesystem only", e);
            vec![]
        }
    };
    
    let filesystem_results = search_filesystem(state, query, thread, limit, min_score).await?;
    
    // Merge and deduplicate results
    let mut all_results: Vec<SearchResultResponse> = Vec::new();
    let mut seen_uris = std::collections::HashSet::new();
    
    // Add vector results first (higher priority)
    for mut result in vector_results {
        if seen_uris.insert(result.uri.clone()) {
            result.source = "hybrid_vector".to_string();
            all_results.push(result);
        }
    }
    
    // Add filesystem results
    for mut result in filesystem_results {
        if seen_uris.insert(result.uri.clone()) {
            result.source = "hybrid_filesystem".to_string();
            all_results.push(result);
        }
    }
    
    // Sort by score and limit
    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    all_results.truncate(limit);
    
    Ok(all_results)
}

/// Calculate text-based relevance score
fn calculate_text_score(content: &str, query: &str) -> f32 {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();
    
    // Count occurrences
    let count = content_lower.matches(&query_lower).count();
    
    // Simple scoring: more matches = higher score
    // Cap at 1.0
    (count as f32 * 0.1).min(1.0)
}

/// Extract snippet around query match
fn extract_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower_content = content.to_lowercase();
    let lower_query = query.to_lowercase();
    
    if let Some(pos) = lower_content.find(&lower_query) {
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + query.len() + max_len / 2).min(content.len());
        let snippet = &content[start..end];
        
        if start > 0 {
            format!("...{}", snippet)
        } else {
            snippet.to_string()
        }
    } else {
        content.chars().take(max_len).collect()
    }
}
