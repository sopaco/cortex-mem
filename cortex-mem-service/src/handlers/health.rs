use axum::{
    extract::State,
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{error::Result, state::AppState};

/// Health check endpoint
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>> {
    let has_llm = state.llm_client.is_some();
    
    Ok(Json(json!({
        "status": "healthy",
        "service": "cortex-mem-service",
        "version": env!("CARGO_PKG_VERSION"),
        "llm_available": has_llm,
        "timestamp": chrono::Utc::now(),
    })))
}
