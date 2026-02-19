# Cortex Memory Core Core Library

`cortex-mem-core` is the foundational library of the Cortex Memory system, providing core services and abstractions for AI agent memory management.

## 🧠 Overview

Cortex Memory Core implements:
- A virtual filesystem with `cortex://` URI scheme for memory storage
- Three-tier memory architecture (L0/L1/L3 layers)
- Session-based conversational memory management
- Vector search integration with Qdrant
- Automated memory extraction and profiling
- Event-driven automation system

## 🏗️ Architecture

### Core Modules

| Module | Purpose | Key Components |
|--------|---------|----------------|
| **`filesystem`** | Virtual file system with custom URI scheme | `CortexFilesystem`, `CortexUri`, `FilesystemOperations` |
| **`session`** | Conversational session management | `SessionManager`, `Message`, `TimelineGenerator` |
| **`vector_store`** | Vector database abstraction | `VectorStore`, embedding integration |
| **`search`** | Semantic and hybrid search engines | `VectorSearchEngine`, `SearchOptions` |
| **`extraction`** | Memory extraction and profiling | `MemoryExtractor`, `ExtractedMemories` |
| **`automation`** | Event-driven automation | `AutomationManager`, `CortexEvent` |
| **`layers`** | Three-tier memory architecture | `ContextLayer`, layer management |
| **`llm`** | Large language model abstraction | `LlmClient`, LLM providers |
| **`events`** | Event system for automation | `CortexEvent`, event handling |

## 🚀 Quick Start

### Basic Usage

```rust
use cortex_mem_core::{CortexFilesystem, FilesystemOperations};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize filesystem
    let fs = Arc::new(CortexFilesystem::new("./cortex-data"));
    fs.initialize().await?;

    // Write a memory
    fs.write("cortex://users/john/preferences.md", 
             "Prefers dark mode and vim keybindings").await?;

    // Read back
    let content = fs.read("cortex://users/john/preferences.md").await?;
    println!("Content: {}", content);

    Ok(())
}
```

### Session Management

```rust
use cortex_mem_core::{SessionManager, SessionConfig, Message, MessageRole};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fs = Arc::new(CortexFilesystem::new("./cortex-data")?);
    fs.initialize().await?;
    
    let session_manager = SessionManager::new(fs);
    
    // Create a session
    let session = session_manager.create_session(
        "tech-support",
        Some("Technical Support Chat".to_string())
    ).await?;
    
    // Add messages
    session_manager.add_message(
        &session.thread_id,
        Message {
            role: MessageRole::User,
            content: "How do I reset my password?".to_string(),
            metadata: Default::default(),
        }
    ).await?;
    
    // Extract memories
    let memories = session_manager.extract_memories(&session.thread_id).await?;
    println!("Extracted {} facts", memories.facts.len());
    
    Ok(())
}
```

### Vector Search

```rust
use cortex_mem_core::{VectorSearchEngine, SearchOptions};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fs = Arc::new(CortexFilesystem::new("./cortex-data")?);
    fs.initialize().await?;
    
    let search_engine = VectorSearchEngine::new(fs, vec![])?;
    
    // Search with options
    let results = search_engine.search(
        "password reset",
        SearchOptions::default()
    ).await?;
    
    for result in results {
        println!("Found: {} (score: {:.2})", result.content, result.score);
    }
    
    Ok(())
}
```

## 🌐 The Cortex Filesystem

The Cortex Filesystem extends standard file operations with custom URIs:

### URI Scheme

```
cortex://{dimension}/{tenant}/{path/to/resource}
```

#### Dimensions:
- **`users`** - User-specific memories
- **`agents`** - Agent-specific memories  
- **`threads`** - Conversational sessions
- **`global`** - Shared resources and knowledge

### Example URIs

```
cortex://users/john/preferences.md
cortex://agents/tech-support/knowledge.md
cortex://threads/session-123/timeline/2024/01/15/14_30_00_abc123.md
cortex://global/company-policy.md
```

## 📚 Memory Architecture

Cortex implements a three-tier memory system:

### L0: Abstract Layer (~100 tokens)
- Ultra-condensed summaries
- Key fact extraction
- Perfect for quick context

### L1: Overview Layer (~500-2000 tokens)
- Detailed summaries
- Key points and decisions
- Good for partial context

### L3: Full Content
- Complete original content
- Source of truth
- Used for deep analysis

## 🔧 Configuration

```rust
use cortex_mem_core::{CortexFilesystem, config::Config};

// From environment
let config = Config::from_env()?;

// From file
let config = Config::from_file("config.toml")?;

// Custom configuration
let fs = CortexFilesystem::with_config(config)?;
```

## 🔄 Event System

Cortex includes an event-driven automation system:

```rust
use cortex_mem_core::{CortexEvent, AutomationManager};

// Handle events
match event {
    CortexEvent::FilesystemEvent(event) => {
        // File changed - trigger re-indexing
    }
    CortexEvent::SessionEvent(event) => {
        // Session updated - trigger extraction
    }
    CortexEvent::SystemEvent(event) => {
        // System event - handle accordingly
    }
}
```

## 🔗 Integration with Other Crates

- **`cortex-mem-config`**: Provides configuration types
- **`cortex-mem-tools`**: Utilities and helpers
- **`cortex-mem-rig`**: RIG framework adapters
- **`cortex-mem-service`**: REST API implementation
- **`cortex-mem-cli`**: Command-line interface
- **`cortex-mem-mcp`**: MCP server implementation
- **`cortex-mem-insights`**: Observability dashboard

## 🧪 Testing

Running tests requires all features:

```bash
cargo test -p cortex-mem-core --all-features
```

## 🌟 Features

- **vector-search**: Enable Qdrant vector database integration (enabled by default)
- **embeddings**: Enable embedding generation
- **automation**: Enable event-driven automation
- **extraction**: Enable LLM-based memory extraction

## 📝 Dependencies

Key dependencies include:
- `serde` for serialization
- `tokio` for async runtime
- `qdrant-client` for vector storage
- `rig-core` for LLM integration
- `chrono` for timestamps
- `uuid` for unique identifiers
- `regex` for text matching
- `sha2` for hashing

## 📄 License

MIT License - see the [`LICENSE`](../../LICENSE) file for details.

## 🤝 Contributing

Please read our contributing guidelines and submit pull requests to the main repository.

## 🔍 Additional Documentation

- [Architecture Overview](../../litho.docs/en/2.Architecture.md)
- [Core Workflow](../../litho.docs/en/3.Workflow.md)
- [API Reference](docs/api-reference.md)