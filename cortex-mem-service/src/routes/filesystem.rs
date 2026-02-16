use axum::{
    Router,
    routing::get,
};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/list", get(crate::handlers::filesystem::list_directory))
        .route("/read/*path", get(crate::handlers::filesystem::read_file))
}
