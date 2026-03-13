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
use cortex_mem_core::session::SessionMetadata;

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
    
    let user_id = payload.get("user_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let agent_id = payload.get("agent_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let session_mgr = state.session_manager.write().await;
    let mut metadata = session_mgr.create_session_with_ids(&thread_id, user_id, agent_id).await?;
    
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
    use cortex_mem_core::MessageRole;

    let role = match payload.role.to_lowercase().as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => return Err(crate::error::AppError::BadRequest(
            format!("Invalid role: {}", payload.role)
        )),
    };

    // Ensure the session exists before adding a message (auto-create if missing)
    {
        let session_mgr = state.session_manager.read().await;
        if session_mgr.load_session(&thread_id).await.is_err() {
            drop(session_mgr);
            let session_mgr = state.session_manager.write().await;
            session_mgr.create_session_with_ids(&thread_id, None, None).await?;
            tracing::info!("Auto-created session '{}' on first message", thread_id);
        }
    }

    // Use SessionManager::add_message to trigger MemoryEventCoordinator events
    // This ensures proper event chain for automatic indexing and layer generation
    let session_mgr = state.session_manager.read().await;
    let message = session_mgr.add_message(&thread_id, role, payload.content).await?;
    drop(session_mgr);

    // Build message URI (matches what MessageStorage actually writes)
    let message_uri = format!(
        "cortex://session/{}/timeline/{}/{}/{}_{}.md",
        thread_id,
        message.timestamp.format("%Y-%m"),
        message.timestamp.format("%d"),
        message.timestamp.format("%H_%M_%S"),
        &message.id[..8]
    );

    // Emit LayerUpdateNeeded so the tenant-aware MemoryEventCoordinator
    // (re)generates L0/L1 layer summaries for the session's timeline directory.
    // VectorSyncNeeded is handled automatically by AutomationManager (via SessionManager's
    // MessageAdded event → CortexEvent → AutomationManager::index_session_l2).
    {
        use cortex_mem_core::memory_events::{ChangeType, MemoryEvent};
        use cortex_mem_core::memory_index::MemoryScope;

        let tx_guard = state.memory_event_tx.read().await;
        if let Some(ref tx) = *tx_guard {
            let day_dir_uri = format!(
                "cortex://session/{}/timeline/{}/{}",
                thread_id,
                message.timestamp.format("%Y-%m"),
                message.timestamp.format("%d"),
            );
            match tx.send(MemoryEvent::LayerUpdateNeeded {
                scope: MemoryScope::Session,
                owner_id: thread_id.clone(),
                directory_uri: day_dir_uri,
                change_type: ChangeType::Add,
                changed_file: message_uri.clone(),
            }) {
                Ok(_) => tracing::info!("📤 Dispatched LayerUpdateNeeded for session {}", thread_id),
                Err(e) => tracing::error!("❌ Failed to dispatch LayerUpdateNeeded: {}", e),
            }
        } else {
            tracing::warn!("⚠️ No memory_event_tx available, skipping event dispatch");
        }
    }

    Ok(Json(ApiResponse::success(format!("Message saved to {}", message_uri))))
}

/// Close session
pub async fn close_session(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
) -> Result<Json<ApiResponse<SessionResponse>>> {
    let mut session_mgr = state.session_manager.write().await;
    let metadata = session_mgr.close_session(&thread_id).await?;
    drop(session_mgr);

    // Emit SessionClosed event to trigger full memory extraction pipeline.
    // This mirrors what cortex-mem-tools does in close_session_sync.
    let tx_guard = state.memory_event_tx.read().await;
    if let Some(ref tx) = *tx_guard {
        // Load user_id / agent_id from metadata; fall back to "default" if not set.
        let user_id = metadata.user_id.clone().unwrap_or_else(|| "default".to_string());
        let agent_id = metadata.agent_id.clone().unwrap_or_else(|| "default".to_string());
        let _ = tx.send(cortex_mem_core::memory_events::MemoryEvent::SessionClosed {
            session_id: thread_id.clone(),
            user_id,
            agent_id,
        });
        tracing::info!("SessionClosed event emitted for thread {}", thread_id);
    }
    drop(tx_guard);

    let response = SessionResponse {
        thread_id: metadata.thread_id,
        status: format!("{:?}", metadata.status),
        message_count: metadata.message_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}
