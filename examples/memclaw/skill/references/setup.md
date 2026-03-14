# Setup Guide

Installation and configuration guide for MemClaw.

## Requirements

| Requirement | Details |
|-------------|---------|
| **Platforms** | Windows x86, macOS Apple Silicon |
| **Node.js** | ≥ 22.0.0 |
| **OpenClaw** | Installed and configured |

## Installation

### Method 1: Install from ClawHub

```bash
openclaw plugins install memclaw
```

### Method 2: Local Development Installation

For developers using a local version or developing the plugin:

```bash
# Clone the repository
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem/examples/memclaw

# Install dependencies
bun install

# Build the plugin
bun run build

# Create symlink to plugin directory
mkdir -p ~/.openclaw/plugins
ln -sf "$(pwd)" ~/.openclaw/plugins/memclaw
```

Configure in `openclaw.json` with local path:

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "path": "./plugins/memclaw"
      }
    }
  }
}
```

After code changes, rebuild with `bun run build` and restart OpenClaw.

## OpenClaw Configuration

Edit your `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://127.0.0.1:8085",
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

## LLM Configuration

On first run, MemClaw creates a configuration file:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\memclaw\config.toml` |
| macOS | `~/Library/Application Support/memclaw/config.toml` |

Edit the configuration file and fill in required fields:

```toml
# LLM Configuration [REQUIRED for memory processing]
[llm]
# Your LLM API endpoint (OpenAI-compatible)
api_base_url = "https://api.openai.com/v1"
# Your API key [REQUIRED]
api_key = "sk-xxx"
# Model for memory extraction and layer generation
model_efficient = "gpt-4o-mini"
temperature = 0.1
max_tokens = 4096

# Embedding Configuration [REQUIRED for vector search]
[embedding]
# Your embedding API endpoint (OpenAI-compatible)
api_base_url = "https://api.openai.com/v1"
# Your API key [REQUIRED - can be same as llm.api_key]
api_key = "sk-xxx"
model_name = "text-embedding-3-small"
batch_size = 10
```

Then restart OpenClaw.

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `serviceUrl` | string | `http://127.0.0.1:8085` | Cortex Memory service URL |
| `tenantId` | string | `tenant_claw` | Tenant ID for data isolation |
| `autoStartServices` | boolean | `true` | Auto-start Qdrant and service |
| `defaultSessionId` | string | `default` | Default session for memory operations |
| `searchLimit` | number | `10` | Default number of search results |
| `minScore` | number | `0.6` | Minimum relevance score (0-1) |

## Troubleshooting

### Services Won't Start

1. Check that ports 6333, 6334, 8085 are available
2. Verify `api_key` fields are filled in config.toml
3. Run `openclaw skills` to check plugin status

### Configuration File Not Created

1. Ensure OpenClaw has write permissions to the config directory
2. Check OpenClaw logs for error messages
3. Manually create the directory and restart OpenClaw

### API Key Issues

1. Verify your API key is valid and has sufficient credits
2. Ensure `api_base_url` is correct for your provider
3. Check network connectivity to the API endpoint
