use axum::{extract::State, Json};
use std::sync::Arc;

use crate::{
    error::Result,
    models::ApiResponse,
    state::AppState,
};

/// List all available tenants
pub async fn list_tenants(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<String>>>> {
    let tenants = state.list_tenants().await;
    Ok(Json(ApiResponse::success(tenants)))
}

/// Switch to a different tenant
pub async fn switch_tenant(
    State(state): State<Arc<AppState>>,
    Json(tenant_id): Json<TenantSwitchRequest>,
) -> Result<Json<ApiResponse<String>>> {
    state.switch_tenant(&tenant_id.tenant_id).await?;
    Ok(Json(ApiResponse::success(tenant_id.tenant_id)))
}

#[derive(serde::Deserialize)]
pub struct TenantSwitchRequest {
    pub tenant_id: String,
}
