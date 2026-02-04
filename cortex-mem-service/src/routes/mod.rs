use axum::{
    Router,
    routing::{get, post},
};
use crate::state::AppState;

mod filesystem;
mod sessions;
mod search;
mod automation;

pub fn api_routes() -> Router<std::sync::Arc<AppState>> {
    Router::new()
        // Filesystem routes
        .nest("/filesystem", filesystem::routes())
        // Session routes
        .nest("/sessions", sessions::routes())
        // Search routes
        .nest("/search", search::routes())
        // Automation routes
        .route("/automation/extract/:thread_id", post(crate::handlers::automation::trigger_extraction))
}
