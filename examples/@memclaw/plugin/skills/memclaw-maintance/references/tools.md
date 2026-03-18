# Maintenance Tools Reference

> **Note**: For daily memory operations (`cortex_search`, `cortex_ls`, `cortex_add_memory`, etc.), please refer to the `memclaw` skill documentation.

This document covers maintenance and migration tools for MemClaw.

---

## cortex_migrate

Migrate from OpenClaw's native memory system to MemClaw.

**Parameters:** None

**Use Cases:**
- First-time use with existing OpenClaw memories
- Want to preserve previous conversation history
- Switching from built-in memory to MemClaw

**Execution Effects:**
1. Finds OpenClaw memory files (`memory/*.md` and `MEMORY.md`)
2. Converts to MemClaw's L2 format
3. Generates L0/L1 layers and vector indices

**Prerequisites:**
- OpenClaw workspace exists at `~/.openclaw/workspace`
- Memory files exist at `~/.openclaw/workspace/memory/`

**Example:**
```json
{}
```

> **Run only once** during initial setup.

---

## cortex_maintenance

Perform periodic maintenance on MemClaw data.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `dryRun` | boolean | No | false | Preview changes without executing |
| `commands` | array | No | `["prune", "reindex", "ensure-all"]` | Maintenance commands to execute |

**Use Cases:**
- Search results are incomplete or outdated
- Recovering from crash or data corruption
- Need to clean up disk space

**Available Commands:**
- `prune` — Remove vectors whose source files no longer exist
- `reindex` — Rebuild vector indices and remove stale entries
- `ensure-all` — Generate missing L0/L1 layer files

**Example:**
```json
{
  "dryRun": false,
  "commands": ["prune", "reindex", "ensure-all"]
}
```

> **Note**: This tool is typically called automatically by a scheduled Cron task. Manual invocation is for troubleshooting or on-demand maintenance.
