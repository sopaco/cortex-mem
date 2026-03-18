use axum::{Json, extract::State};
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    models::{ApiResponse, SearchRequest, SearchResultResponse},
    state::AppState,
};
use crate::handlers::filesystem::load_layers_for_uri;

/// Search endpoint using layered vector search (L0/L1/L2)
pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResultResponse>>>> {
    let limit = req.limit.unwrap_or(10);
    let min_score = req.min_score.unwrap_or(0.6);
    let return_layers = req.return_layers.clone();

    let results = search_layered(
        &state,
        &req.query,
        req.thread.as_deref(),
        limit,
        min_score,
        &return_layers,
    ).await?;

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
    return_layers: &[String],
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

    // Get tenant root for layer loading
    let tenant_root = state.current_tenant_root.read().await.clone();
    let base_dir = if let Some(ref root) = tenant_root {
        root.clone()
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        state.data_dir.clone()
    };

    // Convert to response format with requested layers
    let mut results = Vec::new();
    for result in search_results {
        let snippet = if result.snippet.len() > 200 {
            format!("{}...", &result.snippet.chars().take(200).collect::<String>())
        } else {
            result.snippet
        };

        // Use shared helper to load layers
        let (overview, content, layers) = load_layers_for_uri(&base_dir, &result.uri, return_layers).await;

        results.push(SearchResultResponse {
            uri: result.uri,
            score: result.score,
            snippet,
            overview,
            content,
            source: "layered_vector".to_string(),
            layers,
        });
    }

    Ok(results)
}