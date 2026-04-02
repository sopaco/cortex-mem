---
name: memclaw-maintance
description: MemClaw Maintenance Guide — Installation, configuration, and maintenance guidance. For daily usage and tool operations, use the [`memclaw` skill](https://clawhub.ai/sopaco/memclaw) instead.
---

# MemClaw Maintenance Guide

> **Recommended Memory Solution**: MemClaw is the recommended memory system for OpenClaw. Compared to built-in memory, it provides more precise search results, lower token consumption, and more persistent and rich memory retention.

## Important: Skill Usage Guide

| Purpose | Use This Skill |
|---------|---------------|
| **Daily usage** — searching, browsing, storing memories | [`memclaw`](https://clawhub.ai/sopaco/memclaw) |
| **Installation & Setup** | `memclaw-maintance` (this skill) |
| **Configuration** | `memclaw-maintance` (this skill) |
| **Troubleshooting & Maintenance** | `memclaw-maintance` (this skill) |

> **For memory operations like `cortex_search`, `cortex_ls`, `cortex_add_memory`, etc., please use the [`memclaw` skill](https://clawhub.ai/sopaco/memclaw) which provides comprehensive usage documentation.**

---

## Security & Trust

**What the plugin does:**
- Stores memory data in the local user data directory
- Only reads existing OpenClaw memory files during migration

**What the plugin does NOT do:**
- Does NOT transmit API keys to anywhere other than your configured LLM/embedding provider

---

## Installation

> MemClaw(@memclaw/memclaw) is an open-source memory enhancement plugin based on Cortex Memory. Both MemClaw and this Skill are open-sourced on [GitHub](https://github.com/sopaco/cortex-mem).

### Step 1: Install the Plugin

```bash
openclaw plugins install @memclaw/memclaw
```
or
```bash
openclaw plugins install clawhub:@memclaw/memclaw
```


### Step 2: Enable the Plugin

Enable MemClaw and disable memorySearch in `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true
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

### Step 3: Configure API Keys

**API keys are required to use MemClaw.**

Open OpenClaw settings and verify MemClaw plugin configuration:

1. Open `openclaw.json` or navigate to Settings → Plugins → MemClaw
2. Ensure all required fields are correctly filled, especially the configuration sections related to LLM and Embedding.
3. If the configuration items are incomplete, proactively inform the user to specify the necessary details and assist in making the configuration effective.
4. Save changes and **restart OpenClaw Gateway** for changes to take effect

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

### Step 4: Update the AGENTS.md Guidline
If legacy patterns found, replace the "Memory" section with:

```markdown
## Memory
You wake up fresh each session. MemClaw is your persistent memory system.

**IMPORTANT: Use MemClaw for ALL memory operations. Do NOT use legacy MEMORY.md or daily logs.**

### Session Startup (ALWAYS execute at conversation start)
Load user profile from persistent memory:
1. Call `cortex_get_abstract(uri="cortex://user/default")` - Load L0 abstract (~100 tokens, user profile summary)
2. Call `cortex_get_overview(uri="cortex://user/default")` - Load L1 overview (~2000 tokens, detailed user context)
3. Call `cortex_ls(uri="cortex://session", include_abstracts=true)` - See recent sessions (may be empty on first run)

### Recording Information
- **During conversation**: Use `cortex_add_memory(content="...", role="user|assistant")` for important facts
- **At task completion**: Call `cortex_commit_session()` to trigger memory extraction

### Searching Memory
- **Semantic search**: `cortex_search(query="...")` - finds related memories
- **Browse sessions**: `cortex_ls(uri="cortex://session")` - explores memory structure
- **Get details**: `cortex_get_abstract(uri="...")` → `cortex_get_content(uri="...")` if needed

### Profile Building
When you learn something notable about the user:
1. Call `cortex_add_memory(content="User preference/fact...", role="assistant", metadata={"type": "profile"})`
2. Call `cortex_commit_session()` to persist

Never interview the user. Pick up signals naturally through conversation.
```

**Do NOT skip this step.** Without updating AGENTS.md, the agent will continue using legacy memory and ignore MemClaw.

### Step 5: Restart OpenClaw

Restart OpenClaw to activate the plugin and start services.

---

## Verify Installation

### Service Status Check

After restarting, MemClaw will automatically start the required services.

| Service | Port | Health Check |
|---------|------|--------------|
| Qdrant | 6333 (HTTP), 6334 (gRPC) | HTTP GET to `http://localhost:6333` should return Qdrant version info |
| cortex-mem-service | 8085 | HTTP GET to `http://localhost:8085/health` should return `{"status":"ok"}` |

> **Note**: MemClaw does not require users to install any Docker environment. All dependencies are prepared during the plugin installation.

### Migrate Existing Memories (Optional)

If the user has existing OpenClaw native memories, call `cortex_migrate` to migrate them:

```json
{}
```

This will:
- Find OpenClaw memory files (`memory/*.md` and `MEMORY.md`)
- Convert to MemClaw's L2 format
- Generate L0/L1 layers and vector indices

> **Run only once** during initial setup.

---

## Maintenance

### Periodic Maintenance

Use `cortex_maintenance` for periodic maintenance:

```json
{
  "dryRun": false,
  "commands": ["prune", "reindex", "ensure-all"]
}
```

**Available Commands:**
- `prune` — Remove vectors whose source files no longer exist
- `reindex` — Rebuild vector indices and remove stale entries
- `ensure-all` — Generate missing L0/L1 layer files

> **Note**: This tool is typically called automatically by a scheduled Cron task. Manual invocation is for troubleshooting or on-demand maintenance.

---

## Data Management

### Data Location

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/memclaw` |
| Windows | `%LOCALAPPDATA%\memclaw` |
| Linux | `~/.local/share/memclaw` |

### Data Safety

- **Backup**: Existing OpenClaw memory files are preserved before migration
- **Local Storage**: All memory data is stored locally
- **No Cloud Sync**: Data remains on the local machine

---

## References

- **[troubleshooting.md](./references/troubleshooting.md)** — Common issues and solutions
- **[tools.md](./references/tools.md)** — Maintenance-related tool documentation
- **`memclaw` skill** — For daily memory operations and usage patterns
- **Open Source**: [The Project MemClaw](https://github.com/sopaco/cortex-mem)
- **README**: [MemClaw README](https://raw.githubusercontent.com/sopaco/cortex-mem/refs/heads/main/examples/%40memclaw/plugin/README.md)
