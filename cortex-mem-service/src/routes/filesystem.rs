use axum::{
    Router,
    routing::get,
};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(crate::handlers::filesystem::list_directory))
}
