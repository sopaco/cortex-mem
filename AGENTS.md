# Cortex Memory - AI Agent Context Guide

> This file provides essential context for AI coding agents working on the Cortex Memory project.

## Quick Start for Agents

1. **Read `.ai-context/project-overview.md`** to understand what this project is about
2. **Read `.ai-context/architecture.md`** to understand the system design
3. **Read `.ai-context/modules.md`** to understand each crate's responsibility
4. **Read `.ai-context/uri-structure.md`** to understand the `cortex://` URI scheme
5. **Read `.ai-context/api-reference.md`** for REST API endpoints

## Project Summary

Cortex Memory is a **Rust-based AI-native memory framework** that provides:
- Three-tier memory hierarchy (L0 Abstract → L1 Overview → L2 Detail)
- Virtual filesystem with `cortex://` URI scheme
- Vector-based semantic search via Qdrant
- Multi-tenant support

## Key Technical Decisions

| Aspect | Decision |
|--------|----------|
| Language | Rust 1.86+ (Edition 2024) |
| Async Runtime | Tokio |
| Web Framework | Axum |
| Vector DB | Qdrant |
| API Version | `/api/v2/*` |
| Default Port | 8085 |

## Common Tasks

### Adding a New API Endpoint

1. Add route in `cortex-mem-service/src/routes/mod.rs`
2. Add handler in `cortex-mem-service/src/handlers/`
3. Add models in `cortex-mem-service/src/models.rs`

### Adding a New Tool (MCP)

1. Define tool schema in `cortex-mem-tools/src/tools/`
2. Add operation in `cortex-mem-tools/src/operations.rs`
3. Register in `cortex-mem-mcp/src/service.rs`

### Modifying URI Structure

1. Update `cortex-mem-core/src/filesystem/uri.rs`
2. Update `cortex-mem-core/src/types.rs` (Dimension enum)
3. Update `.ai-context/uri-structure.md`

## File Locations Quick Reference

| What | Where |
|------|-------|
| Core business logic | `cortex-mem-core/src/` |
| REST API handlers | `cortex-mem-service/src/handlers/` |
| REST API routes | `cortex-mem-service/src/routes/` |
| MCP tools | `cortex-mem-tools/src/tools/` |
| CLI commands | `cortex-mem-cli/src/commands/` |
| Configuration | `cortex-mem-config/src/lib.rs` |
| URI parsing | `cortex-mem-core/src/filesystem/uri.rs` |
| Vector search | `cortex-mem-core/src/search/` |
| Session management | `cortex-mem-core/src/session/` |
| Layer generation | `cortex-mem-core/src/layers/` |
| LLM client | `cortex-mem-core/src/llm/` |
| Embedding | `cortex-mem-core/src/embedding/` |
| MemClaw plugin | `examples/@memclaw/plugin/` |

## Build & Test Commands

```bash
# Build all crates
cargo build --workspace

# Build release
cargo build --workspace --release

# Run tests
cargo test --workspace

# Run the service
cargo run --bin cortex-mem-service -- --config config.toml

# Run CLI
cargo run --bin cortex-mem -- --help

# Run MCP server
cargo run --bin cortex-mem-mcp -- --config config.toml
```

## Configuration

Configuration is via `config.toml`. See `.ai-context/configuration.md` for details.

Key environment variables:
- `OPENAI_API_KEY` or `LLM_API_KEY` - LLM API key
- `EMBEDDING_API_KEY` - Embedding API key

## Data Directory Structure

```
cortex-data/
├── tenants/
│   └── {tenant_id}/
│       ├── session/{session_id}/timeline/{YYYY-MM}/{DD}/{HH_MM_SS}_{id}.md
│       ├── user/{user_id}/preferences/{name}.md
│       ├── user/{user_id}/entities/{name}.md
│       ├── user/{user_id}/events/{name}.md
│       ├── agent/{agent_id}/cases/{name}.md
│       └── resources/
```

**Note**: `user_id` is required in the path. Default value is `"default"` in most scenarios.

## Important Patterns

### Three-Tier Layer Access

```rust
// L0: Abstract (~100 tokens) - Quick relevance check
GET /api/v2/filesystem/abstract?uri=cortex://session/{id}/timeline

// L1: Overview (~2000 tokens) - Moderate detail
GET /api/v2/filesystem/overview?uri=cortex://session/{id}/timeline

// L2: Full content - Complete original
GET /api/v2/filesystem/content?uri=cortex://session/{id}/timeline/{file}.md
```

### Session Lifecycle

1. `POST /api/v2/sessions` - Create session
2. `POST /api/v2/sessions/{id}/messages` - Add messages
3. `POST /api/v2/sessions/{id}/close` - Close & trigger extraction

### Search with Layered Retrieval

```json
POST /api/v2/search
{
  "query": "user preferences",
  "return_layers": ["L0", "L1"],
  "limit": 10,
  "min_score": 0.6
}
```

## Context Files

| File | Purpose |
|------|---------|
| `project-overview.md` | Project goals, features, and ecosystem |
| `architecture.md` | System architecture and data flow |
| `modules.md` | Each crate's responsibility and key types |
| `uri-structure.md` | Complete URI scheme reference |
| `api-reference.md` | All REST API endpoints |
| `configuration.md` | Configuration file format |
| `development-guide.md` | Development workflow and conventions |
