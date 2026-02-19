use axum::{Json, extract::State};
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    models::{ApiResponse, SearchRequest, SearchResultResponse},
    state::AppState,
};

/// Search endpoint using layered vector search (L0/L1/L2)
pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResultResponse>>>> {
    let limit = req.limit.unwrap_or(10);
    let min_score = req.min_score.unwrap_or(0.5);

    let results = search_layered(&state, &req.query, req.thread.as_deref(), limit, min_score).await?;

    Ok(Json(ApiResponse::success(results)))
}

/// Layered semantic search using L0/L1/L2 tiered retrieval
/// 
/// This implementation uses VectorSearchEngine::layered_semantic_search for
/// consistency with cortex-mem-tools search behavior.
async fn search_layered(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<Vec<SearchResultResponse>> {
    use cortex_mem_core::SearchOptions;

    // Check if vector engine is available
    let vector_engine_lock = state.vector_engine.read().await;
    let vector_engine = vector_engine_lock.as_ref().ok_or_else(|| {
        AppError::BadRequest("Vector search not available. Qdrant and Embedding service must be configured.".to_string())
    })?;

    // Build search options
    let mut options = SearchOptions {
        limit,
        threshold: min_score,
        root_uri: None,
        recursive: true,
    };

    // Set scope based on thread parameter
    if let Some(thread_id) = thread {
        options.root_uri = Some(format!("cortex://session/{}", thread_id));
    }

    // Use layered semantic search (L0 -> L1 -> L2)
    let search_results = vector_engine
        .layered_semantic_search(query, &options)
        .await
        .map_err(|e| AppError::Internal(format!("Layered search failed: {}", e)))?;

    // Convert to response format
    let results: Vec<SearchResultResponse> = search_results
        .into_iter()
        .map(|result| {
            let snippet = if result.snippet.len() > 200 {
                format!("{}...", &result.snippet.chars().take(200).collect::<String>())
            } else {
                result.snippet
            };

            SearchResultResponse {
                uri: result.uri,
                score: result.score,
                snippet,
                content: result.content,
                source: "layered_vector".to_string(),
            }
        })
        .collect();

    Ok(results)
}
