# Cortex-Mem Configuration Guide

This document provides detailed configuration options for Cortex-Mem.

## Table of Contents

- [Server Configuration](#server-configuration)
- [Vector Store Configuration](#vector-store-configuration)
- [LLM Configuration](#llm-configuration)
- [Cascade Configuration](#cascade-configuration)
- [Cache Configuration](#cache-configuration)
- [Tenant Configuration](#tenant-configuration)

## Server Configuration

### Basic Settings

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4
max_connections = 1000

[server.tls]
enabled = false
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"
```

### Logging

```toml
[server.logging]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json, text
output = "stdout"  # stdout, stderr, file
file_path = "/var/log/cortex-mem.log"
```

## Vector Store Configuration

### Qdrant Settings

```toml
[vector_store]
type = "qdrant"

[vector_store.qdrant]
url = "http://localhost:6333"
api_key = ""  # Optional API key

# Connection settings
timeout_ms = 5000
max_retries = 3
retry_delay_ms = 100

# Collection defaults
collection_prefix = "cortex_mem_"
default_vector_dim = 1536
```

### Collection Management

```toml
[vector_store.collection]
# Auto-create collections if missing
auto_create = true

# Replication factor
replication_factor = 2

# Shard count
shards = 4
```

### Performance Tuning

```toml
[vector_store.performance]
# Batch size for bulk operations
batch_size = 100

# Parallelism for search
search_parallelism = 4

# Cache size for vectors (number of vectors)
vector_cache_size = 10000
```

## LLM Configuration

### OpenAI Settings

```toml
[llm]
provider = "openai"

[llm.openai]
api_key = "${OPENAI_API_KEY}"  # Use environment variable
model = "gpt-4"
embedding_model = "text-embedding-ada-002"

# API settings
base_url = "https://api.openai.com/v1"
timeout_ms = 30000
max_retries = 3
```

### Model Parameters

```toml
[llm.parameters]
temperature = 0.7
max_tokens = 2000
top_p = 1.0
frequency_penalty = 0.0
presence_penalty = 0.0
```

### Embedding Configuration

```toml
[llm.embedding]
# Batch size for embedding requests
batch_size = 100

# Cache embeddings for repeated content
cache_enabled = true
cache_ttl_seconds = 86400  # 24 hours
```

## Cascade Configuration

### Layer Settings

```toml
[cascade]
# Enable cascade updates
enabled = true

[cascade.layers.L0]
# Leaf layer - raw observations
max_memories = 10000
ttl_days = 30

[cascade.layers.L1]
# Intermediate layer - patterns
max_memories = 5000
ttl_days = 90

[cascade.layers.L2]
# Root layer - core memories
max_memories = 1000
ttl_days = 365
```

### Debouncer Settings

```toml
[cascade.debouncer]
# Wait time before processing updates
delay_ms = 5000

# Maximum wait time before forced update
max_delay_ms = 30000

# Maximum batch size
max_batch_size = 50
```

### Update Triggers

```toml
[cascade.triggers]
# Trigger L0->L1 when L0 reaches this count
L0_threshold = 100

# Trigger L1->L2 when L1 reaches this count
L1_threshold = 50

# Trigger on importance threshold
importance_threshold = 0.8
```

## Cache Configuration

### LLM Result Cache

```toml
[cache.llm]
enabled = true

# Cache type: memory, redis
type = "memory"

# Maximum cache size (entries)
max_entries = 1000

# Time-to-live in seconds
ttl_seconds = 3600

# Redis settings (if type = "redis")
[cache.llm.redis]
url = "redis://localhost:6379"
prefix = "cortex_mem_llm_"
```

### Query Cache

```toml
[cache.query]
enabled = true
ttl_seconds = 300
max_entries = 500
```

## Tenant Configuration

### Default Settings

```toml
[tenant.defaults]
# Default vector dimension
vector_dim = 1536

# Default isolation level
isolation = "strict"  # strict, moderate, relaxed

# Enable tenant-level caching
cache_enabled = true
```

### Resource Limits

```toml
[tenant.limits]
# Maximum memories per tenant
max_memories = 100000

# Maximum storage in MB
max_storage_mb = 1024

# Maximum API calls per minute
rate_limit = 100

# Maximum concurrent connections
max_connections = 10
```

### Multi-Tenancy

```toml
[tenant.multi_tenant]
# Enable tenant isolation
isolation_enabled = true

# Tenant resolution strategy
resolution = "header"  # header, path, subdomain

# Header name for tenant ID
tenant_header = "X-Tenant-ID"
```

## Environment Variables

The following environment variables can override configuration:

| Variable | Description | Example |
|----------|-------------|---------|
| `CORTEX_MEM_CONFIG` | Path to config file | `/etc/cortex-mem/config.toml` |
| `CORTEX_MEM_LOG_LEVEL` | Logging level | `debug` |
| `OPENAI_API_KEY` | OpenAI API key | `sk-...` |
| `QDRANT_URL` | Qdrant server URL | `http://localhost:6333` |
| `QDRANT_API_KEY` | Qdrant API key | `...` |
| `REDIS_URL` | Redis server URL | `redis://localhost:6379` |

## Example Complete Configuration

```toml
# Cortex-Mem Configuration File

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[server.logging]
level = "info"
format = "json"

[vector_store]
type = "qdrant"

[vector_store.qdrant]
url = "http://localhost:6333"
timeout_ms = 5000
collection_prefix = "cortex_mem_"
default_vector_dim = 1536

[llm]
provider = "openai"

[llm.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4"
embedding_model = "text-embedding-ada-002"

[llm.embedding]
batch_size = 100
cache_enabled = true

[cascade]
enabled = true

[cascade.debouncer]
delay_ms = 5000
max_batch_size = 50

[cache.llm]
enabled = true
type = "memory"
max_entries = 1000
ttl_seconds = 3600

[tenant.defaults]
vector_dim = 1536
isolation = "strict"
```

## Validation

To validate your configuration:

```bash
cortex-mem config validate --config /path/to/config.toml
```

This checks for:
- Valid syntax
- Required fields present
- Value constraints
- Dependency compatibility
