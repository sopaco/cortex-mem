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
- **Cross-Platform**: Supports Windows x64, macOS Apple Silicon, and Linux x64

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
         ├── cortex_search        → Layered semantic search
         ├── cortex_recall        → Recall with context
         ├── cortex_add_memory    → Store memories
         ├── cortex_commit_session → Commit & extract
         ├── cortex_migrate       → Migrate existing memory
         ├── cortex_maintenance   → Periodic maintenance
         ├── cortex_ls            → Browse memory filesystem
         ├── cortex_get_abstract  → L0 quick preview
         ├── cortex_get_overview  → L1 moderate detail
         ├── cortex_get_content   → L2 full content
         └── cortex_explore       → Smart exploration
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
| **Platforms** | Windows x64, macOS Apple Silicon, Linux x64 |
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
          "llmModel": "gpt-5-mini",
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

Layered semantic search with fine-grained control over returned content.

**Key Parameters:**
- `return_layers`: `["L0"]` (default, ~100 tokens), `["L0","L1"]` (~2100 tokens), `["L0","L1","L2"]` (full)

```json
{
  "query": "database architecture decisions",
  "limit": 5,
  "min_score": 0.6,
  "return_layers": ["L0"]
}
```

For more context, use `return_layers: ["L0","L1"]`. For full content, use `["L0","L1","L2"]`.

### cortex_recall

Recall memories with more context (snippet + full content).

```json
{
  "query": "user preferences for code style",
  "limit": 10
}
```

### cortex_add_memory

Store a message for future retrieval with optional metadata.

```json
{
  "content": "User prefers TypeScript with strict mode",
  "role": "assistant",
  "session_id": "default",
  "metadata": {
    "tags": ["preference", "typescript"],
    "importance": "high"
  }
}
```

**Parameters:**
- `content`: The message content (required)
- `role`: `"user"`, `"assistant"`, or `"system"` (default: user)
- `session_id`: Session/thread ID (uses default if not specified)
- `metadata`: Optional metadata like tags, importance, or custom fields



### cortex_commit_session

Commit a session and trigger memory extraction pipeline (takes 30-60 seconds).

```json
{
  "session_id": "default"
}
```

> **Important**: Call this tool proactively at natural checkpoints, not just when the conversation ends. Ideal timing: after completing important tasks, topic transitions, or accumulating enough conversation content.

### cortex_ls

List directory contents to browse the memory space like a virtual filesystem.

```json
{
  "uri": "cortex://session",
  "recursive": false,
  "include_abstracts": false
}
```

**Parameters:**
- `uri`: Directory URI to list (default: `cortex://session`)
- `recursive`: List all subdirectories recursively
- `include_abstracts`: Show L0 abstracts for quick preview

**Common URIs:**
- `cortex://session` - List all sessions
- `cortex://session/{session_id}` - Browse a specific session
- `cortex://session/{session_id}/timeline` - View timeline messages
- `cortex://user/{user_id}/preferences` - View user preferences (extracted memories)
- `cortex://user/{user_id}/entities` - View user entities (people, projects, etc.)
- `cortex://agent/{agent_id}/cases` - View agent problem-solution cases

### cortex_get_abstract

Get L0 abstract layer (~100 tokens) for quick relevance checking.

```json
{
  "uri": "cortex://session/abc123/timeline/2024-01-15_001.md"
}
```

Use this to quickly determine if content is relevant before reading more.

### cortex_get_overview

Get L1 overview layer (~2000 tokens) with core information and context.

```json
{
  "uri": "cortex://session/abc123/timeline/2024-01-15_001.md"
}
```

Use when you need more detail than the abstract but not the full content.

### cortex_get_content

Get L2 full content layer - the complete original content.

```json
{
  "uri": "cortex://session/abc123/timeline/2024-01-15_001.md"
}
```

Use only when you need the complete, unprocessed content.

### cortex_explore

Smart exploration combining search and browsing for guided discovery.

```json
{
  "query": "authentication flow",
  "start_uri": "cortex://session",
  "return_layers": ["L0"]
}
```

Returns an exploration path with relevance scores and matching results.

### cortex_migrate

Migrate from OpenClaw native memory to MemClaw. Run once during initial setup.

### cortex_maintenance

Perform periodic maintenance on MemClaw data (prune, reindex, ensure-all layers).

```json
{
  "dryRun": false,
  "commands": ["prune", "reindex", "ensure-all"]
}
```

**Parameters:**
- `dryRun`: Preview changes without executing (default: false)
- `commands`: Which maintenance commands to run (default: all)

This tool runs automatically every 3 hours. Call manually when search results seem incomplete or stale.

## Quick Decision Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     How to Access Memories                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Do you know WHERE the information is?                          │
│       │                                                          │
│       ├── YES ──► Use Direct Tiered Access                       │
│       │           cortex_ls → cortex_get_abstract/overview/content│
│       │                                                          │
│       └── NO ──► Do you know WHAT you're looking for?            │
│                    │                                             │
│                    ├── YES ──► Use Semantic Search               │
│                    │            cortex_search                     │
│                    │                                             │
│                    └── NO ──► Use Exploration                    │
│                                 cortex_explore                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

| Scenario | Tool |
|----------|------|
| Find information across all sessions | `cortex_search` |
| Browse memory structure | `cortex_ls` |
| Quick relevance check for URI | `cortex_get_abstract` |
| Get more details on relevant URI | `cortex_get_overview` |
| Need exact full content | `cortex_get_content` |
| Explore with purpose | `cortex_explore` |
| Save important information | `cortex_add_memory` |
| Complete a task/topic | `cortex_commit_session` |
| First-time use with existing memories | `cortex_migrate` |
| Data maintenance | `cortex_maintenance` |

For detailed guidance on tool selection, session lifecycle, and best practices, see the [Skills Documentation](skills/memclaw/SKILL.md).

## Troubleshooting

### Plugin Not Working

### Services Won't Start

1. Check that ports 6333, 6334, 8085 are available
2. Verify LLM and Embedding credentials are configured correctly
3. Run `openclaw skills` to check plugin status

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
