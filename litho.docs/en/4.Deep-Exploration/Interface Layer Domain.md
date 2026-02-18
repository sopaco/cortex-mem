**Technical Documentation: Interface Layer Domain**
**System:** Cortex-Mem  
**Version:** 1.0  
**Last Updated:** 2026-02-17 16:46:44 (UTC)

---

## 1. Executive Summary

The **Interface Layer Domain** serves as the unified access gateway for the Cortex-Mem memory management system, implementing a **Multi-Protocol Adapter Architecture** that exposes core memory capabilities through three distinct interaction paradigms. This domain translates external requests into internal domain operations while maintaining strict separation between transport concerns and business logic.

The domain implements three primary interface implementations:
1. **Command-Line Interface (CLI)** – For scripting, automation, and developer workflows
2. **REST API Service** – For service integration and third-party application connectivity
3. **Model Context Protocol (MCP) Server** – For AI assistant and agent integration

By abstracting the complexity of core memory operations behind protocol-specific adapters, the Interface Layer enables consistent access to semantic storage, vector search, session management, and automated extraction capabilities across diverse user personas and integration scenarios.

---

## 2. Architectural Positioning

### 2.1 Hexagonal Architecture Context

Within the Cortex-Mem architecture, the Interface Layer Domain occupies the **Interface Adapters** layer (following Hexagonal/Clean Architecture principles):

```
┌─────────────────────────────────────────────────────────────┐
│                    Interface Layer Domain                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │     CLI      │  │  REST API    │  │ MCP Server   │      │
│  │  (cortex-    │  │  (cortex-    │  │  (cortex-    │      │
│  │   mem-cli)   │  │ mem-service) │  │  mem-mcp)    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Core Domain (Memory Operations)             │
└─────────────────────────────────────────────────────────────┘
```

**Dependency Rule Compliance:** All interface implementations depend inward toward the core domain (`cortex-mem-core` and `cortex-mem-tools`) and configuration domain (`cortex-mem-config`), never the reverse. This ensures domain logic remains isolated from transport and protocol concerns.

### 2.2 Component Topology

The domain follows a layered internal architecture:

**Transport & Protocol Layer**
- **Clap Parser** (CLI): Derive-based argument parsing and validation
- **Axum Router** (REST): HTTP routing, middleware, and request dispatch
- **RMCP Framework** (MCP): Protocol handling and tool routing

**Application Layer**
- **Command Dispatch**: CLI subcommand routing (add/search/list/get/delete/session/stats)
- **Request Handlers**: REST endpoint handlers organized by functional area
- **Tool Router**: MCP tool registration and execution routing

**Core Services Integration**
- **Shared State**: `Arc<MemoryOperations>` providing thread-safe access to core capabilities
- **Configuration**: TOML-based settings with environment variable overrides
- **Optional Components**: Graceful degradation when LLM, Qdrant, or embedding services are unavailable

---

## 3. Component Specifications

### 3.1 CLI Application (cortex-mem-cli)

**Technology Stack:** Rust, `clap` (derive macros), `colored` (terminal styling)

The CLI provides interactive and scriptable access to memory operations through a command-subcommand pattern with global options and colored output indicators.

**Command Structure:**
```bash
cortex-mem [GLOBAL_OPTIONS] <SUBCOMMAND> [SUBCOMMAND_OPTIONS]

Global Options:
  -c, --config <PATH>    Path to configuration file
  -t, --tenant <ID>      Tenant identifier for multi-tenancy
  -v, --verbose          Enable verbose output
```

**Subcommands:**

| Command | Description | Key Options |
|---------|-------------|-------------|
| `add` | Store new memory entry | `-t, --thread <ID>`, `-r, --role <ROLE>`, content positional arg |
| `search` | Semantic vector search | `-l, --limit <N>`, `-s, --min-score <FLOAT>`, query positional arg |
| `list` | Enumerate memory entries | `-u, --uri <URI>`, `--include-abstracts` |
| `get` | Retrieve specific memory | URI positional arg |
| `delete` | Remove memory entry | URI positional arg |
| `session` | Session lifecycle management | `create`, `close`, `list` sub-subcommands |
| `stats` | Display system statistics | `--format <json\|table>` |

**Implementation Pattern:**
- Modular command structure under `src/commands/` with individual handlers
- Initialization sequence: Argument parsing → Configuration loading → LLM client setup → `MemoryOperations` initialization
- Error handling via `anyhow` for ergonomic error propagation
- Colored output with emoji indicators for status visualization

**Example Usage:**
```bash
# Store a memory with session context
cortex-mem add -t thread-abc123 "User prefers dark mode interfaces" -r user

# Semantic search with relevance threshold
cortex-mem search "dark mode preferences" --limit 5 --min-score 0.75

# List all memories in user dimension
cortex-mem list -u cortex://user/default/memories
```

### 3.2 REST API Service (cortex-mem-service)

**Technology Stack:** Rust, `axum` web framework, `tower-http` (middleware), `serde` (serialization)

The REST API provides HTTP-based access to memory operations, implementing a layered architecture with clear separation between routing, handling, and state management.

**Architecture Layers:**
```
src/
├── main.rs          # Server initialization and startup
├── state.rs         # AppState definition and shared resources
├── error.rs         # AppError enum and IntoResponse implementations
├── models.rs        # Request/response DTOs
├── routes/          # Route definitions and path mounting
│   ├── mod.rs       # Router composition
│   ├── filesystem.rs # File operations endpoints
│   ├── sessions.rs  # Session management endpoints
│   ├── search.rs    # Semantic search endpoints
│   └── automation.rs # Background processing endpoints
└── handlers/        # Business logic implementation
    ├── mod.rs
    ├── filesystem.rs
    ├── sessions.rs
    ├── search.rs
    ├── automation.rs
    └── health.rs    # Health check endpoints
```

**Endpoint Specification:**

| Method | Path | Description | Request Body | Response |
|--------|------|-------------|--------------|----------|
| GET | `/health` | Service health check | - | `{"status": "healthy"}` |
| GET | `/api/v2/filesystem/list` | List directory contents | Query: `?uri=cortex://...` | Directory listing JSON |
| GET | `/api/v2/filesystem/read/{path}` | Read file content | - | File content or error |
| POST | `/api/v2/sessions` | Create new session | `{"thread_id": "..."}` | Session metadata |
| GET | `/api/v2/sessions` | List all sessions | Query filters | Session array |
| POST | `/api/v2/sessions/{thread_id}/messages` | Append message | `{"role": "...", "content": "..."}` | Message confirmation |
| POST | `/api/v2/sessions/{thread_id}/close` | Close session | Optional metadata | Close confirmation |
| POST | `/api/v2/search` | Semantic vector search | `{"query": "...", "limit": 5, "min_score": 0.7}` | Search results array |
| POST | `/api/v2/automation/extract/{thread_id}` | Trigger memory extraction | - | Extraction statistics |
| POST | `/api/v2/automation/index/{thread_id}` | Index session to vector DB | - | Indexing statistics |
| POST | `/api/v2/automation/index-all` | Index all threads | - | Batch statistics |

**Middleware Stack:**
- **CORS**: Cross-origin request support for web dashboard integration
- **Tracing**: Request logging and observability via `tower-http`
- **Compression**: Response compression for large memory payloads

**State Management:**
```rust
pub struct AppState {
    pub memory_ops: Arc<MemoryOperations>,
    pub config: Config,
    // Optional components with graceful degradation
    pub llm_client: Option<Arc<dyn LlmClient>>,
    pub vector_store: Option<Arc<dyn VectorStore>>,
}
```

**Error Handling:**
Custom `AppError` enum implementing `axum::response::IntoResponse`, mapping domain errors to appropriate HTTP status codes (400 Bad Request, 404 Not Found, 500 Internal Server Error) with structured JSON error bodies:
```json
{
  "error": "Memory not found for URI: cortex://user/default/memories/abc123",
  "status": 404
}
```

### 3.3 MCP Server (cortex-mem-mcp)

**Technology Stack:** Rust, `rmcp` framework, procedural macros

The Model Context Protocol (MCP) server enables AI assistants to integrate with Cortex-Mem as a tool provider, exposing memory operations through the standardized MCP specification over stdio transport.

**Tool Registry:**
The server exposes six primary tools via procedural macros (`#[tool]`, `#[tool_router]`):

| Tool Name | Description | Parameters |
|-----------|-------------|------------|
| `store_memory` | Persist message to memory | `content` (string), `thread_id` (string), `role` (string) |
| `query_memory` | Semantic search | `query` (string), `scope` (string), `limit` (int) |
| `list_memories` | Enumerate entries | `uri` (string), `limit` (int), `include_abstracts` (bool) |
| `get_memory` | Retrieve specific entry | `uri` (string) |
| `delete_memory` | Remove entry | `uri` (string) |
| `get_abstract` | Fetch L0 summary | `uri` (string) |

**Protocol Implementation:**
- **Transport**: stdio (standard input/output) with JSON-RPC 2.0 message format
- **Discovery**: Dynamic tool schema exposure via MCP capability negotiation
- **Execution**: Synchronous tool execution with structured result returns
- **Error Handling**: `anyhow`-based error propagation mapped to MCP error codes

**Integration Pattern:**
AI assistants (e.g., Claude Desktop, IDE agents) communicate via JSON-RPC:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "store_memory",
    "arguments": {
      "content": "User prefers concise technical documentation",
      "thread_id": "session-xyz",
      "role": "user"
    }
  },
  "id": 1
}
```

---

## 4. Core Workflows

### 4.1 Memory Storage Request Flow

**Trigger:** User submits memory via any interface (CLI, API, or MCP)

**Sequence:**
1. **Interface Reception**
   - CLI: `clap` parses arguments and invokes command handler
   - REST: Axum router dispatches to handler based on path/method
   - MCP: RMCP framework routes to tool implementation

2. **Request Validation**
   - Schema validation (JSON Schema for REST/MCP, type checking for CLI)
   - Authorization checks (tenant isolation via URI scope)
   - Content sanitization

3. **Core Domain Delegation**
   - Interface constructs domain objects (Memory, Session, etc.)
   - Invocation of `MemoryOperations` methods through `Arc` shared state
   - Async await for embedding generation (if configured)

4. **Persistence Coordination**
   - Parallel execution paths:
     - Vector Store: Embedding → Qdrant vector database
     - Filesystem: Content → Markdown files via `cortex://` URI scheme

5. **Response Formatting**
   - CLI: Terminal-formatted output with color coding
   - REST: JSON response with URI and ID
   - MCP: Tool result structure with success confirmation

### 4.2 Semantic Search Flow

**Trigger:** Search query submitted via `cortex-mem search` or `POST /api/v2/search`

**Processing Pipeline:**
1. **Query Reception** → Parse filters (scope, type, date ranges, importance thresholds)
2. **Vectorization** → Embedding Client converts query text to vector representation
3. **Similarity Search** → Vector Search Engine queries Qdrant with HNSW algorithm
4. **Metadata Filtering** → Database-level filtering by dimension, category, tenant
5. **Content Retrieval** → Full memory content fetched from filesystem for top-K results
6. **Scoring & Ranking** → Relevance scores calculated and threshold filtering applied
7. **Response Assembly** → Ranked results with snippets and metadata returned to caller

### 4.3 Session Lifecycle Flow

**Trigger:** Session creation or message append via `cortex-mem session` or `/api/v2/sessions`

**State Transitions:**
```
Created → Active → [Indexing] → [Extraction] → Closed
```

1. **Creation**: `SessionManager` initializes thread context with metadata
2. **Message Accumulation**: Timeline appends messages with role/timestamp; persists to markdown
3. **Auto-Indexing** (Async): AutoIndexer queues messages for vector database insertion
4. **Auto-Extraction** (Conditional): On session close, LLM analyzes timeline to extract structured memories (facts, preferences, entities)
5. **Termination**: Metadata finalized, optional memory extraction triggered

---

## 5. Integration Patterns

### 5.1 Shared State Architecture

All three interfaces utilize a **Shared State Pattern** for thread-safe access to core services:

```rust
// Thread-safe sharing across async handlers
pub type SharedState = Arc<AppState>;

// In Axum handlers
pub async fn search_handler(
    State(state): State<SharedState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<SearchResults>, AppError> {
    state.memory_ops.vector_search(payload.query).await
}
```

**Benefits:**
- Consistent connection pooling for Qdrant and LLM clients
- Configuration singleton pattern across all interfaces
- Memory-efficient sharing of loaded models and caches

### 5.2 Configuration Convergence

All interfaces follow identical initialization sequences:

1. **CLI/Environment Parsing**: Command-line args or environment variables
2. **TOML Loading**: Configuration from `config.toml` (path configurable via `--config`)
3. **Client Initialization**: Optional LLM, embedding, and vector store clients
4. **MemoryOperations Setup**: Core facade initialized with all dependencies
5. **Service Startup**: Bind to transport (stdio for MCP, TCP for REST, process exit for CLI)

**Configuration Sections:**
- `[llm]`: Provider URL, API keys, model selection
- `[qdrant]`: Vector database connection strings
- `[embedding]`: Embedding service configuration, dimension settings (default 1536)
- `[cortex]`: Data directory paths, tenant defaults

### 5.3 Error Handling Strategy

| Interface | Error Type | User Presentation |
|-----------|-----------|-------------------|
| CLI | `anyhow::Error` | Colored stderr output with context chain, non-zero exit codes |
| REST | `AppError` (custom enum) | HTTP status codes + JSON `{"error": "..."}` body |
| MCP | `rmcp::Error` | JSON-RPC error objects with codes |

**Common Error Categories:**
- **400 Bad Request**: Invalid URI format, missing required fields
- **404 Not Found**: Memory URI does not exist, session not found
- **503 Service Unavailable**: LLM/Qdrant connection failures (with graceful degradation)
- **500 Internal Error**: Unexpected domain failures

---

## 6. Deployment Models

### 6.1 Standalone Developer Mode
```
[cortex-mem-cli] ↔ [Local Filesystem + Qdrant]
```
- Direct filesystem access
- Local Qdrant instance or embedded mode
- No network exposure

### 6.2 Client-Server Architecture
```
[cortex-mem-cli] ─┐
                  ├─→ [cortex-mem-service] ↔ [Qdrant Cluster + Shared FS]
[Web Dashboard] ──┘         │
                            ↓
                      [cortex-mem-insights]
```
- Centralized service with REST API
- Multi-tenant isolation via tenant ID headers
- Web dashboard consumes REST API

### 6.3 AI Agent Integration Mode
```
[Claude Desktop / IDE Agent] ↔ [cortex-mem-mcp] ↔ [cortex-mem-core] ↔ [Storage]
```
- MCP server runs as subprocess of AI assistant
- stdio transport for local integration
- Sandboxed filesystem access

---

## 7. Security Considerations

### 7.1 Transport Security
- **REST API**: TLS termination recommended for production (via reverse proxy)
- **MCP**: stdio transport inherently local; no network exposure
- **CLI**: Local process only

### 7.2 Data Isolation
- **URI-based Scoping**: All requests validated against tenant scope (`cortex://{dimension}/{tenant}/...`)
- **Path Traversal Protection**: URI parser validates path components to prevent directory escape
- **Multi-tenancy**: Tenant ID required in all operations; cross-tenant access blocked at domain layer

### 7.3 Input Validation
- Strict schema validation for REST/MCP JSON payloads
- Content length limits to prevent memory exhaustion
- URI scheme whitelist (only `cortex://` allowed)

---

## 8. Performance Characteristics

### 8.1 Concurrency
- **Async/Await**: Tokio runtime handles concurrent requests efficiently
- **Embedding Batching**: CLI and REST support batch operations for bulk indexing
- **Connection Pooling**: Persistent connections to Qdrant and LLM services

### 8.2 Caching Layers
- **Layer Cache**: L0/L1 generated summaries cached in filesystem to avoid LLM re-generation costs
- **Vector Cache**: Qdrant HNSW indices provide sub-100ms similarity search
- **Configuration**: Loaded once at startup; hot-reload not supported (restart required)

### 8.3 Resource Limits
- **Request Timeouts**: REST API configurable timeouts for long-running searches
- **Payload Limits**: Axum default body limits (can be configured)
- **Backpressure**: Async channel-based processing prevents memory overflow during bulk operations

---

## 9. Extension Guidelines

### 9.1 Adding New CLI Commands
1. Create module in `src/commands/{command}.rs`
2. Implement `clap::Parser` derive struct for arguments
3. Add variant to main CLI enum with `#[command(subcommand)]`
4. Dispatch to `MemoryOperations` methods
5. Implement colored output formatting

### 9.2 Adding REST Endpoints
1. Define request/response models in `models.rs`
2. Implement handler function in appropriate `handlers/` module
3. Add route in corresponding `routes/` module with method and path
4. Update `AppState` if new dependencies required
5. Add CORS configuration if endpoint accessed from browser

### 9.3 Adding MCP Tools
1. Add method to service struct with `#[tool]` attribute
2. Define parameters using serializable structs
3. Implement business logic delegating to `MemoryOperations`
4. Update tool router if new tool categories introduced
5. Document tool schema for AI assistant consumption

---

## 10. Troubleshooting

### 10.1 Common Integration Issues

**CLI: "Configuration file not found"**
- Ensure `config.toml` exists in default location (`~/.config/cortex-mem/`) or specify via `-c`

**REST: "Connection refused" to Qdrant**
- Verify Qdrant service running and URL correct in config
- Check firewall rules for port 6333 (default Qdrant HTTP)

**MCP: "Tool not found" by AI assistant**
- Verify MCP server started correctly (stdio transport active)
- Check tool names match exactly (case-sensitive)
- Ensure JSON-RPC message format compliance

### 10.2 Debug Mode
All interfaces support verbose logging via:
- CLI: `-v, --verbose` flag
- REST: `RUST_LOG=debug` environment variable
- MCP: stderr logging (visible in AI assistant logs)

---

## Appendix A: Protocol Specifications

### A.1 URI Scheme Reference
All interfaces utilize the `cortex://` URI scheme for resource addressing:
```
cortex://{dimension}/{tenant}/{category}/{resource_id}

Examples:
cortex://user/acme-corp/memories/uuid-123
cortex://session/thread-abc/messages/msg-456
cortex://agent/bot-1/profiles/default
```

### A.2 JSON Schema Examples

**Search Request (REST):**
```json
{
  "query": "machine learning preferences",
  "limit": 10,
  "min_score": 0.75,
  "filters": {
    "dimension": "user",
    "categories": ["preferences", "facts"],
    "date_after": "2024-01-01"
  }
}
```

**MCP Tool Result:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "Memory stored successfully at cortex://user/default/memories/uuid-789"
    }
  ],
  "isError": false
}
```

---

**Document Control**  
**Author:** Technical Architecture Team  
**Reviewers:** System Engineering, DevOps  
**Classification:** Technical Implementation Guide  
**Related Documents:** Core Domain Architecture, Vector Search Domain Spec, Deployment Guide