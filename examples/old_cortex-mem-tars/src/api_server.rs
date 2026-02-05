use anyhow::Result;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use cortex_mem_core::memory::MemoryManager;
use cortex_mem_core::types::{Filters, MemoryMetadata, MemoryType, Message};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

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

/// 验证说话人类型
fn validate_speaker_type(speaker_type: &str) -> Result<()> {
    if speaker_type != "user" && speaker_type != "other" {
        return Err(anyhow::anyhow!(
            "speaker_type must be 'user' or 'other', got: '{}'",
            speaker_type
        ));
    }
    Ok(())
}

/// 验证说话人置信度
fn validate_speaker_confidence(confidence: f32) -> Result<()> {
    if confidence < 0.0 || confidence > 1.0 {
        return Err(anyhow::anyhow!(
            "speaker_confidence must be between 0 and 1, got: {}",
            confidence
        ));
    }
    Ok(())
}

/// API 服务器状态
#[derive(Clone)]
pub struct ApiServerState {
    pub memory_manager: Arc<MemoryManager>,
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
    // 检查模式：如果是 chat 模式，返回特殊响应
    if state.audio_connect_mode == "chat" {
        log::info!("Chat 模式：收到消息，将模拟用户输入: {}", request.content);

        // 将消息发送到外部消息通道，由 App 处理
        if let Some(ref sender) = state.external_message_sender {
            if let Err(e) = sender.send(request.content.clone()) {
                log::error!("发送外部消息失败: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        success: false,
                        error_type: Some("channel_error".to_string()),
                        error: format!("Failed to send message to channel: {}", e),
                    }),
                ));
            }
            log::info!("✅ 消息已发送到外部消息通道");
        } else {
            log::error!("❌ external_message_sender 未初始化");
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
            message: Some(format!(
                "Chat mode: Message received and queued - {}",
                request.content
            )),
        }));
    }

    // 以下是 store 模式的原有逻辑
    // 验证必填字段
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

    // 验证说话人类型（如果提供）
    if let Some(ref speaker_type) = request.speaker_type {
        if let Err(e) = validate_speaker_type(speaker_type) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("invalid_speaker_type".to_string()),
                    error: e.to_string(),
                }),
            ));
        }
    }

    // 验证说话人置信度（如果提供）
    if let Some(confidence) = request.speaker_confidence {
        if let Err(e) = validate_speaker_confidence(confidence) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("invalid_speaker_confidence".to_string()),
                    error: e.to_string(),
                }),
            ));
        }
    }

    // 解析时间戳
    let timestamp: DateTime<Utc> = match DateTime::parse_from_rfc3339(&request.timestamp) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("invalid_timestamp".to_string()),
                    error: "Invalid timestamp format. Expected RFC 3339".to_string(),
                }),
            ));
        }
    };

    let timestamp_str = format!(
        "{}_{}",
        timestamp.format("%Y-%m-%d"),
        timestamp.format("%H:%M:%S")
    );
    // 生成记忆 ID
    let memory_id = format!(
        "mem_{}_{}",
        timestamp_str,
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("unknown")
    );

    // 创建消息
    let messages = vec![Message {
        role: "user".to_string(),
        content: format!(
            "当前我所处的办公与会议环境中，时间是{}，能听到这样的声音：{}",
            timestamp_str,
            request.content.clone()
        ),
        name: None,
    }];

    // 创建元数据
    let mut custom_metadata = HashMap::new();
    custom_metadata.insert("source".to_string(), json!("audio_listener"));
    custom_metadata.insert("original_timestamp".to_string(), json!(request.timestamp));

    // 添加说话人信息到元数据
    if let Some(ref speaker_type) = request.speaker_type {
        custom_metadata.insert("speaker_type".to_string(), json!(speaker_type));
    }
    if let Some(confidence) = request.speaker_confidence {
        custom_metadata.insert("speaker_confidence".to_string(), json!(confidence));
    }

    // 获取当前选中的机器人 ID
    let current_bot_id = state
        .current_bot_id
        .read()
        .map(|bot_id| bot_id.clone())
        .unwrap_or(None);

    let agent_id = match current_bot_id {
        Some(id) => id,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("no_bot_selected".to_string()),
                    error: "No bot selected. Please select a bot before storing memory.".to_string(),
                }),
            ));
        }
    };

    let metadata = MemoryMetadata {
        user_id: Some("tars_user".to_string()),
        agent_id: Some(agent_id),
        run_id: None,
        actor_id: None,
        role: Some("user".to_string()),
        memory_type: MemoryType::Episodic,
        hash: Uuid::new_v4().to_string(),
        importance_score: 0.8,
        entities: vec![],
        topics: vec![],
        custom: custom_metadata,
    };

    // 保存到记忆系统
    match state.memory_manager.add_memory(&messages, metadata).await {
        Ok(results) => {
            log::info!(
                "✅ Memory stored successfully: {} (content length: {}, speaker_type: {:?})",
                memory_id,
                request.content.len(),
                request.speaker_type
            );

            Ok(Json(StoreMemoryResponse {
                success: true,
                memory_id: Some(memory_id),
                message: Some(format!(
                    "Memory stored successfully, {} memories created",
                    results.len()
                )),
            }))
        }
        Err(e) => {
            log::error!("❌ Failed to store memory: {}", e);

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

/// 查询记忆端点
async fn retrieve_memory(
    State(state): State<ApiServerState>,
    Query(params): Query<RetrieveMemoryQuery>,
) -> Result<Json<RetrieveMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 验证 speaker_type 参数（如果提供）
    if let Some(ref speaker_type) = params.speaker_type {
        if let Err(e) = validate_speaker_type(speaker_type) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("invalid_speaker_type".to_string()),
                    error: e.to_string(),
                }),
            ));
        }
    }

    let limit = params.limit.unwrap_or(5);

    // 构建过滤器
    let mut filters = Filters::default();
    if let Some(ref speaker_type) = params.speaker_type {
        let mut custom = HashMap::new();
        custom.insert("speaker_type".to_string(), json!(speaker_type));
        filters.custom = custom;
    }

    // 执行查询
    match state
        .memory_manager
        .search(params.query.as_deref().unwrap_or(""), &filters, limit)
        .await
    {
        Ok(scored_memories) => {
            let memories: Vec<MemoryItem> = scored_memories
                .into_iter()
                .map(|sm| MemoryItem {
                    id: sm.memory.id,
                    content: sm.memory.content,
                    source: "audio_listener".to_string(),
                    timestamp: sm.memory.created_at.to_rfc3339(),
                    speaker_type: sm
                        .memory
                        .metadata
                        .custom
                        .get("speaker_type")
                        .and_then(|v: &Value| v.as_str())
                        .map(|s| s.to_string()),
                    speaker_confidence: sm
                        .memory
                        .metadata
                        .custom
                        .get("speaker_confidence")
                        .and_then(|v: &Value| v.as_f64())
                        .map(|f| f as f32),
                    relevance: Some(sm.score),
                })
                .collect();

            log::info!(
                "✅ Retrieved {} memories (filter: speaker_type={:?}, query={:?})",
                memories.len(),
                params.speaker_type,
                params.query
            );

            Ok(Json(RetrieveMemoryResponse { memories }))
        }
        Err(e) => {
            log::error!("❌ Failed to retrieve memories: {}", e);

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

/// 列出记忆端点
async fn list_memory(
    State(state): State<ApiServerState>,
    Query(params): Query<ListMemoryQuery>,
) -> Result<Json<ListMemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 验证 speaker_type 参数（如果提供）
    if let Some(ref speaker_type) = params.speaker_type {
        if let Err(e) = validate_speaker_type(speaker_type) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error_type: Some("invalid_speaker_type".to_string()),
                    error: e.to_string(),
                }),
            ));
        }
    }

    let limit = params.limit.unwrap_or(10);
    let offset = params.offset.unwrap_or(0);

    // 构建过滤器
    let mut filters = Filters::default();
    if let Some(ref speaker_type) = params.speaker_type {
        let mut custom = HashMap::new();
        custom.insert("speaker_type".to_string(), json!(speaker_type));
        filters.custom = custom;
    }

    // 执行查询
    match state
        .memory_manager
        .list(&filters, Some(limit + offset))
        .await
    {
        Ok(memories) => {
            // 应用分页
            let paginated_memories: Vec<_> = memories
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|memory| MemoryItem {
                    id: memory.id,
                    content: memory.content,
                    source: "audio_listener".to_string(),
                    timestamp: memory.created_at.to_rfc3339(),
                    speaker_type: memory
                        .metadata
                        .custom
                        .get("speaker_type")
                        .and_then(|v: &Value| v.as_str())
                        .map(|s| s.to_string()),
                    speaker_confidence: memory
                        .metadata
                        .custom
                        .get("speaker_confidence")
                        .and_then(|v: &Value| v.as_f64())
                        .map(|f| f as f32),
                    relevance: None,
                })
                .collect();

            let total = paginated_memories.len();

            log::info!(
                "✅ Listed {} memories (filter: speaker_type={:?}, limit={}, offset={})",
                total,
                params.speaker_type,
                limit,
                offset
            );

            Ok(Json(ListMemoryResponse {
                memories: paginated_memories,
                total,
            }))
        }
        Err(e) => {
            log::error!("❌ Failed to list memories: {}", e);

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

/// 启动 API 服务器
pub async fn start_api_server(state: ApiServerState, port: u16) -> Result<()> {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);

    log::info!("🚀 Starting TARS API server on http://{}", addr);

    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            log::info!("✅ Successfully bound to address: {}", addr);

            match axum::serve(listener, app).await {
                Ok(_) => {
                    log::info!("✅ API server stopped gracefully");
                    Ok(())
                }
                Err(e) => {
                    log::error!("❌ API server error: {}", e);
                    Err(anyhow::anyhow!("API server error: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("❌ Failed to bind to address {}: {}", addr, e);
            Err(anyhow::anyhow!("Failed to bind to address {}: {}", addr, e))
        }
    }
}
