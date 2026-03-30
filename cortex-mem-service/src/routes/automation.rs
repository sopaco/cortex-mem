use axum::{Router, routing::post};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/extract/:thread_id", post(crate::handlers::automation::trigger_extraction))
        .route("/reindex", post(crate::handlers::automation::trigger_reindex))
}
