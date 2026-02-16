use axum::{extract::State, Json};
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    models::{ApiResponse, SearchRequest, SearchResultResponse},
    state::AppState,
};

/// Search endpoint using vector search
pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResultResponse>>>> {
    let limit = req.limit.unwrap_or(10);
    let min_score = req.min_score.unwrap_or(0.0);

    let results = search_vector(&state, &req.query, req.thread.as_deref(), limit, min_score).await?;

    Ok(Json(ApiResponse::success(results)))
}

/// Vector search implementation
async fn search_vector(
    state: &AppState,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<Vec<SearchResultResponse>> {
    use cortex_mem_core::{Filters, VectorStore};

    // Check if vector store is available
    let vector_store = state.vector_store.as_ref().ok_or_else(|| {
        AppError::BadRequest("Vector search not available. Qdrant not configured.".to_string())
    })?;

    // Check if embedding client is available
    let embedding_client = state.embedding_client.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Vector search not available. Embedding service not configured.".to_string(),
        )
    })?;

    // Generate query embedding
    let query_embedding = embedding_client
        .embed(query)
        .await
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