# Cortex Memory RIG Integration

This crate provides integration between the cortex-mem memory system and the RIG AI agent framework. It offers tools that AI agents can use to store, search, and retrieve memories.

## Overview

The cortex-mem-rig crate has been refactored to provide a consistent interface with cortex-mem-mcp. It now offers four distinct tools that mirror the MCP protocol tools:

- `store_memory` - Store a new memory
- `query_memory` - Search memories using semantic similarity
- `list_memories` - List memories with optional filtering
- `get_memory` - Retrieve a specific memory by ID

This design ensures that AI agents have a consistent experience whether they're using MCP or RIG to interface with the cortex-mem system.

## Features

- **Tool-based Interface**: Four distinct tools with specific purposes
- **Consistent with MCP**: Same function signatures and parameters as cortex-mem-mcp
- **Semantic Search**: Advanced memory searching using vector embeddings
- **Type-safe**: Fully type-safe Rust interface with structured parameters
- **Error Handling**: Comprehensive error handling for all operations
- **Backward Compatibility**: Maintains compatibility with existing code through a wrapper

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
cortex-mem-core = { version = "0.1" }
cortex-mem-config = { version = "0.1" }
cortex-mem-rig = { version = "0.1" }
rig = { version = "0.1" }
tokio = { version = "1" }
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Basic Usage

### Set up the memory system

```rust
use std::sync::Arc;
use cortex_mem_config::Config;
use cortex_mem_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
};
use cortex_mem_rig:: create_memory_tools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load("config.toml")?;
    
    // Initialize memory system
    let (vector_store, llm_client) = initialize_memory_system(&config).await?;
    
    // Create memory manager
    let memory_manager = Arc::new(MemoryManager::new(
        vector_store,
        llm_client,
        config.memory.clone(),
    ));
    
    // Create memory tools
    let memory_tools = create_memory_tools(
        memory_manager,
        &config,
        None, // Use default configuration
    );
    
    Ok(())
}
```

### Using the tools

```rust
use cortex_mem_rig::{
    MemoryTools, StoreMemoryArgs, QueryMemoryArgs, 
    ListMemoriesArgs, GetMemoryArgs
};

// Store a memory
let store_args = StoreMemoryArgs {
    content: "The user prefers working in the morning".to_string(),
    user_id: Some("user123".to_string()),
    agent_id: Some("agent456".to_string()),
    memory_type: Some("personal".to_string()),
    topics: Some(vec!["preferences".to_string()]),
};

let result = memory_tools.store_memory().call(store_args).await?;
println!("Stored memory: {}", result.success);

// Query memories
let query_args = QueryMemoryArgs {
    query: "What are the user's preferences?".to_string(),
    k: Some(5),
    memory_type: None,
    min_salience: Some(0.5),
    topics: Some(vec!["preferences".to_string()]),
    user_id: Some("user123".to_string()),
    agent_id: Some("agent456".to_string()),
};

let result = memory_tools.query_memory().call(query_args).await?;
if let Some(data) = result.data {
    println!("Found memories: {}", data);
}

// List memories
let list_args = ListMemoriesArgs {
    limit: Some(10),
    memory_type: Some("personal".to_string()),
    user_id: Some("user123".to_string()),
    agent_id: Some("agent456".to_string()),
};

let result = memory_tools.list_memories().call(list_args).await?;
if let Some(data) = result.data {
    println!("Memories: {}", data);
}

// Get a specific memory
let get_args = GetMemoryArgs {
    memory_id: "memory_123".to_string(),
};

let result = memory_tools.get_memory().call(get_args).await?;
if let Some(data) = result.data {
    println!("Memory: {}", data);
}
```

## Configuration

You can customize the memory tools with a `MemoryToolConfig`:

```rust
use cortex_mem_rig::MemoryToolConfig;

let config = MemoryToolConfig {
    default_user_id: Some("default_user".to_string()),
    default_agent_id: Some("default_agent".to_string()),
    max_search_results: Some(20),
    auto_enhance: Some(true),
    search_similarity_threshold: Some(0.7),
};

let memory_tools = create_memory_tools(
    memory_manager,
    &global_config,
    Some(config),
);
```

## Integration with RIG Agent Framework

The tools are designed to work seamlessly with the RIG AI agent framework:

```rust
use rig::agent::Agent;
use rig::providers::openai;

// Create an agent with memory capabilities
let agent = Agent::builder()
    .model(openai::GPT_4_O) // Example model
    .preamble("You are a helpful assistant with access to memories.")
    .tool(memory_tools.store_memory())
    .tool(memory_tools.query_memory())
    .tool(memory_tools.list_memories())
    .tool(memory_tools.get_memory())
    .build();

// The agent can now use these tools to interact with memories
```

## API Reference

### Store Memory

Stores a new memory in the system.

**Parameters:**
- `content` (required, string): The content of the memory
- `user_id` (optional, string): User ID associated with the memory
- `agent_id` (optional, string): Agent ID associated with the memory
- `memory_type` (optional, string): Type of memory (conversational, procedural, factual, semantic, episodic, personal)
- `topics` (optional, array of strings): Topics to associate with the memory

### Query Memory

Searches memories using semantic similarity.

**Parameters:**
- `query` (required, string): Query string for semantic search
- `k` (optional, integer): Maximum number of results to return (default: 10)
- `memory_type` (optional, string): Type of memory to filter by
- `min_salience` (optional, number): Minimum salience/importance score threshold (0-1)
- `topics` (optional, array of strings): Topics to filter memories by
- `user_id` (optional, string): User ID to filter memories
- `agent_id` (optional, string): Agent ID to filter memories

### List Memories

Retrieves memories with optional filtering.

**Parameters:**
- `limit` (optional, integer): Maximum number of memories to return (default: 100, max: 1000)
- `memory_type` (optional, string): Type of memory to filter by
- `user_id` (optional, string): User ID to filter memories
- `agent_id` (optional, string): Agent ID to filter memories

### Get Memory

Retrieves a specific memory by its exact ID.

**Parameters:**
- `memory_id` (required, string): Exact ID of the memory to retrieve

## Migration from Previous Versions

If you were using the old single-tool interface, you can still use the backward-compatible wrapper:

```rust
// Old way (deprecated)
use cortex_mem_rig::{create_memory_tool, MemoryTool};

let tool = create_memory_tool(memory_manager, &config, None);

// New way (recommended)
use cortex_mem_rig::create_memory_tools;

let tools = create_memory_tools(memory_manager, &config, None);
```

The old `MemoryTool` is still available but marked as deprecated. It now internally uses the new tool structure, so your existing code will continue to work, but you should migrate to the new interface for cleaner code and better type safety.

## Examples

See the `examples` directory for complete working examples:
- `memory_tools_example.rs`: Basic usage example with all tools

## Architecture

The crate shares the core functionality with cortex-mem-mcp through the cortex-mem-tools crate:

1. `cortex-mem-tools`: Provides shared operations and tool definitions
2. `cortex-mem-rig`: Implements RIG-specific tool interfaces
3. `cortex-mem-mcp`: Implements MCP-specific tool interfaces

Both rig and mcp use the same underlying operations, ensuring consistent behavior across different AI frameworks.

## License

This project is licensed under the MIT License.