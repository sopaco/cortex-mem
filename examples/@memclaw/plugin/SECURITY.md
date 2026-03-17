# Security Information

This document describes security considerations for the MemClaw plugin.

## Data Flow

```
User Input → OpenClaw → MemClaw Plugin → cortex-mem-service (localhost:8085)
                                              │
                                              ▼
                                         Qdrant (localhost:6334)
                                              │
                                              ▼
                                         Local Storage
```

**Key Points:**
- All data processing happens **locally** on your machine
- No data is sent to external servers except your configured LLM/Embedding providers
- API keys are only transmitted to your configured API endpoints

## Credentials

### Required Credentials

| Credential | Purpose | Storage Location |
|------------|---------|------------------|
| `llmApiKey` | Memory extraction and summarization | OpenClaw plugin config (marked `sensitive: true`) |
| `embeddingApiKey` | Vector embedding generation | OpenClaw plugin config (marked `sensitive: true`) |

### Credential Security

- API keys are stored in `openclaw.json` with the `sensitive` flag
- Keys are **never** logged or transmitted except to your configured API provider
- Keys are **never** sent to the MemClaw developers or any third party

## Binary Packages

### What's Included

MemClaw uses platform-specific binary packages distributed via npm:

| Package | Platform | Contents |
|---------|----------|----------|
| `@memclaw/bin-darwin-arm64` | macOS Apple Silicon | Qdrant, cortex-mem-service, cortex-mem-cli |
| `@memclaw/bin-win-x64` | Windows x64 | Qdrant, cortex-mem-service, cortex-mem-cli |

### Verification

To verify binary packages:

```bash
# Check package integrity via npm
npm view @memclaw/bin-darwin-arm64
npm view @memclaw/bin-win-x64

# Inspect package contents
npm pack @memclaw/bin-darwin-arm64
tar -tzf memclaw-bin-darwin-arm64-*.tgz
```

### Source Code

The source code for building these binaries is available in the main repository:
- Repository: https://github.com/sopaco/cortex-mem
- Build scripts: `cortex-mem-core/`, `cortex-mem-service/`

## Network Security

### Ports Used

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| Qdrant HTTP | 6333 | TCP | REST API, health checks |
| Qdrant gRPC | 6334 | TCP | Vector operations |
| cortex-mem-service | 8085 | TCP | Memory service API |

### Firewall Configuration

If you use a firewall, ensure:
- Ports 6333, 6334, 8085 are allowed for **localhost only**
- External connections to these ports are blocked

### Localhost Only

All services bind to `localhost` (127.0.0.1) by default:
- No external network access is required
- Services are not accessible from other machines
