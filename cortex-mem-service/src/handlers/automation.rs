use axum::{
    Router,
    routing::post,
    extract::{Path, State},
    Json,
};
use std::sync::Arc;
use serde::Deserialize;

use crate::{
    error::{Result, AppError},
    models::ApiResponse,
    state::AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/extract/:thread_id", post(trigger_extraction))
}

#[derive(Debug, Deserialize)]
pub struct ExtractionRequest {
    #[serde(default)]
    auto_save: bool,
}

/// Trigger memory extraction for a session
pub async fn trigger_extraction(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
    Json(req): Json<ExtractionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    use cortex_mem_core::extraction::{MemoryExtractor, ExtractionConfig};

    // Check if LLM client is available
    let llm_client = state.llm_client.as_ref()
        .ok_or_else(|| AppError::BadRequest(
            "LLM client not configured. Set LLM_API_BASE_URL, LLM_API_KEY, and LLM_MODEL environment variables.".to_string()
        ))?;

    // Create extraction config
    let config = ExtractionConfig {
        extract_facts: true,
        extract_decisions: true,
        extract_entities: true,
        min_confidence: 0.5,
        max_messages_per_batch: 50,
    };

    // Create extractor
    let extractor = MemoryExtractor::new(
        state.filesystem.clone(),
        llm_client.clone(),
        config,
    );

    // Get message storage
    let message_storage = cortex_mem_core::MessageStorage::new(state.filesystem.clone());
    
    // List all message URIs for the thread
    let message_uris = message_storage.list_messages(&thread_id).await?;
    
    // Load messages
    let mut messages = Vec::new();
    for uri in message_uris {
        if let Ok(msg) = message_storage.load_message(&uri).await {
            messages.push(msg);
        }
    }

    if messages.is_empty() {
        return Err(AppError::NotFound(format!("No messages found in thread {}", thread_id)));
    }

    // Extract memories
    let extraction_result = extractor.extract_from_messages(&thread_id, &messages).await?;

    // Optionally save to user/agent memories
    if req.auto_save {
        // TODO: Save to cortex://users/{user_id}/memories/
        // TODO: Save to cortex://agents/{agent_id}/memories/
        tracing::info!("Auto-save not yet implemented");
    }

    let response = serde_json::json!({
        "thread_id": thread_id,
        "message_count": messages.len(),
        "facts_count": extraction_result.facts.len(),
        "decisions_count": extraction_result.decisions.len(),
        "entities_count": extraction_result.entities.len(),
        "facts": extraction_result.facts,
        "decisions": extraction_result.decisions,
        "entities": extraction_result.entities,
    });

    Ok(Json(ApiResponse::success(response)))
}
