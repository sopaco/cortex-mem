use axum::{
    Router,
    routing::{get, post},
};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tenants", get(crate::handlers::tenants::list_tenants))
        .route("/tenants/switch", post(crate::handlers::tenants::switch_tenant))
}
