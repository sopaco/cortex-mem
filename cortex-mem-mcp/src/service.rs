use anyhow::Result;
use cortex_mem_core::{layers::manager::LayerManager, *};
use rmcp::{
    handler::server::tool::ToolRouter, handler::server::wrapper::Parameters, model::*, tool,
    tool_handler, tool_router, Json, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// MCP Service for Cortex Memory
#[derive(Clone)]
pub struct MemoryMcpService {
    filesystem: Arc<CortexFilesystem>,
    _default_agent_id: Option<String>,
    _default_user_id: Option<String>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StoreMemoryArgs {
    /// Content to store
    content: String,
    /// Optional user dimension
    user_dimension: Option<String>,
    /// Optional repository dimension
    repos_dimension: Option<String>,
    /// Optional tags for categorization
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StoreMemoryResult {
    success: bool,
    uri: String,
    memory_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct QueryMemoryArgs {
    /// Search query
    query: String,
    /// Optional user dimension to filter
    user_dimension: Option<String>,
    /// Optional repository dimension to filter
    repos_dimension: Option<String>,
    /// Maximum number of results
    limit: Option<usize>,
    /// Search mode: "keyword" (default), "vector", "hybrid", "layered"
    #[cfg(feature = "vector-search")]
    mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct QueryMemoryResult {
    success: bool,
    query: String,
    results: Vec<String>,
    total: usize,
    message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListMemoriesArgs {
    /// User dimension to list
    user_dimension: Option<String>,
    /// Repository dimension to list
    repos_dimension: Option<String>,
    /// Maximum number of results
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListMemoriesResult {
    success: bool,
    memories: Vec<String>,
    total: usize,
    message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetMemoryArgs {
    /// URI of the memory to retrieve
    uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetMemoryResult {
    success: bool,
    uri: String,
    content: String,
}

#[tool_router]
impl MemoryMcpService {
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        default_agent_id: Option<String>,
        default_user_id: Option<String>,
    ) -> Self {
        Self {
            filesystem,
            _default_agent_id: default_agent_id,
            _default_user_id: default_user_id,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Store a new memory in the cortex memory system")]
    async fn store_memory(
        &self,
        params: Parameters<StoreMemoryArgs>,
    ) -> Result<Json<StoreMemoryResult>, String> {
        debug!("store_memory called with args: {:?}", params.0);

        // Generate a unique ID for this memory
        let memory_id = uuid::Uuid::new_v4().to_string();

        // Construct URI based on dimensions
        let uri = if let Some(user) = &params.0.user_dimension {
            format!("cortex://user/{}/memories/{}", user, memory_id)
        } else if let Some(repos) = &params.0.repos_dimension {
            format!("cortex://repos/{}/memories/{}", repos, memory_id)
        } else {
            format!("cortex://memories/{}", memory_id)
        };

        // Write the memory content
        if let Err(e) = self.filesystem.write(&uri, &params.0.content).await {
            return Err(format!("Failed to store memory: {}", e));
        }

        info!("Memory stored at: {}", uri);

        Ok(Json(StoreMemoryResult {
            success: true,
            uri,
            memory_id,
        }))
    }

    #[tool(description = "Search and retrieve memories using semantic search")]
    async fn query_memory(
        &self,
        params: Parameters<QueryMemoryArgs>,
    ) -> Result<Json<QueryMemoryResult>, String> {
        debug!("query_memory called with args: {:?}", params.0);

        let limit = params.0.limit.unwrap_or(10);
        let query = &params.0.query;

        // Build search scope based on dimensions
        let scope = if let Some(ref repos) = params.0.repos_dimension {
            if let Some(ref user) = params.0.user_dimension {
                format!("cortex://agents/{}/users/{}/threads", repos, user)
            } else {
                format!("cortex://agents/{}/threads", repos)
            }
        } else if let Some(ref user) = params.0.user_dimension {
            format!("cortex://users/{}/threads", user)
        } else {
            "cortex://threads".to_string()
        };

        // Determine search mode
        #[cfg(feature = "vector-search")]
        let mode = params.0.mode.as_deref().unwrap_or("keyword");
        #[cfg(not(feature = "vector-search"))]
        let mode = "keyword";

        match mode {
            "keyword" => {
                // Use RetrievalEngine for keyword search
                let layer_manager = Arc::new(LayerManager::new(self.filesystem.clone()));
                let engine =
                    cortex_mem_core::RetrievalEngine::new(self.filesystem.clone(), layer_manager);

                let options = cortex_mem_core::RetrievalOptions {
                    top_k: limit,
                    min_score: 0.3,
                    load_details: true,
                    max_candidates: limit * 2,
                };

                match engine.search(query, &scope, options).await {
                    Ok(result) => {
                        let results: Vec<String> = result
                            .results
                            .iter()
                            .map(|r| format!("{}: {}", r.uri, r.snippet))
                            .collect();

                        let total = results.len();
                        info!(
                            "Keyword query '{}' found {} results in scope {}",
                            query, total, scope
                        );

                        Ok(Json(QueryMemoryResult {
                            success: true,
                            query: query.clone(),
                            results,
                            total,
                            message: format!("Found {} memories (keyword mode)", total),
                        }))
                    }
                    Err(e) => {
                        error!("Keyword query failed: {}", e);
                        Err(format!("Keyword search failed: {}", e))
                    }
                }
            }

            #[cfg(feature = "vector-search")]
            "vector" | "hybrid" | "layered" => {
                // For vector-based modes, we would need VectorSearchEngine
                // Since MCP service doesn't have vector_engine initialized by default,
                // we return an error message
                Err(format!(
                    "Vector search mode '{}' requires MCP service to be initialized with vector-search support. \
                    Please use MemoryMcpService::with_vector() constructor or fall back to 'keyword' mode.",
                    mode
                ))
            }

            _ => Err(format!(
                "Unknown search mode '{}'. Supported modes: keyword{}",
                mode,
                if cfg!(feature = "vector-search") {
                    ", vector, hybrid, layered"
                } else {
                    ""
                }
            )),
        }
    }

    #[tool(description = "List memories from a specific dimension")]
    async fn list_memories(
        &self,
        params: Parameters<ListMemoriesArgs>,
    ) -> Result<Json<ListMemoriesResult>, String> {
        debug!("list_memories called with args: {:?}", params.0);

        let limit = params.0.limit.unwrap_or(50);

        // Build scope based on dimensions
        let scope = if let Some(ref repos) = params.0.repos_dimension {
            if let Some(ref user) = params.0.user_dimension {
                format!("cortex://agents/{}/users/{}/threads", repos, user)
            } else {
                format!("cortex://agents/{}/threads", repos)
            }
        } else if let Some(ref user) = params.0.user_dimension {
            format!("cortex://users/{}/threads", user)
        } else {
            "cortex://threads".to_string()
        };

        // List files in the scope
        match self.filesystem.list(&scope).await {
            Ok(entries) => {
                let mut memories = Vec::new();

                for entry in entries {
                    // Skip hidden files except .abstract.md and .overview.md
                    if entry.name.starts_with('.')
                        && entry.name != ".abstract.md"
                        && entry.name != ".overview.md"
                    {
                        continue;
                    }

                    if entry.is_directory {
                        memories.push(format!("📁 {}/", entry.name));
                    } else {
                        memories.push(format!("📄 {} ({} bytes)", entry.name, entry.size));
                    }

                    if memories.len() >= limit {
                        break;
                    }
                }

                let total = memories.len();
                info!("Listed {} memories in scope {}", total, scope);

                Ok(Json(ListMemoriesResult {
                    success: true,
                    memories,
                    total,
                    message: format!("Found {} items", total),
                }))
            }
            Err(e) => {
                error!("List failed: {}", e);
                Err(format!("Failed to list memories: {}", e))
            }
        }
    }

    #[tool(description = "Retrieve a specific memory by its URI")]
    async fn get_memory(
        &self,
        params: Parameters<GetMemoryArgs>,
    ) -> Result<Json<GetMemoryResult>, String> {
        debug!("get_memory called with args: {:?}", params.0);

        // Read the memory content
        match self.filesystem.read(&params.0.uri).await {
            Ok(content) => {
                info!("Memory retrieved from: {}", params.0.uri);
                Ok(Json(GetMemoryResult {
                    success: true,
                    uri: params.0.uri.clone(),
                    content,
                }))
            }
            Err(e) => Err(format!("Failed to get memory: {}", e)),
        }
    }
}

#[tool_handler]
impl ServerHandler for MemoryMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Cortex Memory MCP Server - Provides memory management tools for AI assistants. \
                Supports storing, querying, listing, and retrieving memories with dimension support."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
