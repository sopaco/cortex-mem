use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use cortex_mem_core::{
    memory::{DefaultMemoryOptimizer, MemoryOptimizer},
    types::{
        OptimizationConfig, OptimizationFilters, OptimizationRequest, OptimizationResult,
        OptimizationStrategy,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;

use crate::{models::ErrorResponse, AppState};

/// 优化任务状态（用于内存存储）
#[derive(Debug, Clone, Serialize)]
pub struct OptimizationJobState {
    pub job_id: String,
    pub status: String,
    pub progress: u8,
    pub current_phase: String,
    pub logs: Vec<String>,
    pub result: Option<OptimizationResult>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration: Option<i64>,
}

/// 启动优化请求
#[derive(Debug, Deserialize)]
pub struct StartOptimizationRequest {
    pub memory_type: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub similarity_threshold: Option<f32>,
    pub dry_run: Option<bool>,
    pub verbose: Option<bool>,
    pub strategy: Option<String>,
    pub aggressive: Option<bool>,
    pub timeout_minutes: Option<u64>,
}

/// 优化响应
#[derive(Debug, Serialize)]
pub struct OptimizationResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<ErrorInfo>,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// 优化历史查询参数
#[derive(Debug, Deserialize)]
pub struct OptimizationHistoryQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 清理请求
#[derive(Debug, Deserialize)]
pub struct CleanupRequest {
    pub max_age_days: Option<u64>,
}

/// 分析请求
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub memory_type: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub similarity_threshold: Option<f32>,
}

/// 启动优化任务
pub async fn start_optimization(
    State(state): State<AppState>,
    Json(request): Json<StartOptimizationRequest>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let job_id = format!("opt_{}_{}", Utc::now().timestamp(), Uuid::new_v4());

    // 初始化任务状态
    let job_state = OptimizationJobState {
        job_id: job_id.clone(),
        status: "pending".to_string(),
        progress: 0,
        current_phase: "初始化".to_string(),
        logs: vec![format!("优化任务 {} 已创建", job_id)],
        result: None,
        start_time: Utc::now().to_rfc3339(),
        end_time: None,
        duration: None,
    };

    // 存储任务状态
    {
        let mut jobs = state.optimization_jobs.write().await;
        jobs.insert(job_id.clone(), job_state.clone());
    }

    // 解析策略
    let strategy = match request.strategy.as_deref() {
        Some("full") => OptimizationStrategy::Full,
        Some("deduplication") => OptimizationStrategy::Deduplication,
        Some("quality") => OptimizationStrategy::Quality,
        Some("relevance") => OptimizationStrategy::Relevance,
        _ => OptimizationStrategy::Full,
    };

    // 构建优化请求
    let opt_request = OptimizationRequest {
        optimization_id: Some(job_id.clone()),
        strategy,
        filters: OptimizationFilters {
            user_id: request.user_id.clone(),
            agent_id: request.agent_id.clone(),
            memory_type: request
                .memory_type
                .as_ref()
                .map(|t| cortex_mem_core::types::MemoryType::parse(t)),
            date_range: None,
            importance_range: None,
            custom_filters: HashMap::new(),
        },
        aggressive: request.aggressive.unwrap_or(false),
        dry_run: request.dry_run.unwrap_or(false),
        timeout_minutes: request.timeout_minutes,
    };

    // 异步执行优化
    let state_clone = state.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        execute_optimization(state_clone, job_id_clone, opt_request).await;
    });

    // 返回响应
    let response = OptimizationResponse {
        success: true,
        data: Some(serde_json::json!({
            "job_id": job_id,
            "message": "优化任务已启动",
            "status": "pending",
            "start_time": job_state.start_time,
        })),
        error: None,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}

/// 执行优化任务
async fn execute_optimization(
    state: AppState,
    job_id: String,
    request: OptimizationRequest,
) {
    // 更新状态为运行中
    {
        let mut jobs = state.optimization_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = "running".to_string();
            job.progress = 10;
            job.current_phase = "准备优化".to_string();
            job.logs.push("开始准备优化任务...".to_string());
        }
    }

    // 创建优化器（使用默认配置）
    let config = OptimizationConfig::default();

    let optimizer = DefaultMemoryOptimizer::new(state.memory_manager.clone(), config);

    // 执行优化
    {
        let mut jobs = state.optimization_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.progress = 30;
            job.current_phase = "执行优化命令".to_string();
            job.logs.push("正在执行优化...".to_string());
        }
    }

    match optimizer.optimize(&request).await {
        Ok(result) => {
            // 成功完成
            let mut jobs = state.optimization_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                let end_time = Utc::now();
                let duration = (end_time.timestamp() - Utc::now().timestamp()).abs();

                job.status = "completed".to_string();
                job.progress = 100;
                job.current_phase = "完成".to_string();
                job.result = Some(result);
                job.end_time = Some(end_time.to_rfc3339());
                job.duration = Some(duration);
                job.logs.push("优化任务完成".to_string());
            }

            info!("优化任务 {} 完成", job_id);
        }
        Err(e) => {
            // 失败
            let mut jobs = state.optimization_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                let end_time = Utc::now();
                let duration = (end_time.timestamp() - Utc::now().timestamp()).abs();

                job.status = "failed".to_string();
                job.end_time = Some(end_time.to_rfc3339());
                job.duration = Some(duration);
                job.logs
                    .push(format!("执行失败: {}", e.to_string()));
            }

            error!("优化任务 {} 失败: {}", job_id, e);
        }
    }
}

/// 获取优化任务状态
pub async fn get_optimization_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let jobs = state.optimization_jobs.read().await;

    if let Some(job_state) = jobs.get(&job_id) {
        let response = OptimizationResponse {
            success: true,
            data: Some(serde_json::to_value(job_state).unwrap()),
            error: None,
            timestamp: Utc::now().to_rfc3339(),
        };
        Ok(Json(response))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("优化任务 {} 不存在", job_id),
                code: "JOB_NOT_FOUND".to_string(),
            }),
        ))
    }
}

/// 取消优化任务
pub async fn cancel_optimization(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut jobs = state.optimization_jobs.write().await;

    if let Some(job_state) = jobs.get_mut(&job_id) {
        if job_state.status == "completed"
            || job_state.status == "failed"
            || job_state.status == "cancelled"
        {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("优化任务 {} 已结束，无法取消", job_id),
                    code: "JOB_COMPLETED".to_string(),
                }),
            ));
        }

        job_state.status = "cancelled".to_string();
        job_state.logs.push("优化任务已被用户取消".to_string());
        let end_time = Utc::now();
        job_state.end_time = Some(end_time.to_rfc3339());

        let response = OptimizationResponse {
            success: true,
            data: Some(serde_json::json!({
                "job_id": job_id,
                "message": "优化任务已取消",
                "status": "cancelled",
                "cancelled_at": end_time.to_rfc3339(),
            })),
            error: None,
            timestamp: Utc::now().to_rfc3339(),
        };

        Ok(Json(response))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("优化任务 {} 不存在", job_id),
                code: "JOB_NOT_FOUND".to_string(),
            }),
        ))
    }
}

/// 获取优化历史
pub async fn get_optimization_history(
    State(state): State<AppState>,
    Query(query): Query<OptimizationHistoryQuery>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let jobs = state.optimization_jobs.read().await;

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let mut history: Vec<_> = jobs.values().cloned().collect();

    // 应用过滤器
    if let Some(status) = &query.status {
        history.retain(|job| &job.status == status);
    }

    // 按开始时间倒序排序
    history.sort_by(|a, b| b.start_time.cmp(&a.start_time));

    // 分页
    let total = history.len();
    let paginated: Vec<_> = history
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|job| {
            serde_json::json!({
                "job_id": job.job_id,
                "status": job.status,
                "start_time": job.start_time,
                "end_time": job.end_time,
                "duration": job.duration,
                "logs_count": job.logs.len(),
                "has_result": job.result.is_some(),
            })
        })
        .collect();

    let response = OptimizationResponse {
        success: true,
        data: Some(serde_json::json!({
            "total": total,
            "history": paginated,
            "pagination": {
                "limit": limit,
                "offset": offset,
                "total": total,
            },
        })),
        error: None,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}

/// 分析优化问题（预览模式）
pub async fn analyze_optimization(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeRequest>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 构建优化请求（dry_run模式）
    let opt_request = OptimizationRequest {
        optimization_id: None,
        strategy: OptimizationStrategy::Full,
        filters: OptimizationFilters {
            user_id: request.user_id.clone(),
            agent_id: request.agent_id.clone(),
            memory_type: request
                .memory_type
                .as_ref()
                .map(|t| cortex_mem_core::types::MemoryType::parse(t)),
            date_range: None,
            importance_range: None,
            custom_filters: HashMap::new(),
        },
        aggressive: false,
        dry_run: true,
        timeout_minutes: Some(5),
    };

    // 创建优化器（使用默认配置）
    let config = OptimizationConfig::default();

    let optimizer = DefaultMemoryOptimizer::new(state.memory_manager.clone(), config);

    match optimizer.optimize(&opt_request).await {
        Ok(result) => {
            // 解析结果
            let issues = &result.issues_found;
            let total_affected = issues.iter().map(|i| i.affected_memories.len()).sum::<usize>();

            let response = OptimizationResponse {
                success: true,
                data: Some(serde_json::json!({
                    "issues": issues,
                    "summary": {
                        "total_issues": issues.len(),
                        "total_affected_memories": total_affected,
                        "estimated_savings_mb": (total_affected as f64 * 0.15).round(),
                        "estimated_duration_minutes": (total_affected / 10).max(1),
                    },
                    "recommendations": issues.iter().map(|issue| {
                        serde_json::json!({
                            "type": format!("{:?}", issue.kind),
                            "action": match issue.severity {
                                cortex_mem_core::types::IssueSeverity::High | cortex_mem_core::types::IssueSeverity::Critical => "立即处理",
                                cortex_mem_core::types::IssueSeverity::Medium => "建议处理",
                                cortex_mem_core::types::IssueSeverity::Low => "可选处理",
                            },
                            "priority": format!("{:?}", issue.severity),
                        })
                    }).collect::<Vec<_>>(),
                })),
                error: None,
                timestamp: Utc::now().to_rfc3339(),
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!("分析优化问题失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("分析失败: {}", e),
                    code: "ANALYSIS_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// 获取优化统计
pub async fn get_optimization_statistics(
    State(state): State<AppState>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let jobs = state.optimization_jobs.read().await;

    let history: Vec<_> = jobs.values().collect();

    let total_jobs = history.len();
    let successful_jobs = history.iter().filter(|j| j.status == "completed").count();
    let failed_jobs = history.iter().filter(|j| j.status == "failed").count();
    let cancelled_jobs = history.iter().filter(|j| j.status == "cancelled").count();

    let total_memories_processed = history
        .iter()
        .filter_map(|j| {
            j.result.as_ref().map(|r| {
                r.actions_performed.len()
            })
        })
        .sum::<usize>();

    let avg_duration = if !history.is_empty() {
        history
            .iter()
            .filter_map(|j| j.duration)
            .sum::<i64>() as f64
            / history.len() as f64
    } else {
        0.0
    };

    let last_run = history
        .iter()
        .max_by(|a, b| a.start_time.cmp(&b.start_time))
        .map(|j| j.start_time.clone());

    let response = OptimizationResponse {
        success: true,
        data: Some(serde_json::json!({
            "total_jobs": total_jobs,
            "successful_jobs": successful_jobs,
            "failed_jobs": failed_jobs,
            "cancelled_jobs": cancelled_jobs,
            "total_memories_processed": total_memories_processed,
            "avg_duration": avg_duration,
            "last_run": last_run,
        })),
        error: None,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}

/// 清理旧的历史记录
pub async fn cleanup_history(
    State(state): State<AppState>,
    Json(request): Json<CleanupRequest>,
) -> Result<Json<OptimizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let max_age_days = request.max_age_days.unwrap_or(7);
    let cutoff_time = Utc::now().timestamp() - (max_age_days as i64 * 24 * 60 * 60);

    let mut jobs = state.optimization_jobs.write().await;
    let mut deleted = 0;

    jobs.retain(|id, _| {
        if let Some(timestamp_str) = id.split('_').nth(1) {
            if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                if timestamp < cutoff_time {
                    deleted += 1;
                    return false;
                }
            }
        }
        true
    });

    let response = OptimizationResponse {
        success: true,
        data: Some(serde_json::json!({
            "deleted": deleted,
            "remaining": jobs.len(),
            "message": format!("已清理 {} 条旧记录", deleted),
        })),
        error: None,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}
