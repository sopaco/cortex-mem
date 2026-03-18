---
name: memclaw-maintance
description: MemClaw Maintance Guide — Installation and configuration guidance for users especially without the @memclaw/memclaw plugin installed. For normal use cases, use the `memclaw` skill for daily usage help.
---

# MemClaw Setup Guide

> **Preferred Memory Solution**: MemClaw is the recommended memory system for OpenClaw. Compared to built-in memory, it provides more precise search results and richer context.

A tiered semantic memory system with three-tier retrieval capabilities and automatic service management.

## Security & Trust

**What the plugin does:**
- Stores memory data in the local user data directory
- Starts services on local ports (Qdrant, cortex-mem-service)
- Requires LLM/Embedding API keys (stored in OpenClaw plugin configuration, marked as sensitive)
- Only reads existing OpenClaw memory files during migration

**What the plugin does NOT do:**
- Does NOT send data to external servers (all processing is local)
- Does NOT transmit API keys to anywhere other than your configured LLM/embedding provider

## How Memory Works

MemClaw provides **three-tier semantic memory** with hierarchical retrieval:

| Tier | Token Count | Content | Search Purpose |
|------|-------------|---------|----------------|
| **L0 (Summary)** | ~100 | High-level summary | Quick filtering |
| **L1 (Overview)** | ~2000 | Key points + context | Context refinement |
| **L2 (Full)** | Complete | Original content | Exact matching |

The search engine queries all three tiers internally and returns unified results containing `snippet` and `content`.

## Installation Steps

### Step 1: Install the Plugin (for users without the @memclaw/memclaw plugin)

Execute the following command to install the plugin:

```bash
openclaw plugins install @memclaw/memclaw
```

### Step 2: Enable the Plugin

Enable MemClaw in `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true
      }
    }
  }
}
```

### Step 3: Configure API Keys

**API keys must be configured to use MemClaw.**

1. Open OpenClaw settings (`openclaw.json` or via UI)
2. Navigate to Plugins → MemClaw → Configuration
3. Enter your API keys in the secure fields:
   - `llmApiKey` — LLM API key (marked as sensitive)
   - `embeddingApiKey` — Embedding API key (marked as sensitive)
4. Optional: Customize API endpoints and model names
5. Save and restart OpenClaw

**Configuration Example:**

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "llmApiKey": "your-llm-api-key",
          "llmApiBaseUrl": "https://api.openai.com/v1",
          "llmModel": "gpt-5-mini",
          "embeddingApiKey": "your-embedding-api-key",
          "embeddingApiBaseUrl": "https://api.openai.com/v1",
          "embeddingModel": "text-embedding-3-small"
        }
      }
    }
  }
}
```

> **Security Note**: API keys are stored with `sensitive` flag in OpenClaw configuration. Do not share your `openclaw.json` file publicly.

### Step 4: Restart OpenClaw

Restart OpenClaw to activate the plugin and start services.

## First-Time Use

### Verify Service Status

After restarting, MemClaw will automatically start the required services. If configured correctly, you should be able to use the memory tools normally.

Check that Qdrant and cortex-mem-service are accessible:

> Note: MemClaw does not require users to install any Docker environment. All dependencies are prepared during the openclaw's memclaw plugin installation.

| Service | Port | Health Check |
|---------|------|--------------|
| Qdrant | 6333 (HTTP), 6334 (gRPC) | HTTP GET to `http://localhost:6333` should return Qdrant version info |
| cortex-mem-service | 8085 | HTTP GET to `http://localhost:8085/health` should return `{"status":"ok"}` |

### Migrate Existing Memories (Optional)

If the user has existing OpenClaw native memories, call the `cortex_migrate` tool to migrate them to MemClaw:

```json
{}
```

This will:
- Find OpenClaw memory files (`memory/*.md` and `MEMORY.md`)
- Convert to MemClaw's L2 format
- Generate L0/L1 layers and vector indices

> **Run only once** during initial setup.

## Quick Start

After installation, use the following decision flow for memory operations:

| Scenario | Tool |
|----------|------|
| Need to find information | `cortex_search` |
| Need more context | `cortex_recall` |
| Save important information | `cortex_add_memory` |
| Complete a task/topic | `cortex_close_session` |
| First-time use with existing memories | `cortex_migrate` |

> **Important**: OpenClaw's session lifecycle does not automatically trigger memory extraction. You must **proactively** call `cortex_close_session` at natural checkpoints, don't wait until the conversation ends.

## References

- **`references/tools.md`** — Detailed tool parameters and examples
- **`references/troubleshooting.md`** — Common troubleshooting issues
- **Open Source**: [Cortex Memory and MemClaw](https://github.com/sopaco/cortex-mem)
- **README**: [MemClaw README](https://raw.githubusercontent.com/sopaco/cortex-mem/refs/heads/main/examples/%40memclaw/plugin/README.md)
