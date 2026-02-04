use axum::{
    Router,
    routing::post,
};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(crate::handlers::sessions::create_session))
}
