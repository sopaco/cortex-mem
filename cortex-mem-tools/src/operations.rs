use crate::errors::{MemoryToolsError, MemoryToolsResult};
use crate::types::{
    MemoryOperationPayload, MemoryOperationResponse, QueryParams,
    StoreParams, FilterParams
};
use cortex_mem_core::{
    memory::MemoryManager, Memory, MemoryType, MemoryMetadata
};
use serde_json::{json, Value};
use tracing::{error, info};

/// Core operations handler for memory tools
pub struct MemoryOperations {
    memory_manager: std::sync::Arc<MemoryManager>,
    default_user_id: Option<String>,
    default_agent_id: Option<String>,
    default_limit: usize,
}

impl MemoryOperations {
    /// Create a new MemoryOperations instance
    pub fn new(
        memory_manager: std::sync::Arc<MemoryManager>,
        default_user_id: Option<String>,
        default_agent_id: Option<String>,
        default_limit: usize,
    ) -> Self {
        Self {
            memory_manager,
            default_user_id,
            default_agent_id,
            default_limit,
        }
    }

    /// Store a new memory
    pub async fn store_memory(&self, payload: MemoryOperationPayload) -> MemoryToolsResult<MemoryOperationResponse> {
        let params = StoreParams::from_payload(
            &payload,
            self.default_user_id.clone(),
            self.default_agent_id.clone(),
        )?;

        info!("Storing memory for user: {}", params.user_id);

        let memory_type = MemoryType::parse_with_result(&params.memory_type)
            .map_err(|e| MemoryToolsError::InvalidInput(format!("Invalid memory_type: {}", e)))?;

        let mut metadata = MemoryMetadata::new(memory_type);
        metadata.user_id = Some(params.user_id.clone());
        metadata.agent_id = params.agent_id.clone();

        if let Some(topics) = params.topics {
            metadata.topics = topics;
        }

        match self.memory_manager.store(params.content, metadata).await {
            Ok(memory_id) => {
                info!("Memory stored successfully with ID: {}", memory_id);
                let data = json!({
                    "memory_id": memory_id,
                    "user_id": params.user_id,
                    "agent_id": params.agent_id
                });

                Ok(MemoryOperationResponse::success_with_data(
                    "Memory stored successfully",
                    data,
                ))
            }
            Err(e) => {
                error!("Failed to store memory: {}", e);
                Err(MemoryToolsError::Runtime(format!("Failed to store memory: {}", e)))
            }
        }
    }

    /// Query memories based on semantic similarity
    pub async fn query_memory(&self, payload: MemoryOperationPayload) -> MemoryToolsResult<MemoryOperationResponse> {
        let params = QueryParams::from_payload(&payload, self.default_limit)?;

        info!("Querying memories with query: {}", params.query);

        let memory_type = params.memory_type
            .map(|t| MemoryType::parse_with_result(&t))
            .transpose()
            .map_err(|e| MemoryToolsError::InvalidInput(format!("Invalid memory_type: {}", e)))?;

        // Convert parameters to Filters
        let mut filters = cortex_mem_core::types::Filters::default();

        if let Some(user_id) = params.user_id {
            filters.user_id = Some(user_id);
        }

        if let Some(agent_id) = params.agent_id {
            filters.agent_id = Some(agent_id);
        }

        if let Some(memory_type) = memory_type {
            filters.memory_type = Some(memory_type);
        }

        if let Some(topics) = params.topics {
            filters.topics = Some(topics);
        }

        // Apply time range filters
        if let Some(created_after) = params.created_after {
            filters.created_after = Some(created_after);
        }

        if let Some(created_before) = params.created_before {
            filters.created_before = Some(created_before);
        }

        match self.memory_manager.search(
            &params.query,
            &filters,
            params.limit,
        ).await {
            Ok(memories) => {
                let count = memories.len();
                info!("Found {} memories", count);

                let memories_json: Vec<Value> = memories
                    .into_iter()
                    .map(|scored_memory| memory_to_json(&scored_memory.memory))
                    .collect();

                let data = json!({
                    "count": count,
                    "memories": memories_json
                });

                Ok(MemoryOperationResponse::success_with_data(
                    "Query completed successfully",
                    data,
                ))
            }
            Err(e) => {
                error!("Failed to query memories: {}", e);
                Err(MemoryToolsError::Runtime(format!("Failed to query memories: {}", e)))
            }
        }
    }

    /// List memories with filtering
    pub async fn list_memories(&self, payload: MemoryOperationPayload) -> MemoryToolsResult<MemoryOperationResponse> {
        let params = FilterParams::from_payload(&payload, self.default_limit)?;

        info!("Listing memories with filters");

        // Convert parameters to Filters
        let mut filters = cortex_mem_core::types::Filters::default();

        if let Some(user_id) = params.user_id {
            filters.user_id = Some(user_id);
        }

        if let Some(agent_id) = params.agent_id {
            filters.agent_id = Some(agent_id);
        }

        if let Some(memory_type) = params.memory_type {
            if let Ok(mt) = cortex_mem_core::types::MemoryType::parse_with_result(&memory_type) {
                filters.memory_type = Some(mt);
            }
        }

        // Apply time range filters
        if let Some(created_after) = params.created_after {
            filters.created_after = Some(created_after);
        }

        if let Some(created_before) = params.created_before {
            filters.created_before = Some(created_before);
        }

        match self.memory_manager.list(&filters, Some(params.limit)).await {
            Ok(memories) => {
                let count = memories.len();
                info!("Listed {} memories", count);

                let memories_json: Vec<Value> = memories
                    .into_iter()
                    .map(|memory| memory_to_json(&memory))
                    .collect();

                let data = json!({
                    "count": count,
                    "memories": memories_json
                });

                Ok(MemoryOperationResponse::success_with_data(
                    "List completed successfully",
                    data,
                ))
            }
            Err(e) => {
                error!("Failed to list memories: {}", e);
                Err(MemoryToolsError::Runtime(format!("Failed to list memories: {}", e)))
            }
        }
    }

    /// Get a specific memory by ID
    pub async fn get_memory(&self, payload: MemoryOperationPayload) -> MemoryToolsResult<MemoryOperationResponse> {
        let memory_id = payload.memory_id
            .ok_or_else(|| MemoryToolsError::InvalidInput("Memory ID is required".to_string()))?;

        info!("Getting memory with ID: {}", memory_id);

        match self.memory_manager.get(&memory_id).await {
            Ok(Some(memory)) => {
                let memory_json = memory_to_json(&memory);
                let data = json!({
                    "memory": memory_json
                });

                Ok(MemoryOperationResponse::success_with_data(
                    "Memory retrieved successfully",
                    data,
                ))
            }
            Ok(None) => {
                error!("Memory not found: {}", memory_id);
                Err(MemoryToolsError::MemoryNotFound(memory_id))
            }
            Err(e) => {
                error!("Failed to get memory: {}", e);
                Err(MemoryToolsError::Runtime(format!("Failed to get memory: {}", e)))
            }
        }
    }
}

/// Convert a Memory object to JSON
fn memory_to_json(memory: &Memory) -> Value {
    let mut metadata_obj = json!({});

    if let Some(user_id) = &memory.metadata.user_id {
        metadata_obj["user_id"] = Value::String(user_id.clone());
    }

    if let Some(agent_id) = &memory.metadata.agent_id {
        metadata_obj["agent_id"] = Value::String(agent_id.clone());
    }

    if let Some(run_id) = &memory.metadata.run_id {
        metadata_obj["run_id"] = Value::String(run_id.clone());
    }

    if let Some(actor_id) = &memory.metadata.actor_id {
        metadata_obj["actor_id"] = Value::String(actor_id.clone());
    }

    if let Some(role) = &memory.metadata.role {
        metadata_obj["role"] = Value::String(role.clone());
    }

    metadata_obj["memory_type"] = Value::String(format!("{:?}", memory.metadata.memory_type));

    metadata_obj["hash"] = Value::String(memory.metadata.hash.clone());

    metadata_obj["importance_score"] = Value::Number(serde_json::Number::from_f64(memory.metadata.importance_score as f64).unwrap());

    if !memory.metadata.entities.is_empty() {
        metadata_obj["entities"] = Value::Array(
            memory.metadata.entities.iter()
                .map(|e| Value::String(e.clone()))
                .collect()
        );
    }

    if !memory.metadata.topics.is_empty() {
        metadata_obj["topics"] = Value::Array(
            memory.metadata.topics.iter()
                .map(|t| Value::String(t.clone()))
                .collect()
        );
    }

    if !memory.metadata.custom.is_empty() {
        metadata_obj["custom"] = Value::Object(
            memory.metadata.custom.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        );
    }

    json!({
        "id": memory.id,
        "content": memory.content,
        "created_at": memory.created_at.to_rfc3339(),
        "updated_at": memory.updated_at.to_rfc3339(),
        "metadata": metadata_obj
    })
}
