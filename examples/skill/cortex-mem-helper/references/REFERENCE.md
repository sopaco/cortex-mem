# Cortex-Mem API Reference

This document provides detailed technical reference for the Cortex-Mem API.

## Table of Contents

- [Client Initialization](#client-initialization)
- [Tenant Operations](#tenant-operations)
- [Memory Operations](#memory-operations)
- [Query Operations](#query-operations)
- [Layer Management](#layer-management)
- [Error Handling](#error-handling)

## Client Initialization

### TypeScript

```typescript
interface CortexMemConfig {
  endpoint: string;      // API endpoint URL
  tenantId?: string;     // Default tenant ID
  timeout?: number;      // Request timeout in ms
  retries?: number;      // Number of retry attempts
}

const client = new CortexMemClient(config: CortexMemConfig);
```

### Rust

```rust
pub struct ClientConfig {
    pub endpoint: String,
    pub default_tenant: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
}

impl MemoryClient {
    pub async fn new(config: ClientConfig) -> Result<Self, Error>;
}
```

## Tenant Operations

### switch_tenant

Switches the active tenant context for subsequent operations.

**TypeScript:**
```typescript
await client.switchTenant(tenantId: string): Promise<void>
```

**Rust:**
```rust
pub async fn switch_tenant(&self, tenant_id: &str) -> Result<(), Error>
```

**Parameters:**
- `tenantId` (string): The unique identifier for the tenant

**Throws:**
- `TenantNotFoundError`: If tenant does not exist
- `ValidationError`: If tenant_id is invalid

### create_tenant

Creates a new tenant with specified configuration.

```typescript
await client.createTenant(options: {
  tenantId: string;
  config?: TenantConfig;
}): Promise<Tenant>
```

## Memory Operations

### store

Stores a new memory event in the system.

```typescript
interface MemoryEvent {
  content: string;           // Memory content
  category: MemoryCategory;  // Memory category
  metadata?: Record<string, any>;  // Optional metadata
  timestamp?: Date;          // Optional timestamp (defaults to now)
  importance?: number;       // 0-1 scale, affects layer propagation
}

enum MemoryCategory {
  UserPreference = 'user_preference',
  Observation = 'observation',
  Fact = 'fact',
  Note = 'note',
  Event = 'event'
}

await client.store(event: MemoryEvent): Promise<StoreResult>
```

**Returns:**
```typescript
interface StoreResult {
  id: string;          // Unique memory ID
  layer: 'L0' | 'L1' | 'L2';  // Initial storage layer
  propagated: boolean; // Whether propagated to higher layers
}
```

### retrieve

Retrieves memories based on semantic search.

```typescript
interface RetrieveOptions {
  query: string;              // Search query
  layers?: ('L0' | 'L1' | 'L2')[];  // Target layers (default: all)
  limit?: number;             // Max results (default: 10)
  threshold?: number;         // Similarity threshold (0-1)
  filters?: MemoryFilters;    // Optional filters
  includeMetadata?: boolean;  // Include metadata in results
}

interface MemoryFilters {
  category?: MemoryCategory[];
  dateRange?: {
    start?: Date;
    end?: Date;
  };
  metadata?: Record<string, any>;
}

await client.retrieve(options: RetrieveOptions): Promise<RetrieveResult>
```

**Returns:**
```typescript
interface RetrieveResult {
  memories: MemoryResult[];
  total: number;
  queryTime: number;
}

interface MemoryResult {
  id: string;
  content: string;
  category: MemoryCategory;
  score: number;        // Similarity score
  layer: 'L0' | 'L1' | 'L2';
  metadata?: Record<string, any>;
  timestamp: Date;
}
```

## Layer Management

### get_layer_info

Retrieves information about a specific layer.

```typescript
await client.getLayerInfo(layer: 'L0' | 'L1' | 'L2'): Promise<LayerInfo>

interface LayerInfo {
  memoryCount: number;
  lastUpdate: Date;
  size: number;  // in bytes
  health: 'healthy' | 'degraded' | 'error';
}
```

### trigger_cascade_update

Manually triggers cascade update from a specific layer.

```typescript
await client.triggerCascadeUpdate(options: {
  fromLayer: 'L0' | 'L1';
  force?: boolean;  // Bypass debouncer
}): Promise<CascadeResult>
```

## Error Handling

### Error Types

| Error Type | Description | Recovery Action |
|------------|-------------|-----------------|
| `ConnectionError` | Cannot connect to server | Retry with backoff |
| `TenantNotFoundError` | Tenant does not exist | Create tenant first |
| `ValidationError` | Invalid input parameters | Fix parameter values |
| `LayerNotReadyError` | Layer not initialized | Wait and retry |
| `VectorStoreError` | Qdrant operation failed | Check Qdrant health |
| `LLMError` | LLM operation failed | Check API key/quotas |

### Error Response Format

```typescript
interface CortexMemError {
  code: string;
  message: string;
  details?: Record<string, any>;
  retryable: boolean;
}
```

## Rate Limits

| Operation | Rate Limit | Burst |
|-----------|------------|-------|
| Store | 100/min | 20 |
| Retrieve | 500/min | 50 |
| Layer Operations | 30/min | 10 |

## Versioning

The API follows semantic versioning. Current version: **v2.5**

Breaking changes require major version bump. Check the changelog for migration guides.
