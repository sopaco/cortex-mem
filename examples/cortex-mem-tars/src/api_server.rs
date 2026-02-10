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

use crate::api_models::{
    ErrorResponse, HealthResponse, ListMemoryResponse, MemoryItem, RetrieveMemoryResponse,
    StoreMemoryRequest, StoreMemoryResponse,
};

/// 查询记忆参数
#[derive(Debug, Deserialize)]
pub struct RetrieveMemoryQuery {
    /// 查询关键词
    pub query: Option<String>,
    /// 说话人类型过滤
    pub speaker_type: Option<String>,
    /// 返回数量限制
    pub limit: Option<usize>,
}

/// 列出记忆参数
#[derive(Debug, Deserialize)]
pub struct ListMemoryQuery {
    /// 说话人类型过滤
    pub speaker_type: Option<String>,
    /// 返回数量限制
    pub limit: Option<usize>,
    /// 偏移量
    pub offset: Option<usize>,
}

/// API 服务器状态
#[derive(Clone)]
pub struct ApiServerState {
    pub operations: Arc<MemoryOperations>,
    pub current_bot_id: Arc<std::sync::RwLock<Option<String>>>,
    pub audio_connect_mode: String,
    pub external_message_sender: Option<mpsc::UnboundedSender<String>>,
}

/// 创建 API 路由器
pub fn create_router(state: ApiServerState) -> Router {
    // 配置 CORS
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

/// 健康检查端点
async fn health_check() -> Result<Json<HealthResponse>, StatusCode> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}

/// 存储记忆端点
async fn store_memory(
    State(state): State<ApiServerState>,
    Json(request): Json<StoreMemoryRequest>,
) -> Result<Json<StoreMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    log::info!("收到存储记忆请求");

    // 获取当前 bot_id
    let bot_id = if let Ok(current_bot_id) = state.current_bot_id.read() {
        current_bot_id.clone().unwrap_or_else(|| "default".to_string())
    } else {
        "default".to_string()
    };

    // 使用 add_message 存储消息
    let role = if request.speaker_type.as_deref() == Some("user") {
        "user"
    } else {
        "assistant"
    };

    match state.operations.add_message(&bot_id, role, &request.content).await {
        Ok(message_id) => {
            log::info!("成功存储记忆，ID: {}", message_id);

            let response = StoreMemoryResponse {
                success: true,
                memory_id: Some(message_id),
                message: Some("Memory stored successfully".to_string()),
            };

            Ok(Json(response))
        }
        Err(e) => {
            log::error!("存储记忆失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("STORAGE_ERROR".to_string()),
                    error: format!("Failed to store memory: {}", e),
                }),
            ))
        }
    }
}

/// 检索记忆端点
async fn retrieve_memory(
    State(state): State<ApiServerState>,
    Query(query): Query<RetrieveMemoryQuery>,
) -> Result<Json<RetrieveMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    log::info!("收到检索记忆请求: {:?}", query);

    // 获取当前 bot_id
    let bot_id = if let Ok(current_bot_id) = state.current_bot_id.read() {
        current_bot_id.clone()
    } else {
        None
    };

    // 如果没有查询关键词，返回错误
    let query_text = query.query.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error_type: Some("VALIDATION_ERROR".to_string()),
                error: "Query parameter is required".to_string(),
            }),
        )
    })?;

    let limit = query.limit.unwrap_or(10);

    // 使用新的 search API
    let search_args = cortex_mem_tools::SearchArgs {
        query: query_text,
        engine: Some("keyword".to_string()),
        recursive: Some(true),
        return_layers: Some(vec!["L2".to_string()]),
        scope: bot_id.map(|id| format!("cortex://threads/{}", id)),
        limit: Some(limit),
    };

    match state.operations.search(search_args).await {
        Ok(response) => {
            log::info!("成功检索到 {} 条记忆", response.total);

            let memory_items: Vec<MemoryItem> = response.results
                .into_iter()
                .map(|result| MemoryItem {
                    id: result.uri.clone(),
                    content: result.content.unwrap_or_default(),
                    source: "cortex-mem".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    speaker_type: None,
                    speaker_confidence: None,
                    relevance: Some(result.score),
                })
                .collect();

            let response = RetrieveMemoryResponse {
                memories: memory_items,
            };

            Ok(Json(response))
        }
        Err(e) => {
            log::error!("检索记忆失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("RETRIEVAL_ERROR".to_string()),
                    error: format!("Failed to retrieve memories: {}", e),
                }),
            ))
        }
    }
}

/// 列出记忆端点
async fn list_memory(
    State(state): State<ApiServerState>,
    Query(query): Query<ListMemoryQuery>,
) -> Result<Json<ListMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    log::info!("收到列出记忆请求: {:?}", query);

    // 获取当前 bot_id
    let bot_id = if let Ok(current_bot_id) = state.current_bot_id.read() {
        current_bot_id.clone()
    } else {
        None
    };

    let limit = query.limit.unwrap_or(20);

    // 使用新的 search API
    let search_args = cortex_mem_tools::SearchArgs {
        query: "".to_string(),  // 空查询列出所有
        engine: Some("keyword".to_string()),
        recursive: Some(true),
        return_layers: Some(vec!["L2".to_string()]),  // 返回完整内容
        scope: bot_id.map(|id| format!("cortex://threads/{}", id)),
        limit: Some(limit),
    };
    
    match state.operations.search(search_args).await {
        Ok(response) => {
            log::info!("成功列出 {} 条记忆", response.total);

            let memory_items: Vec<MemoryItem> = response.results
                .into_iter()
                .map(|result| MemoryItem {
                    id: result.uri.clone(),
                    content: result.content.unwrap_or_default(),
                    source: "cortex-mem".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),  // TODO: 从 URI 解析时间戳
                    speaker_type: None,
                    speaker_confidence: None,
                    relevance: Some(result.score),
                })
                .collect();

            let response = ListMemoryResponse {
                memories: memory_items,
                total: response.total,
            };

            Ok(Json(response))
        }
        Err(e) => {
            log::error!("列出记忆失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("LIST_ERROR".to_string()),
                    error: format!("Failed to list memories: {}", e),
                }),
            ))
        }
    }
}

/// 启动 API 服务器
pub async fn start_api_server(state: ApiServerState, port: u16) -> Result<()> {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);

    log::info!("🚀 API 服务器正在启动，监听地址: {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    log::info!("✅ API 服务器成功绑定到 {}", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
