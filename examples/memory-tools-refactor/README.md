# Cortex-Mem Tools Refactor

This example demonstrates the shared memory tools refactor that eliminates code duplication between `cortex-mem-rig` and `cortex-mem-mcp`.

## Overview

Previously, both `cortex-mem-rig` and `cortex-mem-mcp` implemented similar functionality for storing, querying, and retrieving memories, but with different interfaces:

- `cortex-mem-mcp` used the MCP protocol with its own tool definitions
- `cortex-mem-rig` used the Rig framework's Tool trait

This refactor introduces a new `cortex-mem-tools` module that provides:

1. A unified interface for memory operations
2. Common error handling
3. Shared data structures
4. Reusable core logic

## Architecture

The new architecture consists of three layers:

1. **Core Layer (cortex-mem-core)**: Contains the fundamental MemoryManager and related types
2. **Shared Tools Layer (cortex-mem-tools)**: Provides unified operations and common data structures
3. **Adapter Layer (cortex-mem-mcp, cortex-mem-rig)**: Implements framework-specific adapters

### Components

#### cortex-mem-tools

- `MemoryOperations`: Core operations handler that wraps MemoryManager
- `MemoryOperationPayload`: Unified request structure
- `MemoryOperationResponse`: Unified response structure
- `MemoryToolsError`: Common error types

#### cortex-mem-mcp

- `MemoryMcpService`: Now uses the shared operations
- Converts MCP requests to `MemoryOperationPayload`
- Converts `MemoryOperationResponse` back to MCP responses

#### cortex-mem-rig

- `MemoryTool`: Now uses the shared operations
- Converts Rig tool arguments to `MemoryOperationPayload`
- Converts `MemoryOperationResponse` back to Rig tool outputs

## Benefits

1. **Reduced Code Duplication**: Core memory operations are implemented once in the shared module
2. **Consistent Behavior**: Both interfaces now behave identically
3. **Easier Maintenance**: Changes to memory operations only need to be made in one place
4. **Better Testing**: Core operations can be tested independently
5. **Extensibility**: New adapters can be easily added for other frameworks

## Usage

```rust
// Create shared operations
let operations = MemoryOperations::new(
    memory_manager,
    Some("default_user".to_string()),
    Some("default_agent".to_string()),
    10,
);

// Store a memory
let mut payload = MemoryOperationPayload::default();
payload.content = Some("This is a test memory".to_string());
let result = operations.store_memory(payload).await?;

// Query memories
let mut query_payload = MemoryOperationPayload::default();
query_payload.query = Some("test".to_string());
let result = operations.query_memory(query_payload).await?;
```

## Migration Guide

### For cortex-mem-mcp Users

No changes required. The MCP interface remains the same, but internally it now uses the shared operations.

### For cortex-mem-rig Users

No changes required. The Rig tool interface remains the same, but internally it now uses the shared operations.

## Testing

Run the example to see the shared tools in action:

```bash
cd examples/memory-tools-refactor
cargo run
```

## Future Enhancements

1. Add metrics and observability to the shared operations
2. Implement caching in the shared layer
3. Add more sophisticated filtering options
4. Support for batch operations