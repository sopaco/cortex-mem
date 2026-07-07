use axum::{
    extract::{Path, Query, State, Json},
    Json as JsonExtractor,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    error::{Result, AppError},
    models::{ApiResponse, FileEntryResponse, LsRequest, LsResponse, ExploreRequest, ExploreResponse, ExplorationPathItem, SearchResultResponse},
    state::AppState,
};
use chrono::{DateTime, Utc};

// ==================== List Directory ====================

/// List directory contents with optional recursive and abstracts
pub async fn list_directory(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LsRequest>,
) -> Result<Json<ApiResponse<LsResponse>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the base path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = params.uri.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        let uri_path = params.uri.trim_start_matches("cortex://");
        state.data_dir.join(uri_path)
    };
    
    tracing::debug!("Listing directory: {:?} (recursive={}, include_abstracts={})", 
        base_path, params.recursive, params.include_abstracts);
    
    if !base_path.exists() {
        return Ok(Json(ApiResponse::success(LsResponse {
            uri: params.uri,
            total: 0,
            entries: vec![],
        })));
    }

    let uri = params.uri.clone();
    let recursive = params.recursive;
    let include_abstracts = params.include_abstracts;
    let include_layers = params.include_layers;
    
    // Use spawn_blocking to avoid blocking the async runtime
    let entries = tokio::task::spawn_blocking(move || {
        let mut entries = Vec::new();
        list_directory_recursive(&base_path, &uri, recursive, include_abstracts, include_layers, &mut entries);
        entries
    })
    .await
    .map_err(|e| AppError::Internal(format!("Failed to list directory: {}", e)))?;
    
    let total = entries.len();
    Ok(Json(ApiResponse::success(LsResponse {
        uri: params.uri,
        total,
        entries,
    })))
}

/// Recursively list directory contents (synchronous, runs in spawn_blocking)
fn list_directory_recursive(
    base_path: &std::path::Path,
    base_uri: &str,
    recursive: bool,
    include_abstracts: bool,
    include_layers: bool,
    entries: &mut Vec<FileEntryResponse>,
) {
    if !base_path.exists() || !base_path.is_dir() {
        return;
    }
    
    if let Ok(dir) = std::fs::read_dir(base_path) {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files unless include_layers is true and it's a layer file
            if name.starts_with('.') {
                if !include_layers {
                    continue;
                }
                // Only show .abstract.md (L0) and .overview.md (L1) when include_layers is true
                if name != ".abstract.md" && name != ".overview.md" {
                    continue;
                }
            }
            
            if let Ok(metadata) = entry.metadata() {
                let is_dir = metadata.is_dir();
                let size = metadata.len();
                let modified = metadata.modified()
                    .map(|t| DateTime::<Utc>::from(t))
                    .unwrap_or_else(|_| Utc::now());
                
                let entry_uri = format!("{}/{}", base_uri.trim_end_matches('/'), name);
                
                // Load abstract if requested and this is a file
                // All files in the same directory share the directory-level abstract
                let abstract_text = if include_abstracts && !is_dir {
                    let abstract_path = base_path.join(".abstract.md");
                    std::fs::read_to_string(&abstract_path).ok()
                } else {
                    None
                };
                
                entries.push(FileEntryResponse {
                    uri: entry_uri.clone(),
                    name: name.clone(),
                    is_directory: is_dir,
                    size,
                    modified,
                    abstract_text,
                });
                
                // Recurse into subdirectories if requested
                if recursive && is_dir {
                    let sub_path = base_path.join(&name);
                    list_directory_recursive(&sub_path, &entry_uri, true, include_abstracts, include_layers, entries);
                }
            }
        }
    }
}

// ==================== Read/Write File ====================

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
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        let uri_path = path.trim_start_matches("cortex://");
        state.data_dir.join(uri_path)
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
    JsonExtractor(req): JsonExtractor<WriteFileRequest>,
) -> Result<Json<ApiResponse<String>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = req.path.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        let uri_path = req.path.trim_start_matches("cortex://");
        state.data_dir.join(uri_path)
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

// ==================== Directory Stats ====================

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
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        let uri_path = params.uri.trim_start_matches("cortex://");
        state.data_dir.join(uri_path)
    };
    
    tracing::debug!("Getting stats for: {:?}", base_path);
    
    // Use spawn_blocking for potentially slow recursive file counting
    let stats = tokio::task::spawn_blocking(move || {
        count_files_recursive(&base_path)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Failed to get stats: {}", e)))?
    .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(DirectoryStats {
        file_count: stats.0,
        total_size: stats.1,
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
        
        // Only skip '.' and '..' directories, but count hidden files (like .session.json)
        let name = entry_name.to_string_lossy();
        if name == "." || name == ".." {
            continue;
        }
        
        // Skip macOS .DS_Store files
        if name == ".DS_Store" {
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

// ==================== Layered Access ====================

/// Query for layered access endpoints
#[derive(Debug, Deserialize)]
pub struct LayerQuery {
    uri: String,
}

/// Response for layered access endpoints
#[derive(Debug, Serialize)]
pub struct LayerResponse {
    pub uri: String,
    pub content: String,
    pub layer: String,
    pub token_count: usize,
}

/// Get L0 abstract layer (~100 tokens) for quick relevance checking
pub async fn get_abstract(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LayerQuery>,
) -> Result<Json<ApiResponse<LayerResponse>>> {
    let (_base_path, layer_path) = resolve_layer_path(&state, &params.uri, "abstract").await?;
    
    tracing::debug!("Reading abstract layer: {:?}", layer_path);
    
    let content = tokio::fs::read_to_string(&layer_path)
        .await
        .map_err(|e| AppError::Internal(format!("Abstract not found for '{}': {}", params.uri, e)))?;
    
    let token_count = content.split_whitespace().count();
    
    Ok(Json(ApiResponse::success(LayerResponse {
        uri: params.uri,
        content,
        layer: "L0".to_string(),
        token_count,
    })))
}

/// Get L1 overview layer (~2000 tokens) for understanding core information
pub async fn get_overview(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LayerQuery>,
) -> Result<Json<ApiResponse<LayerResponse>>> {
    let (_base_path, layer_path) = resolve_layer_path(&state, &params.uri, "overview").await?;
    
    tracing::debug!("Reading overview layer: {:?}", layer_path);
    
    let content = tokio::fs::read_to_string(&layer_path)
        .await
        .map_err(|e| AppError::Internal(format!("Overview not found for '{}': {}", params.uri, e)))?;
    
    let token_count = content.split_whitespace().count();
    
    Ok(Json(ApiResponse::success(LayerResponse {
        uri: params.uri,
        content,
        layer: "L1".to_string(),
        token_count,
    })))
}

/// Get L2 full content layer - complete original content
pub async fn get_content(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LayerQuery>,
) -> Result<Json<ApiResponse<LayerResponse>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = params.uri.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        let uri_path = params.uri.trim_start_matches("cortex://");
        state.data_dir.join(uri_path)
    };
    
    tracing::debug!("Reading content layer: {:?}", base_path);
    
    let content = tokio::fs::read_to_string(&base_path)
        .await
        .map_err(|e| AppError::Internal(format!("Content not found for '{}': {}", params.uri, e)))?;
    
    let token_count = content.split_whitespace().count();
    
    Ok(Json(ApiResponse::success(LayerResponse {
        uri: params.uri,
        content,
        layer: "L2".to_string(),
        token_count,
    })))
}

/// Resolve the path to a layer file (abstract or overview)
/// 
/// For file URIs (ending with .md): layer file is in the same directory
/// For directory URIs: layer file is directly in that directory
async fn resolve_layer_path(
    state: &AppState,
    uri: &str,
    layer_type: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Determine if URI points to a file (ends with .md) or directory
    let is_file = uri.ends_with(".md");
    
    // Get the directory path
    let dir_uri = if is_file {
        uri.rsplit_once('/').map(|(dir, _)| dir).unwrap_or(uri)
    } else {
        uri
    };
    
    // Build the base path
    let base_path = if let Some(root) = tenant_root {
        let uri_path = dir_uri.trim_start_matches("cortex://");
        root.join(uri_path)
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        let uri_path = dir_uri.trim_start_matches("cortex://");
        state.data_dir.join(uri_path)
    };
    
    // Layer file name
    let layer_file = format!(".{}.md", layer_type);
    let layer_path = base_path.join(&layer_file);
    
    Ok((base_path, layer_path))
}

// ==================== Layer Loading Helper ====================

/// Load additional layers (L1 overview, L2 content) for a URI
/// Returns (overview, content, updated_layers)
pub async fn load_layers_for_uri(
    base_dir: &std::path::Path,
    uri: &str,
    return_layers: &[String],
) -> (Option<String>, Option<String>, Vec<String>) {
    let mut overview = None;
    let mut content = None;
    let mut layers = vec!["L0".to_string()];
    
    let is_file = uri.ends_with(".md");
    let uri_path = uri.trim_start_matches("cortex://");
    let file_path = base_dir.join(uri_path);
    
    let dir_path = if is_file {
        file_path.parent().unwrap_or(&file_path).to_path_buf()
    } else {
        file_path.clone()
    };
    
    // Load L1 overview if requested
    if return_layers.contains(&"L1".to_string()) {
        let overview_path = dir_path.join(".overview.md");
        // Direct read without exists() check - single I/O operation
        if let Ok(content) = tokio::fs::read_to_string(&overview_path).await {
            overview = Some(content);
            layers.push("L1".to_string());
        }
    }
    
    // Load L2 content if requested
    if return_layers.contains(&"L2".to_string()) && is_file {
        // Direct read without exists() check - single I/O operation
        if let Ok(file_content) = tokio::fs::read_to_string(&file_path).await {
            content = Some(file_content);
            layers.push("L2".to_string());
        }
    }
    
    (overview, content, layers)
}

/// Load abstract for a URI (single I/O operation)
pub async fn load_abstract_for_uri(base_dir: &std::path::Path, uri: &str) -> Option<String> {
    let is_file = uri.ends_with(".md");
    let uri_path = uri.trim_start_matches("cortex://");
    let file_path = base_dir.join(uri_path);
    
    let dir_path = if is_file {
        file_path.parent()?
    } else {
        &file_path
    };
    
    let abstract_path = dir_path.join(".abstract.md");
    // Direct read without exists() check - single I/O operation
    tokio::fs::read_to_string(&abstract_path).await.ok()
}

// ==================== Explore ====================

/// Smart exploration of memory space, combining search and browsing
pub async fn explore(
    State(state): State<Arc<AppState>>,
    JsonExtractor(req): JsonExtractor<ExploreRequest>,
) -> Result<Json<ApiResponse<ExploreResponse>>> {
    use cortex_mem_core::SearchOptions;

    // Check if vector engine is available
    let vector_engine_lock = state.vector_engine.read().await;
    let vector_engine = vector_engine_lock.as_ref().ok_or_else(|| {
        AppError::BadRequest("Vector search not available. Qdrant and Embedding service must be configured.".to_string())
    })?;

    // Get tenant root
    let tenant_root = state.current_tenant_root.read().await.clone();
    let base_dir = if let Some(ref root) = tenant_root {
        root.clone()
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        state.data_dir.clone()
    };

    // Perform search within the start_uri scope
    let options = SearchOptions {
        limit: 20,
        threshold: 0.3, // Lower threshold for exploration
        root_uri: Some(req.start_uri.clone()),
        recursive: true,
        precomputed_intent: None,
    };

    let search_results = vector_engine
        .layered_semantic_search(&req.query, &options)
        .await
        .map_err(|e| AppError::Internal(format!("Explore search failed: {}", e)))?;

    // Build exploration path and matches
    let mut exploration_path = Vec::new();
    let mut matches = Vec::new();
    let mut explored_uris = std::collections::HashSet::new();

    for result in search_results {
        // Add to exploration path
        if !explored_uris.contains(&result.uri) {
            // For files: use snippet as abstract (it's already a relevant excerpt)
            // For directories: load the directory's .abstract.md
            let abstract_text = if result.uri.ends_with(".md") {
                // File-level result: snippet is the relevant content excerpt
                Some(result.snippet.clone())
            } else {
                // Directory-level result: load directory's abstract
                load_abstract_for_uri(&base_dir, &result.uri).await
            };

            exploration_path.push(ExplorationPathItem {
                uri: result.uri.clone(),
                relevance_score: result.score,
                abstract_text,
            });
            explored_uris.insert(result.uri.clone());
        }

        // Build match result with requested layers
        let snippet = if result.snippet.len() > 200 {
            format!("{}...", &result.snippet.chars().take(200).collect::<String>())
        } else {
            result.snippet
        };

        // Use shared helper to load layers
        let (overview, content, layers) = load_layers_for_uri(&base_dir, &result.uri, &req.return_layers).await;

        matches.push(SearchResultResponse {
            uri: result.uri,
            score: result.score,
            snippet,
            overview,
            content,
            source: "explore".to_string(),
            layers,
        });
    }

    let total_explored = exploration_path.len();
    let total_matches = matches.len();

    Ok(Json(ApiResponse::success(ExploreResponse {
        query: req.query,
        exploration_path,
        matches,
        total_explored,
        total_matches,
    })))
}