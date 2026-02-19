use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    error::{Result, AppError},
    models::{ApiResponse, FileEntryResponse},
    state::AppState,
};
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    uri: String,
}

/// List directory contents
pub async fn list_directory(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<FileEntryResponse>>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = params.uri.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        let cortex_dir = state.data_dir.join("cortex");
        let uri_path = params.uri.trim_start_matches("cortex://");
        cortex_dir.join(uri_path)
    };
    
    tracing::debug!("Listing directory: {:?}", base_path);
    
    if !base_path.exists() {
        return Ok(Json(ApiResponse::success(vec![])));
    }
    
    let mut entries = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&base_path) {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files except .abstract.md and .overview.md
            if name.starts_with('.') && name != ".abstract.md" && name != ".overview.md" {
                continue;
            }
            
            if let Ok(metadata) = entry.metadata() {
                let is_dir = metadata.is_dir();
                let size = metadata.len();
                let modified = metadata.modified()
                    .map(|t| DateTime::<Utc>::from(t))
                    .unwrap_or_else(|_| Utc::now());
                
                let entry_uri = format!("{}/{}", params.uri.trim_end_matches('/'), name);
                
                entries.push(FileEntryResponse {
                    uri: entry_uri,
                    name,
                    is_directory: is_dir,
                    size,
                    modified,
                });
            }
        }
    }
    
    Ok(Json(ApiResponse::success(entries)))
}

/// Read file content
pub async fn read_file(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = path.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        let cortex_dir = state.data_dir.join("cortex");
        let uri_path = path.trim_start_matches("cortex://");
        cortex_dir.join(uri_path)
    };
    
    tracing::debug!("Reading file: {:?}", base_path);
    
    let content = tokio::fs::read_to_string(&base_path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(content)))
}

/// Write file content
pub async fn write_file(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WriteFileRequest>,
) -> Result<Json<ApiResponse<String>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = req.path.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        let cortex_dir = state.data_dir.join("cortex");
        let uri_path = req.path.trim_start_matches("cortex://");
        cortex_dir.join(uri_path)
    };
    
    tracing::debug!("Writing file: {:?}", base_path);
    
    // Ensure parent directory exists
    if let Some(parent) = base_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    
    tokio::fs::write(&base_path, &req.content)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(base_path.to_string_lossy().to_string())))
}

/// Get directory stats (recursive)
pub async fn get_directory_stats(
    State(state): State<Arc<AppState>>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<ApiResponse<DirectoryStats>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the base path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = params.uri.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        let cortex_dir = state.data_dir.join("cortex");
        let uri_path = params.uri.trim_start_matches("cortex://");
        cortex_dir.join(uri_path)
    };
    
    tracing::debug!("Getting stats for: {:?}", base_path);
    
    let (file_count, total_size) = count_files_recursive(&base_path)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(DirectoryStats {
        file_count,
        total_size,
    })))
}

fn count_files_recursive(path: &std::path::Path) -> std::io::Result<(u64, u64)> {
    use std::fs;
    
    let mut file_count = 0u64;
    let mut total_size = 0u64;
    
    if !path.exists() {
        return Ok((0, 0));
    }
    
    if path.is_file() {
        let metadata = fs::metadata(path)?;
        return Ok((1, metadata.len()));
    }
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let entry_name = entry.file_name();
        
        // Skip hidden files/directories
        if entry_name.to_string_lossy().starts_with('.') {
            continue;
        }
        
        if entry_path.is_dir() {
            let (sub_count, sub_size) = count_files_recursive(&entry_path)?;
            file_count += sub_count;
            total_size += sub_size;
        } else {
            let metadata = fs::metadata(&entry_path)?;
            file_count += 1;
            total_size += metadata.len();
        }
    }
    
    Ok((file_count, total_size))
}

#[derive(serde::Deserialize)]
pub struct WriteFileRequest {
    path: String,
    content: String,
}

#[derive(serde::Deserialize)]
pub struct StatsQuery {
    uri: String,
}

#[derive(serde::Serialize)]
pub struct DirectoryStats {
    file_count: u64,
    total_size: u64,
}
