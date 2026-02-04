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
    use cortex_mem_core::FilesystemOperations;
    
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

/// File system search implementation
async fn search_filesystem(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<Vec<SearchResultResponse>> {
    use cortex_mem_core::FilesystemOperations;
    
    let search_uri = if let Some(thread) = thread {
        format!("cortex://threads/{}/timeline", thread)
    } else {
        "cortex://".to_string()
    };

    // List files recursively
    let entries = state.filesystem.list(&search_uri).await?;
    
    let mut results = Vec::new();
    
    for entry in entries {
        if results.len() >= limit {
            break;
        }
        
        if !entry.is_directory && entry.uri.ends_with(".md") {
            // Read file content
            if let Ok(content) = state.filesystem.read(&entry.uri).await {
                // Simple text search
                if content.to_lowercase().contains(&query.to_lowercase()) {
                    // Calculate simple score
                    let score = calculate_text_score(&content, query);
                    
                    if score >= min_score {
                        // Extract snippet
                        let snippet = extract_snippet(&content, query, 200);
                        
                        results.push(SearchResultResponse {
                            uri: entry.uri.clone(),
                            score,
                            snippet,
                            content: Some(content),
                            source: "filesystem".to_string(),
                        });
                    }
                }
            }
        }
    }
    
    // Sort by score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    
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
    use cortex_mem_core::{Filters, FilesystemOperations};
    
    let vector_store = state.vector_store.as_ref()
        .ok_or_else(|| AppError::BadRequest(
            "Vector search not available. Qdrant not configured.".to_string()
        ))?;
    
    // Build filters
    let mut filters = Filters::default();
    // Note: thread filtering would need custom metadata in Qdrant
    
    // For now, simplified implementation: use filesystem search
    // Real vector search would need embedding generation
    tracing::warn!("Vector search requested but embedding not implemented yet, falling back to filesystem");
    
    search_filesystem(state, query, thread, limit, min_score).await
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
    // For now, just use filesystem search
    // Real implementation would combine vector and filesystem results
    tracing::warn!("Hybrid search requested but embedding not implemented yet, using filesystem only");
    
    search_filesystem(state, query, thread, limit, min_score).await
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
