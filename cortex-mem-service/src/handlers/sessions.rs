use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    error::Result,
    models::{ApiResponse, SessionResponse, AddMessageRequest},
    state::AppState,
};
use cortex_mem_core::{FilesystemOperations, session::SessionMetadata};

/// Create a new session
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<SessionResponse>>> {
    let thread_id = payload.get("thread_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let title = payload.get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let session_mgr = state.session_manager.write().await;
    let mut metadata = session_mgr.create_session(&thread_id).await?;
    
    // Set title if provided
    if let Some(t) = title {
        metadata.set_title(t);
        session_mgr.update_session(&metadata).await?;
    }

    let response = SessionResponse {
        thread_id: metadata.thread_id,
        status: format!("{:?}", metadata.status),
        message_count: metadata.message_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// List all sessions
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<SessionResponse>>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();
    
    // Build the path
    let session_path = if let Some(root) = tenant_root {
        root.join("session")
    } else {
        state.data_dir.join("cortex").join("session")
    };
    
    tracing::debug!("Listing sessions from: {:?}", session_path);
    
    if !session_path.exists() {
        return Ok(Json(ApiResponse::success(vec![])));
    }

    let mut sessions = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&session_path) {
        for entry in dir.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let thread_id = entry.file_name().to_string_lossy().to_string();
                
                // Skip hidden directories
                if thread_id.starts_with('.') {
                    continue;
                }
                
                // Try to load session metadata directly from file
                let metadata_path = entry.path().join(".session.json");
                if metadata_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                        if let Ok(metadata) = serde_json::from_str::<SessionMetadata>(&content) {
                            sessions.push(SessionResponse {
                                thread_id: metadata.thread_id,
                                status: format!("{:?}", metadata.status),
                                message_count: metadata.message_count,
                                created_at: metadata.created_at,
                                updated_at: metadata.updated_at,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(sessions)))
}

/// Add message to session
pub async fn add_message(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
    Json(payload): Json<AddMessageRequest>,
) -> Result<Json<ApiResponse<String>>> {
    use cortex_mem_core::{Message, MessageRole, MessageStorage};

    let role = match payload.role.to_lowercase().as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => return Err(crate::error::AppError::BadRequest(
            format!("Invalid role: {}", payload.role)
        )),
    };

    let message = Message::new(role, payload.content);
    
    // Save message using MessageStorage
    let message_storage = MessageStorage::new(state.filesystem.clone());
    let message_uri = message_storage.save_message(&thread_id, &message).await?;
    
    // Update session metadata
    let session_mgr = state.session_manager.write().await;
    let mut metadata = session_mgr.load_session(&thread_id).await?;
    metadata.update_message_count(metadata.message_count + 1);
    
    // Save updated metadata
    let metadata_uri = format!("cortex://session/{}/.session.json", thread_id);
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    state.filesystem.write(&metadata_uri, &metadata_json).await?;

    Ok(Json(ApiResponse::success(format!("Message saved to {}", message_uri))))
}

/// Close session
pub async fn close_session(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
) -> Result<Json<ApiResponse<SessionResponse>>> {
    let mut session_mgr = state.session_manager.write().await;
    let metadata = session_mgr.close_session(&thread_id).await?;

    let response = SessionResponse {
        thread_id: metadata.thread_id,
        status: format!("{:?}", metadata.status),
        message_count: metadata.message_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}
