---
name: memclaw-admin
description: Install, configure, and manage MemClaw — a layered semantic memory system for OpenClaw. Use this skill when setting up MemClaw for the first time, troubleshooting issues, or migrating data from OpenClaw's native memory.
---

# MemClaw Admin Guide

This skill guides you through installing, configuring, and maintaining MemClaw.

## Overview

MemClaw provides layered semantic memory (L0/L1/L2) for OpenClaw agents. It requires:

1. **Qdrant** — Vector database for semantic search
2. **cortex-mem-service** — Memory processing service
3. **cortex-mem-cli** — Command-line tool for maintenance

All components are **single binaries** — no Docker required.

---

## 1. System Requirements

| Requirement | Details |
|-------------|---------|
| **Platforms** | Windows x64, macOS Apple Silicon (ARM64) |
| **Node.js** | ≥ 22.0.0 |
| **Memory** | ~512MB for services |
| **Disk** | Depends on memory volume |

> **Note**: Other platforms (Linux, macOS Intel, Windows ARM) are not currently supported.

---

## 2. First-time Setup

### Step 1: Install the Plugin

```bash
# From npm (after publishing)
openclaw plugins install memclaw

# Or from local directory
cd examples/memclaw
npm install && npm run build
openclaw plugins install --link $(pwd)
```

### Step 2: Configure openclaw.json

Add the plugin to your OpenClaw configuration at `~/.openclaw/openclaw.json`:

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
      "memorySearch": {
        "enabled": false
      }
    }
  }
}
```

**Important**:
- Set `memorySearch.enabled: false` to disable OpenClaw's built-in memory search
- Otherwise both memory systems will run simultaneously, causing duplicate results

**Optional plugin configuration** (all have defaults):

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "tenantId": "tenant_claw",
          "autoStartServices": true,
          "serviceUrl": "http://127.0.0.1:8085"
        }
      }
    }
  }
}
```

| Config Option | Default | Description |
|---------------|---------|-------------|
| `serviceUrl` | `http://127.0.0.1:8085` | cortex-mem-service endpoint |
| `tenantId` | `tenant_claw` | Tenant for data isolation |
| `defaultSessionId` | `default` | Default session for memories |
| `autoStartServices` | `true` | Auto-start Qdrant and service |
| `searchLimit` | `10` | Max search results |
| `minScore` | `0.6` | Min relevance threshold |

### Step 3: Configure LLM

On first run, MemClaw creates a configuration file:

| Platform | Config Path |
|----------|-------------|
| **Windows** | `%APPDATA%\memclaw\config.toml` |
| **macOS** | `~/Library/Application Support/memclaw/config.toml` |
| **Linux** | `~/.config/memclaw/config.toml` |

The file opens automatically. Fill in required fields:

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "xxx"  # REQUIRED
model_efficient = "gpt-5-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "xxx"  # REQUIRED (can be same as llm.api_key)
model_name = "text-embedding-3-small"
```

### Step 4: Restart OpenClaw

After saving the configuration, restart OpenClaw. Services start automatically.

---

## 3. Verify Installation

Check service status:

```bash
# Qdrant
curl http://localhost:6333

# cortex-mem-service
curl http://localhost:8085/health
```

Both should return successful responses.

---

## 4. Migration from OpenClaw Native Memory

If you have existing memories in OpenClaw's native format:

### What Gets Migrated

| OpenClaw | MemClaw |
|----------|---------|
| `memory/YYYY-MM-DD.md` | Session timeline (L2 files) |
| `MEMORY.md` | `users/{tenant}/preferences.md` |

### Run Migration

Call the `cortex_migrate` tool:

```json
{}
```

Or via CLI:

```bash
cortex-mem-cli --config ~/.config/memclaw/config.toml --tenant tenant_claw layers ensure-all
cortex-mem-cli --config ~/.config/memclaw/config.toml --tenant tenant_claw vector reindex
```

### Migration Notes

- Daily logs become sessions named `migrated-oc-{date}`
- Each paragraph becomes a separate L2 file
- Timestamps are approximated (daily log lacks exact times)
- L0/L1 layers and vector index are generated automatically

---

## 5. Manual Binary Installation

Binaries are bundled in platform-specific npm packages and installed automatically. If the optional dependency was not installed:

```bash
# macOS Apple Silicon
npm install @memclaw/bin-darwin-arm64

# Windows x64
npm install @memclaw/bin-win-x64
```

### Build from Source

If you prefer to build binaries yourself:

```bash
# In cortex-mem project root
cargo build --release

# Binaries will be at:
# target/release/cortex-mem-service
# target/release/cortex-mem-cli
```

For Qdrant, download from: https://github.com/qdrant/qdrant/releases

---

## 6. Service Management

### Start Services Manually

```bash
# Start Qdrant
qdrant --storage-path ~/.local/share/memclaw/data/qdrant-storage \
       --http-port 6333 --grpc-port 6334

# Start cortex-mem-service
cortex-mem-service --config ~/.config/memclaw/config.toml \
                   --data-dir ~/.local/share/memclaw/data
```

### Stop Services

Services run as background processes. Use your system's process manager:

```bash
# Linux/macOS
pkill -f qdrant
pkill -f cortex-mem-service

# Windows
taskkill /IM qdrant.exe /F
taskkill /IM cortex-mem-service.exe /F
```

---

## 7. Configuration Reference

```toml
# Qdrant Vector Database
[qdrant]
url = "http://localhost:6334"
collection_name = "memclaw"
timeout_secs = 30

# LLM for memory processing
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-xxx"
model_efficient = "gpt-5-mini"
temperature = 0.1
max_tokens = 4096

# Embedding for vector search
[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-xxx"
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

# Service settings
[server]
host = "localhost"
port = 8085

# Memory storage
[cortex]
data_dir = "/path/to/memclaw/data"
enable_intent_analysis = false
```

---

## 8. Troubleshooting

### Services Won't Start

1. **Check ports**: 6333, 6334, 8085 must be available
   ```bash
   # Linux/macOS
   lsof -i :6333 -i :8085
   
   # Windows
   netstat -ano | findstr "6333"
   ```

2. **Check config**: Ensure `llm.api_key` and `embedding.api_key` are set

3. **Check binaries**: Verify executables exist in bin directory

### Search Returns No Results

1. Run `cortex_list_sessions` to check for sessions
2. Run vector reindex: `cortex-mem-cli --config ... vector reindex`
3. Lower `min_score` threshold in search

### Migration Fails

1. Ensure OpenClaw workspace exists at `~/.openclaw/workspace`
2. Check for `memory/*.md` files
3. Review logs for specific errors

### Memory Extraction Slow

L0/L1 generation requires LLM calls. For large sessions:
- Expect 30-60 seconds per session
- Use smaller models (gpt-4o-mini) for faster processing
- Ensure LLM API is responsive

---

## 9. CLI Commands Reference

```bash
# List sessions
cortex-mem-cli --config config.toml --tenant tenant_claw session list

# Generate L0/L1 layers
cortex-mem-cli --config config.toml --tenant tenant_claw layers ensure-all

# Rebuild vector index
cortex-mem-cli --config config.toml --tenant tenant_claw vector reindex

# Show statistics
cortex-mem-cli --config config.toml --tenant tenant_claw stats
```

---

## Quick Reference

| Task | Command/Action |
|------|----------------|
| Configure | Edit `config.toml`, set `api_key` fields |
| Start services | Restart OpenClaw (auto-start) |
| Check services | `curl localhost:6333` and `curl localhost:8085/health` |
| Migrate memory | Call `cortex_migrate` tool |
| Rebuild index | `cortex-mem-cli vector reindex` |
| Generate layers | `cortex-mem-cli layers ensure-all` |
