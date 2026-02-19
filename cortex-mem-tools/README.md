# Cortex Memory Tools Library

`cortex-mem-tools` provides high-level abstractions and utilities for working with the Cortex Memory system. It offers simplified APIs for common operations and implements advanced automation features.

## 🛠️ Overview

Cortex Memory Tools implements:
- High-level `MemoryOperations` interface for unified access to Cortex Memory
- Advanced automation with event-driven processing
- Model Context Protocol (MCP) tool definitions
- Utility functions for memory management
- Type-safe error handling and comprehensive types

## 🏗️ Core Components

### MemoryOperations

The primary interface for working with Cortex Memory:

```rust
use cortex_mem_tools::MemoryOperations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create operations from data directory
    let ops = MemoryOperations::from_data_dir("./cortex-data").await?;
    
    // Add a message to a session
    let msg_id = ops.add_message(
        "tech-support",
        "user",
        "How do I reset my password?"
    ).await?;
    
    // Search memories
    let results = ops.search("password reset", None, 10).await?;
    for memory in results {
        println!("Found: {} (score: {:.2})", 
            memory.uri, 
            memory.score.unwrap_or(0.0)
        );
    }
    
    // Extract memories from session
    let memories = ops.extract_memories("tech-support").await?;
    println!("Extracted {} facts", memories.facts.len());
    
    Ok(())
}
```

### Automation System

Cortex Tools provides event-driven automation:

```rust
use cortex_mem_tools::MemoryOperations;
use cortex_mem_core::automation::{AutomationManager, SyncConfig, ExtractConfig};

// Configure automation
let sync_config = SyncConfig {
    auto_index: true,
    index_interval_secs: 300,
    extract_config: ExtractConfig {
        auto_extract: true,
        extract_interval_secs: 600,
        batch_size: 5,
    },
};

// Run automation
let automation = AutomationManager::new(filesystem.clone(), config, sync_config);
automation.run().await?;
```

### MCP Integration

The library provides tool definitions for Model Context Protocol:

```rust
use cortex_mem_tools::mcp::{get_mcp_tool_definitions, execute_mcp_tool};

// Get available tools
let tools = get_mcp_tool_definitions();
for tool in &tools {
    println!("Available tool: {}", tool.name);
}

// Execute a tool
let result = execute_mcp_tool(
    &tool_name,
    &args,
    &operations
).await?;
```

## 🚀 Advanced Usage

### Shared Component Management

```rust
use std::sync::Arc;
use cortex_mem_tools::MemoryOperations;
use cortex_mem_core::{CortexFilesystem, SessionManager, SessionConfig};
use tokio::sync::RwLock;

// Create shared components
let filesystem = Arc::new(CortexFilesystem::new("./cortex-data").await?);
filesystem.initialize().await?;

let session_manager = Arc::new(RwLock::new(
    SessionManager::new(filesystem.clone(), SessionConfig::default())
));

// Create operations interface
let ops = MemoryOperations::new(
    filesystem.clone(),
    session_manager.clone()
).await?;

// Share across threads
```

### Vector Search with Filters

```rust
use cortex_mem_tools::MemoryOperations;
use cortex_mem_core::search::{SearchOptions, SearchFilter};

let options = SearchOptions {
    limit: 10,
    min_score: 0.5,
    filter: SearchFilter {
        dimensions: vec!["user", "session"],
        tenants: vec!["tech-support"],
    },
};

let results = ops.search_with_options("password reset", options).await?;
```

### Custom Event Handling

```rust
use cortex_mem_core::events::{EventBus, CortexEvent};
use cortex_mem_tools::AutomationManager;

// Subscribe to events
let mut event_bus = EventBus::new();
let _receiver = event_bus.subscribe();

// Handle events
while let Some(event) = receiver.recv().await {
    match event {
        CortexEvent::FilesystemEvent(fs_event) => {
            // File changed - trigger re-indexing
        },
        CortexEvent::SessionEvent(session_event) => {
            // Session updated - trigger extraction
        },
        CortexEvent::SystemEvent(system_event) => {
            // Handle system events
        }
    }
}
```

## 🧱 Architecture

```
┌─────────────────────────────────────┐
│            Application Layer          │
├─────────────────────────────────────┤
│        MemoryOperations API          │
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

## 📦 Features

### Default Features

- **Core Operations**: Session management, message handling, search
- **Event System**: Event bus and listeners
- **Types**: Type definitions and error handling

### Optional Features

- **vector-search**: Enable Qdrant vector search integration
- **automation**: Enable event-driven automation
- **mcp**: Enable Model Context Protocol tools
- **embeddings**: Enable embedding generation

## 📚 API Reference

### MemoryOperations

```rust
impl MemoryOperations {
    // Creation
    pub async fn from_data_dir(path: &str) -> Result<Self>
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        session_manager: Arc<RwLock<SessionManager>>
    ) -> Result<Self>
    
    // Session Management
    pub async fn create_session(&self, thread_id: &str, title: Option<String>) -> Result<Session>
    pub async fn close_session(&self, thread_id: &str) -> Result<()>
    pub async fn list_sessions(&self) -> Result<Vec<Session>>
    
    // Message Operations
    pub async fn add_message(&self, thread_id: &str, role: &str, content: &str) -> Result<String>
    pub async fn get_message(&self, uri: &str) -> Result<Message>
    pub async fn delete_message(&self, uri: &str) Result<()>
    
    // Search and Retrieval
    pub async fn search(&self, query: &str, thread: Option<&str>, limit: usize) -> Result<Vec<SearchResult>>
    pub async fn search_with_options(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>>
    
    // Memory Extraction
    pub async fn extract_memories(&self, thread_id: &str) -> Result<ExtractedMemories>
    pub async fn extract_and_save(&self, thread_id: &str) -> Result<()>
    
    // File Operations
    pub async fn list_files(&self, uri_pattern: &str) -> Result<Vec<FileInfo>>
    pub async fn read_file(&self, uri: &str) -> Result<String>
    pub async fn write_file(&self, uri: &str, content: &str) -> Result<()>
}
```

### MCP Tools

The library provides these MCP tools:

| Tool Name | Description |
|-----------|-------------|
| `memory_search` | Search across memories |
| `session_create` | Create new session |
| `message_add` | Add message to session |
| `memory_extract` | Extract memories from session |
| `files_list` | List files in memory |

## 🔧 Configuration

Configuration is provided through the core configuration library:

```rust
use cortex_mem_tools::MemoryOperations;
use cortex_mem_config::Config;

let config = Config::from_file("config.toml")?;
let ops = MemoryOperations::from_config(config).await?;
```

## 🧪 Testing

Run tests with all features:

```bash
cargo test -p cortex-mem-tools --all-features
```

## 🔨 Development

### Adding New Operations

1. Open the corresponding core module
2. Create a new method in the appropriate module
3. Document the method comprehensively
4. Create a test file in `tests/` directory

### Adding New MCP Tools

1. Create a new module in `mcp/tools/`
2. Create a new method in `src/tools.rs`
3. Document the tool comprehensively
4. Create a test file in `tests/` directory

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

## 🔗 Dependencies

- `cortex-mem-core`: Core library
- `cortex-mem-config`: Configuration
- `async-trait`: For async traits
- `tokio`: Async runtime
- `serde`: Serialization

## 🌟 Examples

See the [`examples/`](examples/) directory for complete examples.

- Basic operations
- Automation setup
- MCP integration
- Custom tools

---