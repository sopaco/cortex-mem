use anyhow::Result;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use chrono::Utc;
use cortex_mem_tools::MemoryOperations;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::api_models::{
    ErrorResponse, HealthResponse, ListMemoryResponse, MemoryItem, RetrieveMemoryResponse,
    StoreMemoryRequest, StoreMemoryResponse,
};

/// Retrieve memory query params
#[derive(Debug, Deserialize)]
pub struct RetrieveMemoryQuery {
    /// Search query
    pub query: Option<String>,
    /// Speaker type filter
    pub _speaker_type: Option<String>,
    /// Result limit
    pub limit: Option<usize>,
}

/// List memory query params
#[derive(Debug, Deserialize)]
pub struct ListMemoryQuery {
    /// Speaker type filter
    pub _speaker_type: Option<String>,
    /// Result limit
    pub limit: Option<usize>,
    /// Offset
    pub _offset: Option<usize>,
}

/// API server state
#[derive(Clone)]
pub struct ApiServerState {
    pub operations: Arc<MemoryOperations>,
    pub current_bot_id: Arc<std::sync::RwLock<Option<String>>>,
    pub audio_connect_mode: String,
    pub external_message_sender: Option<mpsc::UnboundedSender<String>>,
}

/// Create API router
pub fn create_router(state: ApiServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/memory/health", get(health_check))
        .route("/api/memory/store", post(store_memory))
        .route("/api/memory/retrieve", get(retrieve_memory))
        .route("/api/memory/list", get(list_memory))
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> Result<Json<HealthResponse>, StatusCode> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };
    Ok(Json(response))
}

/// Store memory endpoint
async fn store_memory(
    State(state): State<ApiServerState>,
    Json(request): Json<StoreMemoryRequest>,
) -> Result<Json<StoreMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Chat mode: send message to external channel
    if state.audio_connect_mode == "chat" {
        log::info!("Chat mode: received message: {}", request.content);

        if let Some(ref sender) = state.external_message_sender {
            if let Err(e) = sender.send(request.content.clone()) {
                log::error!("Failed to send external message: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        success: false,
                        error_type: Some("channel_error".to_string()),
                        error: format!("Failed to send message: {}", e),
                    }),
                ));
            }
            log::info!("Message sent to external channel");
        } else {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("service_unavailable".to_string()),
                    error: "External message channel not initialized".to_string(),
                }),
            ));
        }

        return Ok(Json(StoreMemoryResponse {
            success: true,
            memory_id: None,
            message: Some(format!("Chat mode: Message queued - {}", request.content)),
        }));
    }

    // Store mode
    if request.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error_type: Some("invalid_content".to_string()),
                error: "Missing required field: content".to_string(),
            }),
        ));
    }

    if request.source != "audio_listener" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error_type: Some("invalid_source".to_string()),
                error: "Invalid source value. Expected 'audio_listener'".to_string(),
            }),
        ));
    }

    let memory_id = format!("mem_{}", Uuid::new_v4());

    // Store as a message in the current thread/session
    let thread_id = state
        .current_bot_id
        .read()
        .ok()
        .and_then(|id| id.clone())
        .unwrap_or_else(|| "default".to_string());

    let content = format!(
        "[{}] Audio listener: {}",
        request.timestamp,
        request.content
    );

    match state
        .operations
        .add_message(&thread_id, "user", &content)
        .await
    {
        Ok(id) => {
            log::info!("Memory stored successfully: {}", id);
            Ok(Json(StoreMemoryResponse {
                success: true,
                memory_id: Some(id),
                message: Some("Memory stored successfully".to_string()),
            }))
        }
        Err(e) => {
            log::error!("Failed to store memory: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("internal_error".to_string()),
                    error: format!("Failed to store memory: {}", e),
                }),
            ))
        }
    }
}

/// Retrieve memory endpoint
async fn retrieve_memory(
    State(state): State<ApiServerState>,
    Query(params): Query<RetrieveMemoryQuery>,
) -> Result<Json<RetrieveMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(5);
    let query = params.query.as_deref().unwrap_or("");

    let thread_id = state
        .current_bot_id
        .read()
        .ok()
        .and_then(|id| id.clone());

    match state.operations.search(query, thread_id.as_deref(), limit).await {
        Ok(results) => {
            let memories: Vec<MemoryItem> = results
                .into_iter()
                .map(|memory| MemoryItem {
                    id: memory.uri.clone(),
                    content: memory.content,
                    source: "memory".to_string(),
                    timestamp: memory.created_at.to_rfc3339(),
                    speaker_type: None,
                    speaker_confidence: None,
                    relevance: memory.score,
                })
                .collect();

            log::info!("Retrieved {} memories", memories.len());
            Ok(Json(RetrieveMemoryResponse { memories }))
        }
        Err(e) => {
            log::error!("Failed to retrieve memories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("internal_error".to_string()),
                    error: format!("Failed to retrieve memories: {}", e),
                }),
            ))
        }
    }
}

/// List memory endpoint
async fn list_memory(
    State(state): State<ApiServerState>,
    Query(params): Query<ListMemoryQuery>,
) -> Result<Json<ListMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(10);

    match state.operations.list_sessions().await {
        Ok(sessions) => {
            // For simplicity, return sessions as memory items
            let memories: Vec<MemoryItem> = sessions
                .into_iter()
                .take(limit)
                .map(|session| MemoryItem {
                    id: session.thread_id.clone(),
                    content: format!("Session: {} ({})", session.thread_id, session.status),
                    source: "session".to_string(),
                    timestamp: session.created_at.to_rfc3339(),
                    speaker_type: None,
                    speaker_confidence: None,
                    relevance: None,
                })
                .collect();

            let total = memories.len();
            log::info!("Listed {} sessions", total);
            Ok(Json(ListMemoryResponse { memories, total }))
        }
        Err(e) => {
            log::error!("Failed to list memories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("internal_error".to_string()),
                    error: format!("Failed to list memories: {}", e),
                }),
            ))
        }
    }
}

/// Start API server
pub async fn start_api_server(state: ApiServerState, port: u16) -> Result<()> {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);

    log::info!("Starting TARS API server on http://{}", addr);

    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            log::info!("Successfully bound to address: {}", addr);

            match axum::serve(listener, app).await {
                Ok(_) => {
                    log::info!("API server stopped gracefully");
                    Ok(())
                }
                Err(e) => {
                    log::error!("API server error: {}", e);
                    Err(anyhow::anyhow!("API server error: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to bind to address {}: {}", addr, e);
            Err(anyhow::anyhow!("Failed to bind to address {}: {}", addr, e))
        }
    }
}
