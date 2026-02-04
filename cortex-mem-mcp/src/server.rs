use crate::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, MCPConfig};
use cortex_mem_core::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub struct MCPServer {
    config: MCPConfig,
    filesystem: Arc<CortexFilesystem>,
}

impl MCPServer {
    pub fn new(config: MCPConfig, filesystem: Arc<CortexFilesystem>) -> Self {
        Self { config, filesystem }
    }

    pub fn config(&self) -> &MCPConfig {
        &self.config
    }

    pub async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tools_call(request).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": self.config.server_name,
                    "version": self.config.server_version
                }
            })),
            error: None,
        }
    }

    async fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = vec![
            json!({
                "name": "store_memory",
                "description": "Store a new memory in the cortex memory system",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The memory content to store"
                        },
                        "thread_id": {
                            "type": "string",
                            "description": "Optional thread/session ID to associate the memory with"
                        },
                        "role": {
                            "type": "string",
                            "enum": ["user", "assistant", "system"],
                            "description": "Message role (default: user)",
                            "default": "user"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Optional tags for categorization"
                        }
                    },
                    "required": ["content"]
                }
            }),
            json!({
                "name": "query_memory",
                "description": "Search and retrieve memories using semantic search",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "mode": {
                            "type": "string",
                            "description": "Search mode: 'filesystem' (default), 'vector', or 'hybrid'",
                            "enum": ["filesystem", "vector", "hybrid"],
                            "default": "filesystem"
                        },
                        "thread_id": {
                            "type": "string",
                            "description": "Optional thread ID to limit search scope"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of results (default: 10)",
                            "default": 10
                        },
                        "min_score": {
                            "type": "number",
                            "description": "Minimum relevance score 0-1 (default: 0.3)",
                            "default": 0.3
                        }
                    },
                    "required": ["query"]
                }
            }),
            json!({
                "name": "list_memories",
                "description": "List memories from a specific thread or dimension",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "thread_id": {
                            "type": "string",
                            "description": "Thread ID to list memories from"
                        },
                        "dimension": {
                            "type": "string",
                            "enum": ["threads", "agents", "users", "global"],
                            "description": "Dimension to list from (default: threads)"
                        },
                        "include_metadata": {
                            "type": "boolean",
                            "description": "Include metadata in results (default: false)",
                            "default": false
                        }
                    }
                }
            }),
            json!({
                "name": "get_memory",
                "description": "Retrieve a specific memory by URI",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "Memory URI (e.g., cortex://threads/my-session/timeline/...)"
                        }
                    },
                    "required": ["uri"]
                }
            }),
        ];

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "tools": tools
            })),
            error: None,
        }
    }

    async fn handle_tools_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match request.params.as_ref() {
            Some(p) => p,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Invalid params".to_string(),
                        data: None,
                    }),
                };
            }
        };

        #[derive(Deserialize)]
        struct ToolCallParams {
            name: String,
            arguments: serde_json::Value,
        }

        let tool_call: ToolCallParams = match serde_json::from_value(params.clone()) {
            Ok(tc) => tc,
            Err(e) => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid tool call params: {}", e),
                        data: None,
                    }),
                };
            }
        };

        let result = match tool_call.name.as_str() {
            "store_memory" => self.tool_store_memory(tool_call.arguments).await,
            "query_memory" => self.tool_query_memory(tool_call.arguments).await,
            "list_memories" => self.tool_list_memories(tool_call.arguments).await,
            "get_memory" => self.tool_get_memory(tool_call.arguments).await,
            _ => Err(format!("Unknown tool: {}", tool_call.name)),
        };

        match result {
            Ok(content) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": content
                        }
                    ]
                })),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: format!("Tool execution error: {}", e),
                    data: None,
                }),
            },
        }
    }

    async fn tool_store_memory(&self, args: serde_json::Value) -> std::result::Result<String, String> {
        #[derive(Deserialize)]
        struct StoreMemoryArgs {
            content: String,
            thread_id: Option<String>,
            role: Option<String>,
            #[allow(dead_code)]
            tags: Option<Vec<String>>,
        }

        let args: StoreMemoryArgs = serde_json::from_value(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let thread_id = args.thread_id.unwrap_or_else(|| "default".to_string());
        
        let role = match args.role.as_deref().unwrap_or("user") {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => MessageRole::User,
        };

        let message = Message::new(role, &args.content);
        let storage = MessageStorage::new(self.filesystem.clone());
        
        let uri = storage
            .save_message(&thread_id, &message)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Memory stored successfully\nURI: {}\nID: {}", uri, message.id))
    }

    async fn tool_query_memory(&self, args: serde_json::Value) -> std::result::Result<String, String> {
        #[derive(Deserialize)]
        struct QueryMemoryArgs {
            query: String,
            #[serde(default)]
            mode: SearchMode,
            thread_id: Option<String>,
            limit: Option<usize>,
            min_score: Option<f32>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum SearchMode {
            Filesystem,
            #[cfg(feature = "vector-search")]
            Vector,
            #[cfg(feature = "vector-search")]
            Hybrid,
        }

        impl Default for SearchMode {
            fn default() -> Self {
                SearchMode::Filesystem
            }
        }

        let args: QueryMemoryArgs = serde_json::from_value(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        // Log the mode being used
        match args.mode {
            SearchMode::Filesystem => tracing::info!("Using filesystem search mode"),
            #[cfg(feature = "vector-search")]
            SearchMode::Vector => tracing::info!("Using vector search mode"),
            #[cfg(feature = "vector-search")]
            SearchMode::Hybrid => tracing::info!("Using hybrid search mode"),
        }

        let layer_manager = Arc::new(LayerManager::new(self.filesystem.clone()));
        let engine = RetrievalEngine::new(self.filesystem.clone(), layer_manager);

        let scope = if let Some(thread_id) = args.thread_id {
            format!("cortex://threads/{}", thread_id)
        } else {
            "cortex://threads".to_string()
        };

        let mut options = RetrievalOptions::default();
        options.top_k = args.limit.unwrap_or(10);
        options.min_score = args.min_score.unwrap_or(0.3);

        let result = engine
            .search(&args.query, &scope, options)
            .await
            .map_err(|e| e.to_string())?;

        if result.results.is_empty() {
            return Ok("No memories found".to_string());
        }

        let mut output = format!("Found {} memories:\n\n", result.results.len());
        
        for (i, candidate) in result.results.iter().enumerate() {
            output.push_str(&format!(
                "{}. {} (score: {:.2})\n",
                i + 1,
                candidate.uri,
                candidate.score
            ));
            
            output.push_str(&format!("   {}\n", candidate.snippet));
            output.push('\n');
        }

        Ok(output)
    }

    async fn tool_list_memories(&self, args: serde_json::Value) -> std::result::Result<String, String> {
        #[derive(Deserialize)]
        struct ListMemoriesArgs {
            thread_id: Option<String>,
            dimension: Option<String>,
            #[allow(dead_code)]
            include_metadata: Option<bool>,
        }

        let args: ListMemoriesArgs = serde_json::from_value(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let uri = if let Some(thread_id) = args.thread_id {
            format!("cortex://threads/{}", thread_id)
        } else {
            let dimension = args.dimension.unwrap_or_else(|| "threads".to_string());
            format!("cortex://{}", dimension)
        };

        let entries = self
            .filesystem
            .list(&uri)
            .await
            .map_err(|e| e.to_string())?;

        if entries.is_empty() {
            return Ok("No memories found".to_string());
        }

        let mut output = format!("Found {} items:\n\n", entries.len());

        for entry in entries {
            if entry.name.starts_with('.') && entry.name != ".abstract.md" && entry.name != ".overview.md" {
                continue;
            }

            if entry.is_directory {
                output.push_str(&format!("📁 {}/\n", entry.name));
            } else {
                output.push_str(&format!("📄 {} ({} bytes)\n", entry.name, entry.size));
            }
        }

        Ok(output)
    }

    async fn tool_get_memory(&self, args: serde_json::Value) -> std::result::Result<String, String> {
        #[derive(Deserialize)]
        struct GetMemoryArgs {
            uri: String,
        }

        let args: GetMemoryArgs = serde_json::from_value(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let content = self
            .filesystem
            .read(&args.uri)
            .await
            .map_err(|e| e.to_string())?;

        Ok(content)
    }
}
