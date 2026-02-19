use axum::{
    Router,
    routing::{get, post},
};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(crate::handlers::sessions::list_sessions).post(crate::handlers::sessions::create_session))
        .route("/:thread_id/messages", post(crate::handlers::sessions::add_message))
        .route("/:thread_id/close", post(crate::handlers::sessions::close_session))
}
