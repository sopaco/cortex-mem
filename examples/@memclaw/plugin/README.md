# MemClaw

[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Layered semantic memory plugin for OpenClaw with L0/L1/L2 tiered retrieval, automatic service management, and migration support from OpenClaw native memory.

## Overview

MemClaw is an OpenClaw plugin that provides advanced semantic memory capabilities using Cortex Memory's tiered memory architecture. It stores, searches, and recalls memories with intelligent layer-based retrieval that balances speed and context.

## Features

- **Three-Layer Memory Architecture**: L0 (abstract), L1 (overview), and L2 (full) layers for intelligent retrieval
- **Automatic Service Management**: Auto-starts Qdrant vector database and cortex-mem-service
- **Semantic Search**: Vector-based similarity search across all memory layers
- **Session Management**: Create, list, and close memory sessions
- **Migration Support**: One-click migration from OpenClaw native memory
- **Easy Configuration**: Configure LLM/Embedding directly through OpenClaw plugin settings
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
| **Node.js** | ≥ 20.0.0 |
| **OpenClaw** | Installed and configured |

### Install Plugin

```bash
openclaw plugins install @memclaw/memclaw
```

### Local Development Installation

For developers who want to use a local version of memclaw or develop the plugin:

```bash
# Clone the repository
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem/examples/@memclaw/plugin

# Install dependencies
bun install

# Build the plugin
bun run build
```

**Option A: Use plugins.load.paths**

```json
{
  "plugins": {
    "load": {
      "paths": ["/path/to/cortex-mem/examples/@memclaw/plugin"]
    },
    "entries": {
      "memclaw": { "enabled": true }
    }
  }
}
```

**Option B: Symlink to extensions directory**

```bash
mkdir -p ~/.openclaw/extensions
ln -sf "$(pwd)" ~/.openclaw/extensions/memclaw
```

Then enable in `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "memclaw": { "enabled": true }
    }
  }
}
```

After making code changes, rebuild with `bun run build` and restart OpenClaw.

## Configuration

### Plugin Configuration

Configure MemClaw directly through OpenClaw plugin settings in `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://localhost:8085",
          "tenantId": "tenant_claw",
          "autoStartServices": true,
          "llmApiBaseUrl": "https://api.openai.com/v1",
          "llmApiKey": "your-llm-api-key",
          "llmModel": "gpt-4o-mini",
          "embeddingApiBaseUrl": "https://api.openai.com/v1",
          "embeddingApiKey": "your-embedding-api-key",
          "embeddingModel": "text-embedding-3-small"
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

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `serviceUrl` | string | `http://localhost:8085` | Cortex Memory service URL |
| `tenantId` | string | `tenant_claw` | Tenant ID for data isolation |
| `autoStartServices` | boolean | `true` | Auto-start Qdrant and service |
| `defaultSessionId` | string | `default` | Default session for memory operations |
| `searchLimit` | number | `10` | Default number of search results |
| `minScore` | number | `0.6` | Minimum relevance score (0-1) |
| `qdrantPort` | number | `6334` | Qdrant port (gRPC) |
| `servicePort` | number | `8085` | cortex-mem-service port |
| `llmApiBaseUrl` | string | `https://api.openai.com/v1` | LLM API endpoint URL |
| `llmApiKey` | string | - | LLM API key (required) |
| `llmModel` | string | `gpt-5-mini` | LLM model name |
| `embeddingApiBaseUrl` | string | `https://api.openai.com/v1` | Embedding API endpoint URL |
| `embeddingApiKey` | string | - | Embedding API key (required) |
| `embeddingModel` | string | `text-embedding-3-small` | Embedding model name |

### Configuration via UI

You can also configure the plugin through OpenClaw UI:

1. Open OpenClaw Settings (`openclaw.json` or via UI)
2. Navigate to Plugins → MemClaw → Configuration
3. Fill in the required fields for LLM and Embedding
4. Save and **restart OpenClaw Gateway** for changes to take effect

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

> **Important**: Call this tool proactively at natural checkpoints, not just when the conversation ends. Ideal timing: after completing important tasks, topic transitions, or accumulating enough conversation content.

### cortex_migrate

Migrate from OpenClaw native memory to MemClaw. Run once during initial setup.

### cortex_maintenance

Perform periodic maintenance on MemClaw data (prune, reindex, ensure-all layers).

## Quick Decision Flow

| Scenario | Tool |
|----------|------|
| Need to find information | `cortex_search` |
| Need more context | `cortex_recall` |
| Save important information | `cortex_add_memory` |
| Complete a task/topic | `cortex_close_session` |
| First-time use with existing memories | `cortex_migrate` |

For detailed guidance on tool selection, session lifecycle, and best practices, see the [Skills Documentation](skills/memclaw/SKILL.md).

## Troubleshooting

### Plugin Not Working

1. **Check Configuration**: Open OpenClaw settings and verify MemClaw plugin configuration, especially LLM and Embedding settings
2. **Restart OpenClaw Gateway**: Configuration changes require a gateway restart to take effect
3. **Verify Services**: Run `cortex_list_sessions` to check if the service is responding

### Services Won't Start

1. Check that ports 6333, 6334, 8085 are available
2. Verify LLM and Embedding credentials are configured correctly
3. Run `openclaw skills` to check plugin status

### Search Returns No Results

1. Run `cortex_list_sessions` to verify sessions exist
2. Lower `min_score` threshold (default: 0.6)
3. Ensure memories have been stored (run `cortex_close_session` to extract memories)

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

## Documentation

- **[Skills Documentation](skills/memclaw/SKILL.md)** — Agent skill guide with troubleshooting
- **[Best Practices](skills/memclaw/references/best-practices.md)** — Tool selection, session lifecycle, search strategies
- **[Tools Reference](skills/memclaw/references/tools.md)** — Detailed tool parameters and examples

## License

MIT
