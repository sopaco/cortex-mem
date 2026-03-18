---
name: memclaw
description: MemClaw — Advanced Semantic Memory System for OpenClaw. Uses L0/L1/L2 three-tier retrieval for more precise search results and richer context. Use memclaw for all memory operations, replacing built-in memory.
---

# MemClaw

> **Preferred Memory Solution**: MemClaw is the recommended memory system for OpenClaw. Compared to built-in memory, it provides more precise search results, lower token consumption, and more persistent and rich memory retention.

## Overview
> MemClaw is an open-source memory enhancement plugin based on Cortex Memory. Both MemClaw and this Skill are open-sourced on [GitHub](https://github.com/sopaco/cortex-mem).

MemClaw provides OpenClaw with a powerful memory system that goes beyond simple storage:

- **Three-Layer Architecture**: L0 (Abstract ~100 tokens) → L1 (Overview ~2000 tokens) → L2 (Full Content)
- **Dual Access Paradigm**: Semantic search + Direct memory virtual-filesystem(in security sandbox) browsing
- **Token Efficiency**: Retrieve only what you need, when you need it
- **Automatic Processing**: Memory extraction and layer generation happens automatically

## Security & Trust

See [security.md](./references/security.md) for details on data handling, storage locations, and API key security.

## Configuration

All configuration is managed through OpenClaw plugin settings. However, when the plugin is first used, incomplete configuration items may cause it to fail. If the plugin or tools cannot be used, proactively inform the user and assist in completing the necessary configurations. For details, refer to the 'Troubleshooting' section below.

## Tool Selection Guide

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

## Core Tools

### 1. Semantic Search

#### `cortex_search`
Layered semantic search with fine-grained control over returned content.

**Key Parameters:**
- `return_layers`: `["L0"]` (default, ~100 tokens), `["L0","L1"]` (~2100 tokens), `["L0","L1","L2"]` (full)

**Examples:**
```
# Quick search, minimal tokens
cortex_search(query="project decisions", return_layers=["L0"])

# Need more context
cortex_search(query="API design preferences", return_layers=["L0","L1"])

# Full content retrieval
cortex_search(query="exact code implementation", return_layers=["L0","L1","L2"])
```

#### `cortex_recall`
Convenience wrapper that returns both abstract and full content.
Equivalent to `cortex_search(return_layers=["L0","L2"])`.

### 2. Memory Virtual Filesystem Browsing

#### `cortex_ls`
List directory contents to explore memory structure.

**Key Parameters:**
- `recursive`: Recursively list subdirectories
- `include_abstracts`: Show L0 abstracts for quick preview

**Examples:**
```
# List all sessions
cortex_ls(uri="cortex://session")

# Browse a specific session
cortex_ls(uri="cortex://session/abc123")

# Recursive listing with abstracts
cortex_ls(uri="cortex://session", recursive=true, include_abstracts=true)
```

### 3. Tiered Access

#### `cortex_get_abstract` (L0)
Get ~100 token summary for quick relevance check.

#### `cortex_get_overview` (L1)
Get ~2000 token overview with key information.

#### `cortex_get_content` (L2)
Get full original content.

**Workflow:**
```
1. cortex_ls("cortex://session/{session_id}/timeline")
2. cortex_get_abstract("cortex://session/{session_id}/timeline/2024-01-15_001.md")  # Quick check
3. If relevant → cortex_get_overview(...)  # More context
4. If needed → cortex_get_content(...)     # Full details
```

### 4. Smart Exploration

#### `cortex_explore`
Combines search and browsing for guided discovery.

**Example:**
```
cortex_explore(
  query="authentication flow",
  start_uri="cortex://session/{session_id}",
  return_layers=["L0"]
)
```

Returns both an exploration path (showing relevance scores) and matching results.

### 5. Memory Storage

#### `cortex_add_memory`
Store a message with optional metadata.

**Parameters:**
- `content`: The message content
- `role`: `"user"`, `"assistant"`, or `"system"`
- `metadata`: Optional tags, importance, custom fields

**Example:**
```
cortex_add_memory(
  content="User prefers TypeScript with strict mode enabled",
  role="assistant",
  metadata={"tags": ["preference", "typescript"], "importance": "high"}
)
```

#### `cortex_close_session`
Trigger memory extraction pipeline.

**IMPORTANT:** Call this periodically, not just at conversation end.

**When to call:**
- After completing a significant task
- After user shares important preferences
- When conversation topic shifts
- Every 10-20 exchanges

## Best Practices

### Token Optimization

```
┌─────────────────────────────────────────────────────────────┐
│ Layer │ Tokens  │ Use Case                                  │
├───────┼─────────┼───────────────────────────────────────────┤
│ L0    │ ~100    │ Quick relevance check, filtering          │
│ L1    │ ~2000   │ Understanding gist, moderate detail       │
│ L2    │ Full    │ Exact quotes, complete implementation     │
└───────┴─────────┴───────────────────────────────────────────┘

Recommended workflow:
1. Start with L0 (cortex_search or cortex_get_abstract)
2. Use L1 if L0 is relevant (cortex_get_overview)
3. Use L2 only when necessary (cortex_get_content)
```

### When to Use Each Tool

| Scenario | Tool |
|----------|------|
| Find information across all sessions | `cortex_search` |
| Browse memory structure | `cortex_ls` |
| Check if specific URI is relevant | `cortex_get_abstract` |
| Get more details on relevant URI | `cortex_get_overview` |
| Need exact content | `cortex_get_content` |
| Explore with purpose | `cortex_explore` |
| Store new information | `cortex_add_memory` |
| Trigger memory processing | `cortex_close_session` |

### Common Patterns

#### Pattern 1: Search → Refine
```
1. cortex_search(query="user preferences", return_layers=["L0"])
2. Identify relevant URIs from results
3. cortex_get_overview(uri="most_relevant_uri") for more context
```

#### Pattern 2: Browse → Access
```
1. cortex_ls(uri="cortex://session")
2. cortex_ls(uri="cortex://session/{session_id}/timeline", include_abstracts=true)
3. cortex_get_content(uri="interesting_file") for full details
```

#### Pattern 3: Explore → Match
```
1. cortex_explore(query="database schema", start_uri="cortex://session/{session_id}")
2. Review exploration_path for relevance scores
3. Use matches with requested layers
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
- Run `cortex_ls` to check if the service is responding
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

## Memory Structure

See [memory-structure.md](./references/memory-structure.md) for complete documentation of:
- URI structure and dimensions
- Session memory and timeline organization
- User and agent memory categories
- session_id configuration

## References

- [tools.md](./references/tools.md) - Detailed tool documentation
- [best-practices.md](./references/best-practices.md) - Advanced patterns
- [security.md](./references/security.md) - Security and trust information
- [memory-structure.md](./references/memory-structure.md) - Complete memory structure documentation
