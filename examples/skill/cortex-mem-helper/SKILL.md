---
name: cortex-mem-helper
description: Helps developers understand and use the Cortex-Mem memory management system. Use when working with memory storage, vector databases, tenant management, cascade layers (L0/L1/L2), LLM caching, or when integrating Cortex-Mem into applications. Keywords: memory, vector, qdrant, tenant, cascade, layer.
license: Apache-2.0
metadata:
  author: cortex-mem-team
  version: "1.0"
compatibility: Designed for Rust/TypeScript projects using Cortex-Mem memory system
---

# Cortex-Mem Helper

This skill provides guidance for working with the Cortex-Mem memory management system, a distributed memory architecture with cascade layer updates.

## Overview

Cortex-Mem is a memory management system that provides:

- **Hierarchical Memory Storage**: Multi-layer (L0/L1/L2) cascade architecture
- **Vector Storage**: Qdrant-based vector database for semantic search
- **Tenant Management**: Multi-tenant support with tenant isolation
- **LLM Integration**: Smart caching and debouncing for LLM operations

## Key Concepts

### Cascade Layers

The system uses a three-tier cascade architecture:

1. **L0 (Leaf Layer)**: Raw memory events and observations
2. **L1 (Intermediate Layer)**: Aggregated summaries and patterns
3. **L2 (Root Layer)**: High-level abstractions and core memories

### Memory Operations

- **Store**: Add new memories with automatic layer propagation
- **Retrieve**: Semantic search across layers
- **Update**: Cascade updates with debouncing
- **Delete**: Remove memories with consistency maintenance

## Common Tasks

### Initializing Memory for a Tenant

```
1. Ensure Qdrant collection exists with correct dimensions
2. Initialize tenant context using switch_tenant API
3. Verify layer structure is created
```

### Storing a Memory

```
1. Create MemoryEvent with content and metadata
2. Call store operation
3. System automatically propagates to appropriate layers
```

### Retrieving Memories

```
1. Formulate query with optional filters
2. Specify target layers (L0, L1, L2, or all)
3. Results ranked by semantic similarity
```

## Configuration

Key configuration options:

- **Vector Dimension**: Must match embedding model (typically 1536 for OpenAI)
- **Debounce Interval**: Controls cascade update frequency
- **LLM Cache TTL**: Time-to-live for cached LLM results
- **Collection Prefix**: Namespace for Qdrant collections

## Troubleshooting

### Collection Not Found

Ensure the collection is created with `ensure_collection_with_dim()` before operations.

### Tenant Context Issues

Verify `switch_tenant` is called before any memory operations.

### Slow Cascade Updates

Check:
- LLM cache configuration
- Debounce interval settings
- Network connectivity to vector store

## Reference Files

For detailed technical documentation, see:
- [API Reference](references/REFERENCE.md) - Complete API documentation
- [Configuration Guide](references/CONFIG.md) - Detailed configuration options

## Example Usage

### TypeScript Integration

```typescript
import { CortexMemClient } from 'cortex-mem-openclaw';

const client = new CortexMemClient({
  endpoint: 'http://localhost:8080',
  tenantId: 'my-tenant'
});

// Store a memory
await client.store({
  content: 'User prefers dark mode',
  category: 'user_preference',
  metadata: { source: 'settings' }
});

// Retrieve memories
const results = await client.retrieve({
  query: 'user preferences',
  layers: ['L1', 'L2'],
  limit: 10
});
```

### Rust Integration

```rust
use cortex_mem_core::MemoryClient;

let client = MemoryClient::new(config).await?;
client.switch_tenant("my-tenant").await?;

// Store memory
client.store(MemoryEvent {
    content: "Important note".to_string(),
    category: MemoryCategory::Note,
    ..Default::default()
}).await?;
```

## Best Practices

1. **Always initialize tenant context** before any operations
2. **Use appropriate layer targeting** to optimize retrieval performance
3. **Configure LLM caching** to reduce API costs and latency
4. **Monitor debounce queues** for cascade update health
5. **Set appropriate TTLs** based on memory volatility
