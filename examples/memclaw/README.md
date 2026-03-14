# MemClaw

Layered semantic memory plugin for OpenClaw with L0/L1/L2 tiered retrieval, automatic service management, and migration support from OpenClaw native memory.

## Overview

MemClaw is an OpenClaw plugin that provides advanced semantic memory capabilities using Cortex Memory's tiered memory architecture. It stores, searches, and recalls memories with intelligent layer-based retrieval that balances speed and context.

## Features

- **Three-Layer Memory Architecture**: L0 (abstract), L1 (overview), and L2 (full) layers for intelligent retrieval
- **Automatic Service Management**: Auto-starts Qdrant vector database and cortex-mem-service
- **Semantic Search**: Vector-based similarity search across all memory layers
- **Session Management**: Create, list, and close memory sessions
- **Migration Support**: One-click migration from OpenClaw native memory
- **Cross-Platform**: Supports Windows x64 and macOS Apple Silicon

## Architecture

### Memory Layers

| Layer | Tokens | Content | Role |
|-------|--------|---------|------|
| **L0 (Abstract)** | ~100 | High-level summary | Quick filtering |
| **L1 (Overview)** | ~2000 | Key points + context | Context refinement |
| **L2 (Full)** | Complete | Original content | Precise matching |

The search engine queries all three layers internally and returns unified results with `snippet` and `content`.

### System Components

```
OpenClaw + MemClaw Plugin
         │
         ├── cortex_search    → Search memories
         ├── cortex_recall    → Recall with context
         ├── cortex_add_memory → Store memories
         ├── cortex_list_sessions → List sessions
         ├── cortex_close_session → Close & extract
         └── cortex_migrate   → Migrate existing memory
                    │
                    ▼
         cortex-mem-service (port 8085)
                    │
                    ▼
         Qdrant (port 6334)
```

## Installation

### Requirements

| Requirement | Details |
|-------------|---------|
| **Platforms** | Windows x64, macOS Apple Silicon |
| **Node.js** | ≥ 22.0.0 |
| **OpenClaw** | Installed and configured |

### Install Plugin

```bash
openclaw plugins install memclaw
```

### Local Development Installation

For developers who want to use a local version of memclaw or develop the plugin:

```bash
# Clone the repository
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem/examples/memclaw

# Install dependencies
bun install

# Build the plugin
bun run build

# Create a symlink to the plugin directory
# This makes OpenClaw use your local version
mkdir -p ~/.openclaw/plugins
ln -sf "$(pwd)" ~/.openclaw/plugins/memclaw
```

Then configure in `openclaw.json` with the local plugin path:

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

After making code changes, rebuild with `bun run build` and restart OpenClaw.

### Configure OpenClaw

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

> **Note**: Set `memorySearch.enabled: false` to disable OpenClaw's built-in memory search and use MemClaw instead.

### Configure LLM

On first run, MemClaw creates a configuration file:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\memclaw\config.toml` |
| macOS | `~/Library/Application Support/memclaw/config.toml` |

Edit the configuration file and fill in required fields:

```toml
[llm]
api_key = "xxx"  # REQUIRED: Your LLM API key

[embedding]
api_key = "xxx"  # REQUIRED: Your embedding API key (can be same as llm.api_key)
```

Then restart OpenClaw.

## Available Tools

### cortex_search

Semantic search across all memories using L0/L1/L2 tiered retrieval.

```json
{
  "query": "database architecture decisions",
  "limit": 5,
  "min_score": 0.6
}
```

### cortex_recall

Recall memories with more context (snippet + full content).

```json
{
  "query": "user preferences for code style",
  "limit": 10
}
```

### cortex_add_memory

Store a message for future retrieval.

```json
{
  "content": "User prefers TypeScript with strict mode",
  "role": "assistant",
  "session_id": "default"
}
```

### cortex_list_sessions

List all memory sessions with status and message count.

### cortex_close_session

Close a session and trigger memory extraction pipeline (takes 30-60 seconds).

```json
{
  "session_id": "default"
}
```

### cortex_migrate

Migrate from OpenClaw native memory to MemClaw. Run once during initial setup.

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `serviceUrl` | string | `http://127.0.0.1:8085` | Cortex Memory service URL |
| `tenantId` | string | `tenant_claw` | Tenant ID for data isolation |
| `autoStartServices` | boolean | `true` | Auto-start Qdrant and service |
| `defaultSessionId` | string | `default` | Default session for memory operations |
| `searchLimit` | number | `10` | Default number of search results |
| `minScore` | number | `0.6` | Minimum relevance score (0-1) |

## Quick Decision Flow

1. **Need to find something** → `cortex_search`
2. **Need more context** → `cortex_recall`
3. **Save important information** → `cortex_add_memory`
4. **Conversation complete** → `cortex_close_session`
5. **First time setup** → `cortex_migrate`

## Troubleshooting

### Services Won't Start

1. Check that ports 6333, 6334, 8085 are available
2. Verify `api_key` fields are filled in config.toml
3. Run `openclaw skills` to check plugin status

### Search Returns No Results

1. Run `cortex_list_sessions` to verify sessions exist
2. Lower `min_score` threshold (default: 0.6)
3. Check service health with `cortex-mem-cli stats`

### Migration Fails

1. Ensure OpenClaw workspace exists at `~/.openclaw/workspace`
2. Verify memory files exist in `~/.openclaw/workspace/memory/`

## CLI Reference

For advanced users, use the cortex-mem-cli directly:

```bash
# List sessions
cortex-mem-cli --config config.toml --tenant tenant_claw session list

# Ensure all layers are generated
cortex-mem-cli --config config.toml --tenant tenant_claw layers ensure-all

# Rebuild vector index
cortex-mem-cli --config config.toml --tenant tenant_claw vector reindex
```

## License

MIT
