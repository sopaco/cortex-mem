use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    models::ApiResponse,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ExtractionRequest {
    #[serde(default)]
    auto_save: bool,
}

/// Trigger memory extraction for a session
pub async fn trigger_extraction(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
    Json(req): Json<ExtractionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    use cortex_mem_core::extraction::{ExtractionConfig, MemoryExtractor};

    // Check if LLM client is available
    let llm_client = state.llm_client.as_ref()
        .ok_or_else(|| AppError::BadRequest(
            "LLM client not configured. Set LLM_API_BASE_URL, LLM_API_KEY, and LLM_MODEL environment variables.".to_string()
        ))?;

    // Create extraction config
    let config = ExtractionConfig {
        extract_facts: true,
        extract_decisions: true,
        extract_entities: true,
        min_confidence: 0.5,
        max_messages_per_batch: 50,
    };

    // Create extractor
    let extractor = MemoryExtractor::new(state.filesystem.clone(), llm_client.clone(), config);

    // Get message storage
    let message_storage = cortex_mem_core::MessageStorage::new(state.filesystem.clone());

    // List all message URIs for the thread
    let message_uris = message_storage.list_messages(&thread_id).await?;

    // Load messages
    let mut messages = Vec::new();
    for uri in message_uris {
        if let Ok(msg) = message_storage.load_message(&uri).await {
            messages.push(msg);
        }
    }

    if messages.is_empty() {
        return Err(AppError::NotFound(format!(
            "No messages found in thread {}",
            thread_id
        )));
    }

    // Extract memories
    let extraction_result = extractor
        .extract_from_messages(&thread_id, &messages)
        .await?;

    // Optionally save to user/agent memories
    let entities_for_response = if req.auto_save {
        // 🔧 修复: 使用MemoryExtractor保存提取的记忆
        use cortex_mem_core::session::extraction::MemoryExtractor;
        
        // 从metadata获取user_id和agent_id，如果没有则使用默认值
        let user_id = "default".to_string();  // TODO: 从请求或session metadata获取
        let agent_id = "default".to_string();
        
        let memory_extractor = MemoryExtractor::new(
            llm_client.clone(),
            state.filesystem.clone(),
            user_id,
            agent_id,
        );
        
        // 转换extraction_result为ExtractedMemories格式
        use cortex_mem_core::session::extraction::{
            ExtractedMemories, EntityMemory,
        };
        
        // 先clone entities用于返回
        let entities_clone = extraction_result.entities.clone();
        
        let extracted_memories = ExtractedMemories {
            preferences: vec![],  // extraction_result不包含preferences
            entities: extraction_result.entities.into_iter().map(|e| {
                EntityMemory {
                    name: e.name.clone(),
                    entity_type: e.entity_type.clone(),
                    description: e.description.unwrap_or_else(|| e.name.clone()),
                    context: format!("Extracted from session {}", thread_id),
                }
            }).collect(),
            events: vec![],  // extraction_result不包含events
            cases: vec![],   // extraction_result不包含cases
            personal_info: vec![],
            work_history: vec![],
            relationships: vec![],
            goals: vec![],
        };
        
        if let Err(e) = memory_extractor.save_memories(&extracted_memories).await {
            tracing::warn!("Failed to auto-save memories: {}", e);
        } else {
            tracing::info!("Auto-saved {} entities to user/agent memories", extracted_memories.entities.len());
        }
        
        entities_clone
    } else {
        extraction_result.entities
    };

    let response = serde_json::json!({
        "thread_id": thread_id,
        "message_count": messages.len(),
        "facts": extraction_result.facts,
        "decisions": extraction_result.decisions,
        "entities": entities_for_response,
    });

    Ok(Json(ApiResponse::success(response)))
}

/// Trigger indexing for a specific thread
///
/// 🔧 Note: Manual indexing handlers are deprecated in favor of unified auto-indexing
/// CortexMem already handles automatic indexing when sessions are closed.
/// This endpoint is kept for backward compatibility and debugging purposes.
pub async fn trigger_indexing(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    use cortex_mem_core::{AutoIndexer, CortexFilesystem, IndexerConfig};

    // Check if embedding client is available
    let embedding_client = state
        .embedding_client
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Embedding service not configured.".to_string()))?;

    // 🆕 Create QdrantVectorStore (required for AutoIndexer)
    let qdrant_store = match state.create_qdrant_store().await {
        Ok(store) => Arc::new(store),
        Err(e) => {
            return Err(AppError::BadRequest(format!(
                "Failed to create Qdrant store: {}",
                e
            )));
        }
    };

    // 🆕 Create tenant-aware filesystem
    let filesystem = if let Some(tenant_root) = state.current_tenant_root.read().await.as_ref() {
        Arc::new(CortexFilesystem::new(
            tenant_root.to_string_lossy().as_ref(),
        ))
    } else {
        state.filesystem.clone()
    };

    // Create indexer
    let config = IndexerConfig {
        auto_index: true,
        batch_size: 10,
        async_index: false, // Synchronous for API call
    };

    let indexer = AutoIndexer::new(filesystem, embedding_client.clone(), qdrant_store, config);

    // Index the thread
    let stats = indexer.index_thread(&thread_id).await?;

    let response = serde_json::json!({
        "thread_id": thread_id,
        "indexed": stats.total_indexed,
        "skipped": stats.total_skipped,
        "errors": stats.total_errors,
        "note": "Manual indexing is deprecated. Cortex Memory handles automatic indexing when sessions are closed.",
    });

    Ok(Json(ApiResponse::success(response)))
}

/// Index all threads in the filesystem
///
/// 🔧 Note: Manual indexing handlers are deprecated in favor of unified auto-indexing
/// CortexMem already handles automatic indexing when sessions are closed.
/// This endpoint is kept for backward compatibility and debugging purposes.
pub async fn trigger_indexing_all(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    use cortex_mem_core::{AutoIndexer, CortexFilesystem, FilesystemOperations, IndexerConfig};

    // Check if embedding client is available
    let embedding_client = state
        .embedding_client
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Embedding service not configured.".to_string()))?;

    // 🆕 Create QdrantVectorStore (required for AutoIndexer)
    let qdrant_store = match state.create_qdrant_store().await {
        Ok(store) => Arc::new(store),
        Err(e) => {
            return Err(AppError::BadRequest(format!(
                "Failed to create Qdrant store: {}",
                e
            )));
        }
    };

    // 🆕 Create tenant-aware filesystem
    let filesystem = if let Some(tenant_root) = state.current_tenant_root.read().await.as_ref() {
        Arc::new(CortexFilesystem::new(
            tenant_root.to_string_lossy().as_ref(),
        ))
    } else {
        state.filesystem.clone()
    };

    // Create indexer
    let config = IndexerConfig {
        auto_index: true,
        batch_size: 10,
        async_index: false,
    };

    let indexer = AutoIndexer::new(
        filesystem.clone(),
        embedding_client.clone(),
        qdrant_store,
        config,
    );

    // List all threads
    let threads_uri = "cortex://session";
    let entries = filesystem.list(threads_uri).await?;

    let mut total_indexed = 0;
    let mut total_errors = 0;
    let mut total_skipped = 0;
    let mut threads_processed = 0;

    for entry in entries {
        if entry.is_directory && !entry.name.starts_with('.') {
            let thread_id = &entry.name;
            match indexer.index_thread(thread_id).await {
                Ok(stats) => {
                    total_indexed += stats.total_indexed;
                    total_skipped += stats.total_skipped;
                    total_errors += stats.total_errors;
                    threads_processed += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to index thread {}: {}", thread_id, e);
                    total_errors += 1;
                }
            }
        }
    }

    let response = serde_json::json!({
        "threads_processed": threads_processed,
        "total_indexed": total_indexed,
        "total_skipped": total_skipped,
        "total_errors": total_errors,
        "note": "Manual indexing is deprecated. CortexMem handles automatic indexing when sessions are closed.",
    });

    Ok(Json(ApiResponse::success(response)))
}
