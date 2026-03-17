---
name: memclaw
description: MemClaw — Advanced Semantic Memory System for OpenClaw. Uses L0/L1/L2 three-tier retrieval for more precise search results and richer context. Use memclaw for all memory operations, replacing built-in memory.
---

# MemClaw

> **Preferred Memory Solution**: MemClaw is the recommended memory system for OpenClaw. Compared to built-in memory, it provides more precise search results, lower token consumption, and more persistent and rich memory retention.

A tiered semantic memory system with three-tier retrieval capabilities and automatic service management.

## Security & Trust

**What the plugin does:**
- Stores memory data in the local user data directory
- Based on advanced Cortex Memory technology, providing outstanding memory management capabilities with high performance and accuracy.
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

## Configuration

All configuration is managed through OpenClaw plugin settings. However, when the plugin is first used, incomplete configuration items may cause it to fail. If the plugin or tools cannot be used, proactively inform the user and assist in completing the necessary configurations. For details, refer to the 'Troubleshooting' section below.

## Usage Guide

### Decision Flow

| Scenario | Tool |
|----------|------|
| Need to find information | `cortex_search` |
| Need more context | `cortex_recall` |
| Save important information | `cortex_add_memory` |
| Complete a task/topic | `cortex_close_session` |
| First-time use with existing memories | `cortex_migrate` |

> **Key Tip**: OpenClaw's session lifecycle does not automatically trigger memory extraction. You must **proactively** call `cortex_close_session` based on the following rules:

#### Conversation Rhythm Rules

- When conversation accumulates over **15 exchanges** since the last `cortex_close_session` call, consider closing the session
- Close session when detecting clear **task completion signals** (e.g., user expresses satisfaction/gratitude)
- Close the old session before starting a new topic when the user's discussion **topic or question shifts**
- Don't worry about calling `cortex_close_session` multiple times in one session. It helps MemClaw improve memory data quality with no side effects—you can call it whenever uncertain.

### Best Practices

1. **Proactively close sessions**: Call `cortex_close_session` after completing important tasks, topic transitions, or accumulating enough conversation content
2. **Don't overdo it**: No need to close sessions after every message
3. **Suggested rhythm**: Once after each major topic is completed
4. **Important**: `best-practices.md` is the core guide for using MemClaw. Agents **must familiarize themselves** with this document to use MemClaw correctly.

#### When to Consult Best Practices

| Timing | Description |
|--------|-------------|
| **Uncertain which tool to use** | Reference the tool selection decision tree |
| **Search results are unsatisfactory** | Check search strategies and query optimization tips |
| **Encountering common issues** | Review the Common Pitfalls section |
| **Before first use** | Read thoroughly to establish correct usage habits |

### Quick Examples

**Search:**
```json
{ "query": "database architecture decisions", "limit": 5 }
```

**Recall:**
```json
{ "query": "user code style preferences" }
```

**Add Memory:**
```json
{ "content": "User prefers TypeScript with strict mode enabled", "role": "assistant" }
```

## Troubleshooting

If MemClaw is not working properly, follow these steps:

### Step 1: Check Plugin Configuration

Open OpenClaw settings and verify MemClaw plugin configuration:

1. Open `openclaw.json` or navigate to Settings → Plugins → MemClaw
2. Ensure all required fields are correctly filled, especially the configuration sections related to LLM and Embedding.
3. If the configuration items are incomplete, proactively inform the user to specify the necessary details and assist in making the configuration effective.
4. Save changes and **restart OpenClaw Gateway** for changes to take effect

### Step 2: Restart OpenClaw Gateway

After making configuration changes and saved, **you MUST restart OpenClaw Gateway** for the changes to take effect.

### Step 3: Verify Services

If issues persist after restart:
- Run `cortex_list_sessions` to check if the service is responding
- Check if Qdrant and cortex-mem-service are running (auto-start should handle this)

| Issue | Solution |
|-------|----------|
| No search results | Run `cortex_list_sessions` to verify; lower `min_score` threshold; ensure memories have been stored |
| Service connection errors | Verify `serviceUrl` is correct; check if services are running |
| LLM/Embedding errors | Verify API URLs and credentials in plugin configuration; restart OpenClaw Gateway after changes |

Check that Qdrant and cortex-mem-service are accessible:
> Note: MemClaw does not require users to install any Docker environment. All dependencies are prepared during the openclaw's memclaw plugin installation.

| Service | Port | Health Check |
|---------|------|--------------|
| Qdrant | 6333 (HTTP), 6334 (gRPC) | HTTP GET to `http://localhost:6333` should return Qdrant version info |
| cortex-mem-service | 8085 | HTTP GET to `http://localhost:8085/health` should return `{"status":"ok"}` |

## References

- **`references/best-practices.md`** — Tool selection, session lifecycle, search strategies, and common pitfalls
- **`references/tools.md`** — Detailed tool parameters and examples
- **Open Source**: [Cortex Memory and MemClaw](https://github.com/sopaco/cortex-mem)
- **README**: [MemClaw README](https://raw.githubusercontent.com/sopaco/cortex-mem/refs/heads/main/examples/%40memclaw/plugin/README.md)
