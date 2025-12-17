use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use cortex_mem_core::types::{Filters, MemoryMetadata, MemoryType, Message};
use std::time::Instant;

use tracing::{error, info};

use crate::{AppState, models::{CreateMemoryRequest, ErrorResponse, HealthResponse, ListMemoryQuery, ListResponse, MemoryMetadataResponse, MemoryResponse, ScoredMemoryResponse, SearchMemoryRequest, SearchResponse, SuccessResponse, UpdateMemoryRequest, BatchDeleteRequest, BatchUpdateRequest, BatchOperationResponse, LLMStatusResponse, ModelStatus, LLMHealthResponse}};

/// Health check endpoint
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory_manager.health_check().await {
        Ok(health_status) => {
            let response = HealthResponse {
                status: if health_status.overall {
                    "healthy".to_string()
                } else {
                    "unhealthy".to_string()
                },
                vector_store: health_status.vector_store,
                llm_service: health_status.llm_service,
                timestamp: Utc::now().to_rfc3339(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            error!("Health check failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Health check failed".to_string(),
                    code: "HEALTH_CHECK_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// Create a new memory with enhanced support for procedural memory and conversations
pub async fn create_memory(
    State(state): State<AppState>,
    Json(request): Json<CreateMemoryRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let memory_type = MemoryType::parse(request.memory_type.as_deref().unwrap_or("conversational"));

    let mut metadata = MemoryMetadata::new(memory_type.clone());

    if let Some(user_id) = &request.user_id {
        metadata = metadata.with_user_id(user_id.clone());
    }

    if let Some(agent_id) = &request.agent_id {
        metadata = metadata.with_agent_id(agent_id.clone());
    }

    if let Some(run_id) = &request.run_id {
        metadata = metadata.with_run_id(run_id.clone());
    }

    if let Some(actor_id) = &request.actor_id {
        metadata = metadata.with_actor_id(actor_id.clone());
    }

    if let Some(role) = &request.role {
        metadata = metadata.with_role(role.clone());
    }

    if let Some(custom) = &request.custom {
        metadata.custom = custom.clone();
    }

    // Check if this should be handled as a conversation (for procedural memory or advanced processing)
    let is_conversation = memory_type == MemoryType::Procedural
        || request.content.contains('\n')
        || request.content.contains("Assistant:")
        || request.content.contains("User:");

    if is_conversation {
        // Handle as conversation for advanced processing
        let messages = if request.content.contains('\n') {
            // Parse conversation format
            parse_conversation_content(&request.content, &request.user_id, &request.agent_id)
        } else {
            // Single user message
            vec![Message {
                role: "user".to_string(),
                content: request.content.clone(),
                name: request.user_id.clone(),
            }]
        };

        match state.memory_manager.add_memory(&messages, metadata).await {
            Ok(results) => {
                info!("Memory created successfully with {} actions", results.len());

                let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
                let primary_id = ids.first().cloned().unwrap_or_default();

                Ok(Json(SuccessResponse {
                    message: format!("Memory created successfully with {} actions", results.len()),
                    id: Some(primary_id),
                }))
            }
            Err(e) => {
                error!("Failed to create memory: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to create memory: {}", e),
                        code: "MEMORY_CREATION_FAILED".to_string(),
                    }),
                ))
            }
        }
    } else {
        // Handle as simple content storage
        match state.memory_manager.store(request.content, metadata).await {
            Ok(memory_id) => {
                info!("Memory created with ID: {}", memory_id);
                Ok(Json(SuccessResponse {
                    message: "Memory created successfully".to_string(),
                    id: Some(memory_id),
                }))
            }
            Err(e) => {
                error!("Failed to create memory: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to create memory: {}", e),
                        code: "MEMORY_CREATION_FAILED".to_string(),
                    }),
                ))
            }
        }
    }
}

/// Parse conversation content from HTTP request
fn parse_conversation_content(
    content: &str,
    user_id: &Option<String>,
    agent_id: &Option<String>,
) -> Vec<Message> {
    let mut messages = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("User:") || trimmed.starts_with("user:") {
            let user_content = trimmed[5..].trim();
            messages.push(Message {
                role: "user".to_string(),
                content: user_content.to_string(),
                name: user_id.clone(),
            });
        } else if trimmed.starts_with("Assistant:")
            || trimmed.starts_with("assistant:")
            || trimmed.starts_with("AI:")
        {
            let assistant_content = trimmed[10..].trim();
            messages.push(Message {
                role: "assistant".to_string(),
                content: assistant_content.to_string(),
                name: agent_id.clone(),
            });
        } else {
            // If no role prefix, treat as user message
            messages.push(Message {
                role: "user".to_string(),
                content: trimmed.to_string(),
                name: user_id.clone(),
            });
        }
    }

    // If no messages were parsed, treat entire content as user message
    if messages.is_empty() {
        messages.push(Message {
            role: "user".to_string(),
            content: content.to_string(),
            name: user_id.clone(),
        });
    }

    messages
}

/// Get a memory by ID
pub async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory_manager.get(&id).await {
        Ok(Some(memory)) => {
            let response = MemoryResponse {
                id: memory.id,
                content: memory.content,
                metadata: MemoryMetadataResponse {
                    user_id: memory.metadata.user_id,
                    agent_id: memory.metadata.agent_id,
                    run_id: memory.metadata.run_id,
                    actor_id: memory.metadata.actor_id,
                    role: memory.metadata.role,
                    memory_type: format!("{:?}", memory.metadata.memory_type),
                    hash: memory.metadata.hash,
                    custom: memory.metadata.custom,
                },
                created_at: memory.created_at.to_rfc3339(),
                updated_at: memory.updated_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Memory not found".to_string(),
                code: "MEMORY_NOT_FOUND".to_string(),
            }),
        )),
        Err(e) => {
            error!("Failed to get memory: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to get memory: {}", e),
                    code: "MEMORY_RETRIEVAL_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// Update a memory
pub async fn update_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateMemoryRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory_manager.update(&id, request.content).await {
        Ok(()) => {
            info!("Memory updated: {}", id);
            Ok(Json(SuccessResponse {
                message: "Memory updated successfully".to_string(),
                id: Some(id),
            }))
        }
        Err(e) => {
            error!("Failed to update memory: {}", e);
            let status_code = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status_code,
                Json(ErrorResponse {
                    error: format!("Failed to update memory: {}", e),
                    code: "MEMORY_UPDATE_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// Delete a memory
pub async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory_manager.delete(&id).await {
        Ok(()) => {
            info!("Memory deleted: {}", id);
            Ok(Json(SuccessResponse {
                message: "Memory deleted successfully".to_string(),
                id: Some(id),
            }))
        }
        Err(e) => {
            error!("Failed to delete memory: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to delete memory: {}", e),
                    code: "MEMORY_DELETION_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// Search memories
pub async fn search_memories(
    State(state): State<AppState>,
    Json(request): Json<SearchMemoryRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut filters = Filters::new();

    if let Some(user_id) = request.user_id {
        filters.user_id = Some(user_id);
    }

    if let Some(agent_id) = request.agent_id {
        filters.agent_id = Some(agent_id);
    }

    if let Some(run_id) = request.run_id {
        filters.run_id = Some(run_id);
    }

    if let Some(actor_id) = request.actor_id {
        filters.actor_id = Some(actor_id);
    }

    if let Some(memory_type_str) = request.memory_type {
        filters.memory_type = Some(MemoryType::parse(&memory_type_str));
    }

    let limit = request.limit.unwrap_or(10);

    match state
        .memory_manager
        .search_with_threshold(
            &request.query,
            &filters,
            limit,
            request.similarity_threshold,
        )
        .await
    {
        Ok(results) => {
            let scored_memories: Vec<ScoredMemoryResponse> = results
                .into_iter()
                .map(|scored_memory| ScoredMemoryResponse {
                    memory: MemoryResponse {
                        id: scored_memory.memory.id,
                        content: scored_memory.memory.content,
                        metadata: MemoryMetadataResponse {
                            user_id: scored_memory.memory.metadata.user_id,
                            agent_id: scored_memory.memory.metadata.agent_id,
                            run_id: scored_memory.memory.metadata.run_id,
                            actor_id: scored_memory.memory.metadata.actor_id,
                            role: scored_memory.memory.metadata.role,
                            memory_type: format!("{:?}", scored_memory.memory.metadata.memory_type),
                            hash: scored_memory.memory.metadata.hash,
                            custom: scored_memory.memory.metadata.custom,
                        },
                        created_at: scored_memory.memory.created_at.to_rfc3339(),
                        updated_at: scored_memory.memory.updated_at.to_rfc3339(),
                    },
                    score: scored_memory.score,
                })
                .collect();

            let response = SearchResponse {
                total: scored_memories.len(),
                results: scored_memories,
            };

            info!("Search completed: {} results found", response.total);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to search memories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to search memories: {}", e),
                    code: "MEMORY_SEARCH_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// List memories
pub async fn list_memories(
    State(state): State<AppState>,
    Query(query): Query<ListMemoryQuery>,
) -> Result<Json<ListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut filters = Filters::new();

    if let Some(user_id) = query.user_id {
        filters.user_id = Some(user_id);
    }

    if let Some(agent_id) = query.agent_id {
        filters.agent_id = Some(agent_id);
    }

    if let Some(run_id) = query.run_id {
        filters.run_id = Some(run_id);
    }

    if let Some(actor_id) = query.actor_id {
        filters.actor_id = Some(actor_id);
    }

    if let Some(memory_type_str) = query.memory_type {
        filters.memory_type = Some(MemoryType::parse(&memory_type_str));
    }

    let limit = query.limit;

    match state.memory_manager.list(&filters, limit).await {
        Ok(memories) => {
            let memory_responses: Vec<MemoryResponse> = memories
                .into_iter()
                .map(|memory| MemoryResponse {
                    id: memory.id,
                    content: memory.content,
                    metadata: MemoryMetadataResponse {
                        user_id: memory.metadata.user_id,
                        agent_id: memory.metadata.agent_id,
                        run_id: memory.metadata.run_id,
                        actor_id: memory.metadata.actor_id,
                        role: memory.metadata.role,
                        memory_type: format!("{:?}", memory.metadata.memory_type),
                        hash: memory.metadata.hash,
                        custom: memory.metadata.custom,
                    },
                    created_at: memory.created_at.to_rfc3339(),
                    updated_at: memory.updated_at.to_rfc3339(),
                })
                .collect();

            let response = ListResponse {
                total: memory_responses.len(),
                memories: memory_responses,
            };

            info!("List completed: {} memories found", response.total);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to list memories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to list memories: {}", e),
                    code: "MEMORY_LIST_FAILED".to_string(),
                }),
            ))
        }
    }
}




/// Batch delete memories
pub async fn batch_delete_memories(
    State(state): State<AppState>,
    Json(request): Json<BatchDeleteRequest>,
) -> Result<Json<BatchOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors = Vec::new();

    for memory_id in &request.ids {
        match state.memory_manager.delete(memory_id).await {
            Ok(()) => {
                success_count += 1;
                info!("Memory deleted in batch: {}", memory_id);
            }
            Err(e) => {
                failure_count += 1;
                let error_msg = format!("Failed to delete memory {}: {}", memory_id, e);
                error!("{}", error_msg);
                errors.push(error_msg);
            }
        }
    }

    let response = BatchOperationResponse {
        success_count,
        failure_count,
        errors,
        message: format!("Batch delete completed: {} succeeded, {} failed", success_count, failure_count),
    };

    if failure_count > 0 {
        Err((
            StatusCode::PARTIAL_CONTENT,
            Json(ErrorResponse {
                error: format!("Batch delete partially failed: {} errors", failure_count),
                code: "BATCH_DELETE_PARTIAL_FAILURE".to_string(),
            }),
        ))
    } else {
        Ok(Json(response))
    }
}

/// Batch update memories
pub async fn batch_update_memories(
    State(state): State<AppState>,
    Json(request): Json<BatchUpdateRequest>,
) -> Result<Json<BatchOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors = Vec::new();

    for update in &request.updates {
        match state.memory_manager.update(&update.id, update.content.clone()).await {
            Ok(()) => {
                success_count += 1;
                info!("Memory updated in batch: {}", update.id);
            }
            Err(e) => {
                failure_count += 1;
                let error_msg = format!("Failed to update memory {}: {}", update.id, e);
                error!("{}", error_msg);
                errors.push(error_msg);
            }
        }
    }

    let response = BatchOperationResponse {
        success_count,
        failure_count,
        errors,
        message: format!("Batch update completed: {} succeeded, {} failed", success_count, failure_count),
    };

    if failure_count > 0 {
        Err((
            StatusCode::PARTIAL_CONTENT,
            Json(ErrorResponse {
                error: format!("Batch update partially failed: {} errors", failure_count),
                code: "BATCH_UPDATE_PARTIAL_FAILURE".to_string(),
            }),
        ))
    } else {
        Ok(Json(response))
    }
}

/// Get detailed LLM service status including both completion and embedding models
pub async fn get_llm_status(
    State(state): State<AppState>,
) -> Result<Json<LLMStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let timestamp = Utc::now().to_rfc3339();
    
    // Check completion model (text generation)
    let completion_start = Instant::now();
    let (completion_available, completion_error) = match state
        .memory_manager
        .llm_client()
        .complete("Hello, this is a health check for the completion model.")
        .await
    {
        Ok(_) => (true, None),
        Err(e) => {
            error!("Completion model health check failed: {}", e);
            (false, Some(e.to_string()))
        }
    };
    let completion_latency = completion_start.elapsed().as_millis() as u64;

    // Check embedding model
    let embedding_start = Instant::now();
    let (embedding_available, embedding_error) = match state
        .memory_manager
        .llm_client()
        .embed("health check")
        .await
    {
        Ok(_) => (true, None),
        Err(e) => {
            error!("Embedding model health check failed: {}", e);
            (false, Some(e.to_string()))
        }
    };
    let embedding_latency = embedding_start.elapsed().as_millis() as u64;

    let overall_healthy = completion_available && embedding_available;
    let overall_status = if overall_healthy {
        "healthy".to_string()
    } else {
        "unhealthy".to_string()
    };

    let response = LLMStatusResponse {
        overall_status,
        completion_model: ModelStatus {
            available: completion_available,
            provider: "openai".to_string(),
            model_name: "completion-model".to_string(), // TODO: Get actual model name from config
            latency_ms: if completion_available { Some(completion_latency) } else { None },
            error_message: completion_error,
            last_check: timestamp.clone(),
        },
        embedding_model: ModelStatus {
            available: embedding_available,
            provider: "openai".to_string(),
            model_name: "embedding-model".to_string(), // TODO: Get actual model name from config
            latency_ms: if embedding_available { Some(embedding_latency) } else { None },
            error_message: embedding_error,
            last_check: timestamp.clone(),
        },
        timestamp,
    };

    Ok(Json(response))
}

/// Simple LLM health check endpoint
pub async fn llm_health_check(
    State(state): State<AppState>,
) -> Result<Json<LLMHealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Quick health check for both models
    let (completion_available, embedding_available) = tokio::join!(
        async {
            match state.memory_manager.llm_client().complete("Hi").await {
                Ok(_) => true,
                Err(_) => false,
            }
        },
        async {
            match state.memory_manager.llm_client().embed("Hi").await {
                Ok(_) => true,
                Err(_) => false,
            }
        }
    );

    let response = LLMHealthResponse {
        completion_model_available: completion_available,
        embedding_model_available: embedding_available,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(response))
}