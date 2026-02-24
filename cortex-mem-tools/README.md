# Cortex Memory Tools Library

`cortex-mem-tools` provides high-level abstractions and utilities for working with the Cortex Memory system. It offers simplified APIs for common operations with OpenViking-style tiered access (L0/L1/L2 layers).

## 🛠️ Overview

Cortex Memory Tools implements:
- High-level `MemoryOperations` interface for unified access to Cortex Memory
- **Tiered Access**: L0 (Abstract), L1 (Overview), L2 (Full Content)
- Advanced automation with event-driven processing
- Model Context Protocol (MCP) tool definitions
- Utility functions for memory management
- Type-safe error handling and comprehensive types

## 🏗️ Core Components

### MemoryOperations

The primary interface for working with Cortex Memory. It requires:
- LLM client for layer generation
- Vector search engine for semantic search
- Embedding client for vectorization

```rust
use cortex_mem_tools::MemoryOperations;
use cortex_mem_core::llm::{LLMClient, LLMClientImpl, LLMConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM client
    let llm_config = LLMConfig {
        api_base_url: "https://api.openai.com/v1".to_string(),
        api_key: "your-api-key".to_string(),
        model_efficient: "gpt-4o-mini".to_string(),
        temperature: 0.1,
        max_tokens: 4096,
    };
    let llm_client = Arc::new(LLMClientImpl::new(llm_config)?);
    
    // Create MemoryOperations with all dependencies
    let ops = MemoryOperations::new(
        "./cortex-data",           // data directory
        "default",                  // tenant ID
        llm_client,                 // LLM client
        "http://localhost:6333",    // Qdrant URL
        "cortex_memories",          // Qdrant collection
        "https://api.openai.com/v1", // Embedding API URL
        "your-embedding-key",       // Embedding API key
        "text-embedding-3-small",   // Embedding model
        Some(1536),                 // Embedding dimension
        None,                       // Optional user ID
    ).await?;
    
    // Add a message to a session
    let msg_id = ops.add_message(
        "tech-support",
        "user",
        "How do I reset my password?"
    ).await?;
    
    // Read file content (L2)
    let content = ops.read_file("cortex://session/tech-support/timeline/...").await?;
    
    // Get abstract (L0) - quick relevance check
    let abstract_result = ops.get_abstract("cortex://session/tech-support/...").await?;
    println!("Abstract: {}", abstract_result.abstract_text);
    
    // Get overview (L1) - partial context
    let overview = ops.get_overview("cortex://session/tech-support/...").await?;
    println!("Overview: {}", overview.overview_text);
    
    // List sessions
    let sessions = ops.list_sessions().await?;
    for session in sessions {
        println!("Session: {} ({})", session.thread_id, session.status);
    }
    
    Ok(())
}
```

### Tiered Access (OpenViking Style)

The library implements a three-tier access pattern for efficient memory retrieval:

| Layer | Size | Purpose | Method |
|-------|------|---------|--------|
| **L0 Abstract** | ~100 tokens | Quick relevance judgment | `get_abstract()` |
| **L1 Overview** | ~2000 tokens | Partial context understanding | `get_overview()` |
| **L2 Full** | Complete content | Deep analysis and processing | `read_file()` / `get_read()` |

### Automation System

MemoryOperations includes built-in automation:

- **Auto Indexing**: Messages are automatically indexed for vector search
- **Auto Extraction**: Memories are extracted when sessions are closed
- **Event-Driven**: Uses EventBus for real-time processing

```rust
// Automation is built into MemoryOperations::new()
// No additional configuration needed

// Access the auto extractor for manual extraction
if let Some(extractor) = ops.auto_extractor() {
    extractor.extract_session("tech-support").await?;
}
```

### MCP Integration

The library provides tool definitions for Model Context Protocol:

```rust
use cortex_mem_tools::mcp::{get_mcp_tool_definitions, get_mcp_tool_definition};

// Get all available MCP tool definitions
let tools = get_mcp_tool_definitions();
for tool in &tools {
    println!("Available tool: {} - {}", tool.name, tool.description);
}

// Get a specific tool definition
if let Some(tool) = get_mcp_tool_definition("search") {
    println!("Search tool: {:?}", tool.input_schema);
}
```

Available MCP tools:
- `abstract` - Get L0 abstract
- `overview` - Get L1 overview
- `read` - Get L2 full content
- `search` - Intelligent search with multiple engines
- `find` - Quick search returning L0 abstracts
- `ls` - List directory contents
- `explore` - Intelligently explore memory space
- `store` - Store content with automatic layer generation

## 📚 API Reference

### MemoryOperations

```rust
impl MemoryOperations {
    /// Create with full dependencies (primary constructor)
    pub async fn new(
        data_dir: &str,
        tenant_id: impl Into<String>,
        llm_client: Arc<dyn LLMClient>,
        qdrant_url: &str,
        qdrant_collection: &str,
        embedding_api_base_url: &str,
        embedding_api_key: &str,
        embedding_model_name: &str,
        embedding_dim: Option<usize>,
        user_id: Option<String>,
    ) -> Result<Self>
    
    // Accessors
    pub fn filesystem(&self) -> &Arc<CortexFilesystem>
    pub fn vector_engine(&self) -> &Arc<VectorSearchEngine>
    pub fn session_manager(&self) -> &Arc<RwLock<SessionManager>>
    pub fn auto_extractor(&self) -> Option<&Arc<AutoExtractor>>
    
    // Session Management
    pub async fn add_message(&self, thread_id: &str, role: &str, content: &str) -> Result<String>
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>>
    pub async fn get_session(&self, thread_id: &str) -> Result<SessionInfo>
    pub async fn close_session(&self, thread_id: &str) -> Result<()>
    
    // Tiered Access (L0/L1/L2)
    pub async fn get_abstract(&self, uri: &str) -> Result<AbstractResponse>
    pub async fn get_overview(&self, uri: &str) -> Result<OverviewResponse>
    pub async fn get_read(&self, uri: &str) -> Result<ReadResponse>
    
    // File Operations
    pub async fn read_file(&self, uri: &str) -> Result<String>
    pub async fn list_files(&self, uri: &str) -> Result<Vec<String>>
    pub async fn delete(&self, uri: &str) -> Result<()>
    pub async fn exists(&self, uri: &str) -> Result<bool>
    
    // Tool-based Operations (using typed args)
    pub async fn search(&self, args: SearchArgs) -> Result<SearchResponse>
    pub async fn find(&self, args: FindArgs) -> Result<FindResponse>
    pub async fn ls(&self, args: LsArgs) -> Result<LsResponse>
    pub async fn explore(&self, args: ExploreArgs) -> Result<ExploreResponse>
    pub async fn store(&self, args: StoreArgs) -> Result<StoreResponse>
}
```

### Type Definitions

```rust
// Tiered access responses
pub struct AbstractResponse {
    pub uri: String,
    pub abstract_text: String,
    pub layer: String,  // "L0"
    pub token_count: usize,
}

pub struct OverviewResponse {
    pub uri: String,
    pub overview_text: String,
    pub layer: String,  // "L1"
    pub token_count: usize,
}

pub struct ReadResponse {
    pub uri: String,
    pub content: String,
    pub layer: String,  // "L2"
    pub token_count: usize,
    pub metadata: Option<FileMetadata>,
}

// Search types
pub struct SearchArgs {
    pub query: String,
    pub recursive: Option<bool>,
    pub return_layers: Option<Vec<String>>,  // ["L0", "L1", "L2"]
    pub scope: Option<String>,
    pub limit: Option<usize>,
}

pub struct StoreArgs {
    pub content: String,
    pub thread_id: String,
    pub metadata: Option<Value>,
    pub auto_generate_layers: Option<bool>,
    pub scope: String,  // "session", "user", or "agent"
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
}
```

## 🧱 Architecture

```
┌─────────────────────────────────────┐
│            Application Layer          │
├─────────────────────────────────────┤
│        MemoryOperations API          │
│    ┌──────────┬──────────┬────────┐  │
│    │ L0:Abs   │ L1:Over  │ L2:Full│  │
│    └──────────┴──────────┴────────┘  │
├─────────────────────────────────────┤
│         Automation Layer             │
│    ┌─────────────┬────────────────┐  │
│    │ AutoIndexer │  AutoExtractor │  │
│    └─────────────┴────────────────┘  │
├─────────────────────────────────────┤
│         Event System                 │
│    ┌─────────────┬────────────────┐  │
│    │  EventBus   │ EventListeners │  │
│    └─────────────┴────────────────┘  │
├─────────────────────────────────────┤
│          Core Layer                  │
│  CortexFilesystem│SessionManager    │
│  LayerManager   │VectorSearchEngine│
└─────────────────────────────────────┘
```

## 📦 Dependencies

This crate depends on:
- `cortex-mem-core` - Core library with all memory operations
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `anyhow` / `thiserror` - Error handling
- `tracing` - Logging
- `chrono` - Date/time handling
- `uuid` - Unique identifiers
- `async-trait` - Async trait support

## 🧪 Testing

Run tests with all features:

```bash
cargo test -p cortex-mem-tools
```

## 🔍 Error Handling

All operations return `Result<T, ToolsError>` where `ToolsError` includes:

- Configuration errors
- Filesystem errors
- Session errors
- Search errors
- Automation errors
- MCP tool errors

## 📄 License

MIT License - see the [`LICENSE`](../../LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add comprehensive tests
4. Submit a pull request

## 🔗 Related Crates

- [`cortex-mem-core`](../cortex-mem-core/) - Core library
- [`cortex-mem-mcp`](../cortex-mem-mcp/) - MCP server
- [`cortex-mem-rig`](../cortex-mem-rig/) - Rig integration
- [`cortex-mem-service`](../cortex-mem-service/) - HTTP REST API

---
