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

All configuration is managed through OpenClaw plugin settings. To view or modify:

1. Open OpenClaw settings (`openclaw.json` or via UI)
2. Navigate to Plugins → MemClaw → Configuration
3. Verify or update the required fields
4. Save changes and **restart OpenClaw Gateway** for changes to take effect

## Usage Guide

### Decision Flow

| Scenario | Tool |
|----------|------|
| Need to find information | `cortex_search` |
| Need more context | `cortex_recall` |
| Save important information | `cortex_add_memory` |
| Complete a task/topic | `cortex_close_session` |
| First-time use with existing memories | `cortex_migrate` |

> **Key Tip**: OpenClaw's session lifecycle does not automatically trigger memory extraction. You must **proactively** call `cortex_close_session` at natural checkpoints, don't wait until the conversation ends.

### Best Practices

1. **Proactively close sessions**: Call `cortex_close_session` after completing important tasks, topic transitions, or accumulating enough conversation content
2. **Don't overdo it**: No need to close sessions after every message
3. **Suggested rhythm**: Once after each major topic is completed

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
3. Save the configuration file

### Step 2: Restart OpenClaw Gateway

After making configuration changes, **you MUST restart OpenClaw Gateway** for the changes to take effect.

### Step 3: Verify Services

If issues persist after restart:
- Run `cortex_list_sessions` to check if the service is responding
- Check if Qdrant and cortex-mem-service are running (auto-start should handle this)

| Issue | Solution |
|-------|----------|
| No search results | Run `cortex_list_sessions` to verify; lower `min_score` threshold; ensure memories have been stored |
| Service connection errors | Verify `serviceUrl` is correct; check if services are running |
| LLM/Embedding errors | Verify API URLs and credentials in plugin configuration; restart OpenClaw Gateway after changes |

## References

- **`references/best-practices.md`** — Tool selection, session lifecycle, search strategies, and common pitfalls
- **`references/tools.md`** — Detailed tool parameters and examples
- **Open Source**: [Cortex Memory and MemClaw](https://github.com/sopaco/cortex-mem)
- **README**: [MemClaw README](https://raw.githubusercontent.com/sopaco/cortex-mem/refs/heads/main/examples/%40memclaw/plugin/README.md)
