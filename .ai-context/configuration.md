# Configuration Reference

## Configuration File

Cortex Memory uses a `config.toml` file for configuration.

### Default Locations

Priority order:
1. Explicit `--config` flag
2. `{data_dir}/config.toml`
3. `./config.toml` (current directory)

---

## Complete Configuration Schema

```toml
# =============================================================================
# Qdrant Vector Database Configuration
# =============================================================================
[qdrant]
# URL of Qdrant gRPC endpoint
url = "http://localhost:6334"

# HTTP URL for REST API (optional, for health checks)
http_url = "http://localhost:6333"

# Base collection name (tenant ID will be appended)
collection_name = "cortex-memory"

# Connection timeout in seconds
timeout_secs = 30

# Embedding dimension (must match your embedding model)
# Examples: 1536 for text-embedding-3-small, 4096 for larger models
embedding_dim = 1536

# API key for Qdrant Cloud (optional)
api_key = ""

# =============================================================================
# LLM Configuration (for memory extraction and analysis)
# =============================================================================
[llm]
# Base URL of your LLM provider (OpenAI-compatible API)
api_base_url = "https://api.openai.com/v1"

# API key (supports environment variable expansion)
api_key = "${OPENAI_API_KEY}"

# Model for efficient operations (extraction, classification)
model_efficient = "gpt-5-mini"

# Model for complex reasoning (optional)
model_reasoning = "o1-preview"

# Sampling temperature (0.0 - 2.0)
temperature = 0.1

# Maximum tokens for generation
max_tokens = 40960

# Request timeout in seconds
timeout_secs = 60

# =============================================================================
# Embedding Service Configuration
# =============================================================================
[embedding]
# Base URL of your embedding provider
api_base_url = "https://api.openai.com/v1"

# API key
api_key = "${OPENAI_API_KEY}"

# Embedding model name
model_name = "text-embedding-3-small"

# Batch size for embedding requests
batch_size = 10

# Request timeout in seconds
timeout_secs = 30

# =============================================================================
# Server Configuration (for cortex-mem-service)
# =============================================================================
[server]
# Server host
host = "localhost"

# Server port
port = 8085

# CORS origins (use ["*"] for development)
cors_origins = ["*"]

# =============================================================================
# Cortex Memory Settings
# =============================================================================
[cortex]
# Data directory for memory storage
data_dir = "./cortex-data"

# Enable LLM intent analysis before search
# Improves multi-hop query accuracy
enable_intent_analysis = true

# =============================================================================
# Logging Configuration
# =============================================================================
[logging]
# Enable logging
enabled = true

# Log directory
log_directory = "logs"

# Log level: trace, debug, info, warn, error
level = "info"
```

---

## Environment Variables

### Supported Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | Default API key for LLM and Embedding |
| `LLM_API_KEY` | LLM-specific API key (overrides OPENAI_API_KEY for LLM) |
| `LLM_API_BASE_URL` | LLM API base URL |
| `LLM_MODEL` | LLM model name |
| `EMBEDDING_API_KEY` | Embedding-specific API key |
| `EMBEDDING_API_BASE_URL` | Embedding API base URL |
| `EMBEDDING_MODEL_NAME` | Embedding model name |
| `QDRANT_URL` | Qdrant gRPC URL |
| `QDRANT_API_KEY` | Qdrant API key |
| `QDRANT_COLLECTION` | Qdrant collection name |

### Environment Variable Expansion

Use `${VAR_NAME}` syntax in `config.toml`:

```toml
[llm]
api_key = "${OPENAI_API_KEY}"
```

---

## Common Configurations

### OpenAI

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model_efficient = "gpt-5-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model_name = "text-embedding-3-small"

[qdrant]
embedding_dim = 1536
```

### Azure OpenAI

```toml
[llm]
api_base_url = "https://your-resource.openai.azure.com/openai/deployments/your-deployment"
api_key = "${AZURE_OPENAI_KEY}"
model_efficient = "gpt-4"

[embedding]
api_base_url = "https://your-resource.openai.azure.com/openai/deployments/embedding-deployment"
api_key = "${AZURE_OPENAI_KEY}"
model_name = "text-embedding-ada-002"

[qdrant]
embedding_dim = 1536
```

### Local LLM (Ollama)

```toml
[llm]
api_base_url = "http://localhost:11434/v1"
api_key = "ollama"
model_efficient = "llama3.2"

[embedding]
api_base_url = "http://localhost:11434/v1"
api_key = "ollama"
model_name = "nomic-embed-text"

[qdrant]
embedding_dim = 768  # Check your model's dimension
```

---

## CLI Configuration

### cortex-mem-cli

```bash
# Specify config file
cortex-mem --config /path/to/config.toml [command]

# Specify tenant
cortex-mem --config config.toml --tenant my-tenant [command]

# Verbose output
cortex-mem --config config.toml --verbose [command]
```

### cortex-mem-service

```bash
# Basic start
cortex-mem-service --config config.toml

# Custom host and port
cortex-mem-service --config config.toml --host 0.0.0.0 --port 9000

# Verbose logging
cortex-mem-service --config config.toml --verbose

# Log to file
cortex-mem-service --config config.toml --log-file logs/service.log
```

### cortex-mem-mcp

```bash
# Start MCP server
cortex-mem-mcp --config config.toml
```

---

## Validation

Check configuration validity:

```bash
# CLI will report errors on startup
cortex-mem --config config.toml stats

# Service health check
curl http://localhost:8085/health
```

---

## Troubleshooting

### Common Issues

1. **Embedding dimension mismatch**
   - Error: Vector dimension doesn't match
   - Solution: Set `embedding_dim` to match your embedding model

2. **API key not found**
   - Error: Authentication failed
   - Solution: Set environment variables or use literal keys in config

3. **Qdrant connection failed**
   - Error: Connection refused
   - Solution: Ensure Qdrant is running on the configured port

4. **Collection not found**
   - Error: Collection doesn't exist
   - Solution: Collection is auto-created on first use; ensure config is correct

### Debug Mode

Enable debug logging:

```toml
[logging]
level = "debug"
```

Or use CLI flag:
```bash
cortex-mem-service --config config.toml --verbose
```
