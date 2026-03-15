# Setup Guide

Installation and configuration guide for MemClaw.

## Supported Platforms

| Platform | npm Package |
|----------|-------------|
| macOS Apple Silicon | `@memclaw/bin-darwin-arm64` |
| Windows x64 | `@memclaw/bin-win-x64` |

> **Note**: MemClaw is only supported on the platforms listed above.

## Requirements

| Requirement | Details |
|-------------|---------|
| **Node.js** | ≥ 20.0.0 |
| **OpenClaw** | Installed and configured |
| **Qdrant** | Vector database (port 6333/6334) |
| **cortex-mem-service** | Memory service (port 8085) |

## Binary Installation

MemClaw binaries (Qdrant, cortex-mem-service, cortex-mem-cli) are distributed via platform-specific npm packages:

- `@memclaw/bin-darwin-arm64` — macOS Apple Silicon
- `@memclaw/bin-win-x64` — Windows x64

These packages are installed automatically as optional dependencies when installing `@memclaw/plugin`.

### Manual Binary Installation

If binaries are not installed, run:

```
npm install @memclaw/bin-darwin-arm64
```

or (for Windows):

```
npm install @memclaw/bin-win-x64
```

## First-Time Setup Checklist

**Before using MemClaw, complete these steps:**

### Step 1: Verify Platform Support

Ensure you are on a supported platform (macOS Apple Silicon or Windows x86/x64).

### Step 2: Prepare Data Directory

The `--data-dir` parameter specifies the root directory for MemClaw data. By default, use the system's application data directory:

| Platform | Default `--data-dir` |
|----------|----------------------|
| macOS | `~/Library/Application Support/memclaw` |
| Windows | `%LOCALAPPDATA%\memclaw` |
| Linux | `~/.local/share/memclaw` |

The `config.toml` file should be placed directly in this directory.

### Step 3: Create Configuration File

**CRITICAL**: The `config.toml` file MUST be placed in the data directory BEFORE starting cortex-mem-service.

Create `config.toml` with the following content:

```toml
# MemClaw Configuration
#
# Fill in the required values marked with [REQUIRED] before starting the service.

# ============================================================
# Qdrant Vector Database Configuration
# ============================================================
[qdrant]
# Qdrant gRPC API URL
url = "http://localhost:6334"
# Collection name for storing memory vectors
collection_name = "memclaw"
# Connection timeout in seconds
timeout_secs = 30

# ============================================================
# LLM Configuration [REQUIRED for memory processing]
# ============================================================
[llm]
# Your LLM API endpoint (OpenAI-compatible)
api_base_url = "https://api.openai.com/v1"
# Your API key [REQUIRED]
api_key = "your-api-key-here"
# Model for memory extraction and layer generation
model_efficient = "gpt-5-mini"
temperature = 0.1
max_tokens = 65535

# ============================================================
# Embedding Configuration [REQUIRED for vector search]
# ============================================================
[embedding]
# Your embedding API endpoint (OpenAI-compatible)
api_base_url = "https://api.openai.com/v1"
# Your API key [REQUIRED - can be same as llm.api_key]
api_key = "your-api-key-here"
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

# ============================================================
# Service Configuration
# ============================================================
[server]
host = "localhost"
port = 8085

# ============================================================
# Cortex Memory Settings
# ============================================================
[cortex]
# Data directory path - MUST match the --data-dir argument
# Default paths by platform:
#   macOS:   "~/Library/Application Support/memclaw"
#   Windows: "%LOCALAPPDATA%\\memclaw"
#   Linux:   "~/.local/share/memclaw"
data_dir = "."
enable_intent_analysis = false
```

### Step 4: Verify Services

Check that Qdrant and cortex-mem-service are accessible:

| Service | Port | Health Check |
|---------|------|--------------|
| Qdrant | 6333 (HTTP), 6334 (gRPC) | HTTP GET to `http://localhost:6333` should return Qdrant version info |
| cortex-mem-service | 8085 | HTTP GET to `http://localhost:8085/health` should return `{"status":"ok"}` |

### Step 5: Start Services (if not running)

**Starting Qdrant:**

If `autoStartServices` is `true` in plugin config, MemClaw will start Qdrant automatically.

To start manually, run the Qdrant binary from the platform package with:
- `--storage-path` pointing to a storage directory
- `--http-port 6333`
- `--grpc-port 6334`

**Starting cortex-mem-service:**

**CRITICAL**: cortex-mem-service MUST be started with `--data-dir` flag pointing to the directory containing `config.toml`.

Arguments:
- `--data-dir <path>` — Path to data directory containing `config.toml` (**REQUIRED**)
- `--config <path>` — Path to config file (optional, defaults to `config.toml` in data directory)

Example:
```
cortex-mem-service --data-dir ~/Library/Application\ Support/memclaw
```

Or on Windows:
```
cortex-mem-service --data-dir %LOCALAPPDATA%\memclaw
```

## Plugin Configuration

Edit your `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://localhost:8085",
          "tenantId": "tenant_claw",
          "autoStartServices": true
        }
      }
    }
  },
  "agents": {
    "defaults": {
      "memorySearch": { "enabled": false }
    }
  }
}
```

> **Important**: Set `memorySearch.enabled: false` to disable OpenClaw's built-in memory search and use MemClaw instead.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `serviceUrl` | string | `http://localhost:8085` | Cortex Memory service URL |
| `tenantId` | string | `tenant_claw` | Tenant ID for data isolation |
| `autoStartServices` | boolean | `true` | Auto-start Qdrant and cortex-mem-service |
| `defaultSessionId` | string | `default` | Default session for memory operations |
| `searchLimit` | number | `10` | Default number of search results |
| `minScore` | number | `0.6` | Minimum relevance score (0-1) |

## Troubleshooting

### Platform Not Supported

If you see "Platform not supported" error:
- Verify you are on macOS Apple Silicon or Windows x64
- Check that the correct `@memclaw/bin-*` package is installed

### Binaries Not Found

If binaries are missing:
1. Verify `@memclaw/bin-*` package is in `node_modules`
2. Try reinstalling: `npm install @memclaw/bin-darwin-arm64` (or `bin-win-x64`)

### cortex-mem-service Won't Start

1. Verify `--data-dir` flag is provided
2. Verify `config.toml` exists in the data directory
3. Verify required fields in `config.toml`:
   - `llm.api_key` is non-empty
   - `embedding.api_key` is non-empty
   - `cortex.data_dir` matches `--data-dir` argument

Default data directories:
| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/memclaw` |
| Windows | `%LOCALAPPDATA%\memclaw` |
| Linux | `~/.local/share/memclaw` |

### Services Not Accessible

1. Verify ports 6333, 6334, 8085 are not in use by other applications
2. Verify firewall allows connections on these ports
3. Check service logs for error messages

### Configuration File Issues

1. Ensure `config.toml` uses valid TOML syntax
2. Verify file encoding is UTF-8
3. On Windows, use double backslashes in paths: `C:\\Users\\...`

### API Key Issues

1. Verify API key is valid and has sufficient credits
2. Verify `api_base_url` is correct for your provider
3. Verify network connectivity to the API endpoint
