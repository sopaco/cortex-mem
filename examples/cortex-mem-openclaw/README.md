# Cortex Memory Plugin for OpenClaw

English | [中文](./README_zh.md)

A plugin that provides layered semantic memory capabilities for OpenClaw, supporting L0/L1/L2 tiered retrieval.

## Features

- **Layered Semantic Search**: L0 (Abstract) → L1 (Overview) → L2 (Full Content) tiered retrieval
- **Automatic Vectorization**: Messages are automatically embedded for semantic similarity search
- **Session Isolation**: Supports independent memory spaces across multiple sessions
- **HTTP API**: Communicates with the core service via cortex-mem-service REST API

## Prerequisites

1. **cortex-mem-service** running
   ```bash
   # From cortex-mem project root
   cargo run -p cortex-mem-service -- --data-dir ./cortex-data
   ```

2. **Qdrant Vector Database** (optional, for vector search)
3. **Embedding Service** (OpenAI-compatible API)

## Installation

### Option 1: Local Link Installation (Development Mode)

```bash
# From cortex-mem project root
cd examples/cortex-mem-openclaw

# Install dependencies and build
npm install
npm run build

# Link to OpenClaw
openclaw plugins install --link $(pwd)
```

### Option 2: Install from npm (After Publishing)

```bash
openclaw plugins install @cortex-mem/openclaw-plugin
```

## Configuration

Add the following to your OpenClaw configuration file (`~/.openclaw/openclaw.json` or project directory):

```json
{
  "plugins": {
    "entries": {
      "cortex-mem": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://127.0.0.1:8085",
          "tenantId": "tenant_claw",
          "defaultSessionId": "default",
          "searchLimit": 10,
          "minScore": 0.6
        }
      }
    }
  }
}
```

### Configuration Reference

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `serviceUrl` | string | `http://127.0.0.1:8085` | HTTP endpoint of cortex-mem-service |
| `tenantId` | string | `tenant_claw` | Tenant identifier for data isolation (Qdrant collection & filesystem) |
| `defaultSessionId` | string | `default` | Default session ID |
| `searchLimit` | integer | 10 | Maximum number of search results |
| `minScore` | number | 0.6 | Minimum relevance score threshold |

## Tools

### cortex_search

Layered semantic search returning relevant memory snippets.

```json
{
  "query": "user preferences settings",
  "scope": "session-123",  // optional, limits search scope
  "limit": 10,
  "min_score": 0.6
}
```

Response:
```json
{
  "results": [
    {
      "uri": "cortex://session/abc/timeline/2026-03/11/10_30_00_xxx.md",
      "score": 0.89,
      "snippet": "User prefers dark theme..."
    }
  ],
  "total": 1
}
```

### cortex_recall

Layered memory recall with configurable detail levels.

```json
{
  "query": "project architecture decisions",
  "layers": ["L0", "L1"],  // L0=Abstract, L1=Overview, L2=Full Content
  "scope": "session-123",
  "limit": 5
}
```

### cortex_add_memory

Add a memory to a specific session.

```json
{
  "content": "User selected PostgreSQL as the primary database",
  "role": "assistant",  // user/assistant/system
  "session_id": "session-123"  // optional, uses defaultSessionId if not specified
}
```

### cortex_list_sessions

List all memory sessions.

```json
{}
```

Response:
```json
{
  "sessions": [
    {
      "thread_id": "session-123",
      "status": "active",
      "message_count": 42,
      "created_at": "2026-03-11T10:00:00Z"
    }
  ]
}
```

## Architecture

```
OpenClaw Gateway
       │
       ▼
cortex-mem-plugin (TypeScript)
       │
       ▼ HTTP REST
cortex-mem-service (Rust)
       │
       ├─► Qdrant (Vector Store)
       ├─► CortexFilesystem (File Store)
       └─► LLM (Layer Generation)
```

## Development

```bash
# Install dependencies
npm install

# Development mode (watch compilation)
npm run dev

# Build
npm run build

# Run tests
npm test
```

## License

MIT
