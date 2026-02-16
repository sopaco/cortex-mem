use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    error::Result,
    models::{ApiResponse, FileEntryResponse},
    state::AppState,
};
use cortex_mem_core::FilesystemOperations;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    uri: String,
}

/// List directory contents
pub async fn list_directory(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<FileEntryResponse>>>> {
    let entries = state.filesystem.as_ref().list(&params.uri).await?;
    
    let responses: Vec<FileEntryResponse> = entries
        .into_iter()
        .map(FileEntryResponse::from)
        .collect();
    
    Ok(Json(ApiResponse::success(responses)))
}

/// Read file content
pub async fn read_file(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    let uri = if path.starts_with("cortex://") {
        path
    } else {
        format!("cortex://{}", path)
    };
    
    let content = state.filesystem.as_ref().read(&uri).await?;
    
    Ok(Json(ApiResponse::success(content)))
}
