use axum::Router;
use crate::state::AppState;

mod filesystem;
mod sessions;
mod search;
mod automation;
mod tenants;

pub fn api_routes() -> Router<std::sync::Arc<AppState>> {
    Router::new()
        // Filesystem routes
        .nest("/filesystem", filesystem::routes())
        // Session routes
        .nest("/sessions", sessions::routes())
        // Search routes
        .nest("/search", search::routes())
        // Automation routes
        .nest("/automation", automation::routes())
        // Tenant routes
        .nest("/tenants", tenants::routes())
}
