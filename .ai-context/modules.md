# Modules Reference

## Workspace Structure

```
cortex-mem/
├── cortex-mem-core/       # Core business logic
├── cortex-mem-service/    # REST API server
├── cortex-mem-cli/        # Command-line interface
├── cortex-mem-mcp/        # MCP server
├── cortex-mem-tools/      # MCP tools & operations
├── cortex-mem-rig/        # Rig framework integration
├── cortex-mem-config/     # Configuration management
└── cortex-mem-insights/   # Web dashboard (Svelte)
```

---

## cortex-mem-core

**Purpose**: Core business logic and storage abstraction.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Main `CortexMem` struct, builder pattern |
| `builder.rs` | `CortexMemBuilder` for dependency injection |
| `types.rs` | Core types: `Dimension`, `FileEntry`, `Memory` |
| `error.rs` | Error types |
| `config.rs` | Runtime configuration |

### Submodules

#### `filesystem/`
Virtual filesystem with `cortex://` URI scheme.

| File | Key Types |
|------|-----------|
| `uri.rs` | `CortexUri`, `UriParser` |
| `operations.rs` | `CortexFilesystem`, `FilesystemOperations` trait |

#### `session/`
Session and conversation management.

| File | Key Types |
|------|-----------|
| `manager.rs` | `SessionManager` |
| `types.rs` | `Session`, `Message` |

#### `search/`
Vector search engine with L0/L1/L2 scoring.

| File | Key Types |
|------|-----------|
| `engine.rs` | `VectorSearchEngine` |
| `options.rs` | `SearchOptions` |

#### `layers/`
L0 abstract and L1 overview generation.

| File | Key Types |
|------|-----------|
| `generator.rs` | `LayerGenerator` |
| `cascade_layer_updater.rs` | `CascadeLayerUpdater` |

#### `llm/`
LLM client abstraction.

| File | Key Types |
|------|-----------|
| `client.rs` | `LLMClient` trait, `LLMClientImpl` |
| `prompt.rs` | Prompt templates |

#### `embedding/`
Embedding generation.

| File | Key Types |
|------|-----------|
| `client.rs` | `EmbeddingClient`, `EmbeddingConfig` |

#### `vector_store/`
Qdrant integration.

| File | Key Types |
|------|-----------|
| `qdrant.rs` | `QdrantVectorStore` |
| `mod.rs` | `VectorStore` trait |

#### `automation/`
Background automation.

| File | Key Types |
|------|-----------|
| `sync_manager.rs` | `SyncManager` |
| `auto_indexer.rs` | `AutoIndexer` |

#### `init/`
Bootstrap and initialization.

| File | Key Types |
|------|-----------|
| `bootstrap.rs` | Tenant initialization |

### Key Types

```rust
// Core dimensions for memory storage
pub enum Dimension {
    Resources,
    User,
    Agent,
    Session,
}

// Main runtime
pub struct CortexMem {
    pub filesystem: Arc<dyn FilesystemOperations>,
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub vector_store: Option<Arc<dyn VectorStore>>,
    pub embedding: Option<Arc<EmbeddingClient>>,
    pub llm_client: Option<Arc<dyn LLMClient>>,
}

// Search options
pub struct SearchOptions {
    pub limit: usize,
    pub threshold: f32,
    pub root_uri: Option<String>,
    pub recursive: bool,
}
```

---

## cortex-mem-service

**Purpose**: REST API server exposing all memory operations.

### Key Files

| File | Purpose |
|------|---------|
| `main.rs` | Server entry point, route setup |
| `state.rs` | `AppState` with tenant management |
| `models.rs` | API request/response models |
| `error.rs` | API error types |

### Routes

| File | Endpoints |
|------|-----------|
| `routes/filesystem.rs` | `/api/v2/filesystem/*` |
| `routes/sessions.rs` | `/api/v2/sessions/*` |
| `routes/search.rs` | `/api/v2/search` |
| `routes/tenants.rs` | `/api/v2/tenants/*` |
| `routes/automation.rs` | `/api/v2/automation/*` |

### Handlers

| File | Handlers |
|------|----------|
| `handlers/filesystem.rs` | `list_directory`, `read_file`, `get_abstract`, etc. |
| `handlers/sessions.rs` | `list_sessions`, `create_session`, `add_message`, etc. |
| `handlers/search.rs` | `semantic_search` |
| `handlers/tenants.rs` | `list_tenants`, `switch_tenant` |

### Key Types

```rust
// Application state
pub struct AppState {
    pub cortex: Arc<RwLock<Arc<CortexMem>>>,
    pub session_manager: Arc<RwLock<Arc<RwLock<SessionManager>>>>,
    pub vector_engine: Arc<RwLock<Option<Arc<VectorSearchEngine>>>>,
    pub data_dir: PathBuf,
    pub current_tenant_root: Arc<RwLock<Option<PathBuf>>>,
    pub current_tenant_id: Arc<RwLock<Option<String>>>,
}

// API response wrapper
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

---

## cortex-mem-cli

**Purpose**: Command-line interface for memory operations.

### Key Files

| File | Purpose |
|------|---------|
| `main.rs` | CLI entry point, argument parsing |
| `commands/*.rs` | Individual command implementations |

### Commands

| Command | Description |
|---------|-------------|
| `add` | Add a message to a session |
| `search` | Semantic search |
| `list` | List directory contents |
| `get` | Get a specific memory |
| `delete` | Delete a memory |
| `session` | Session management (list, create, close) |
| `layers` | Layer management (status, ensure-all) |
| `tenant` | Tenant operations |
| `stats` | System statistics |
| `vector` | Vector operations (reindex, prune) |

---

## cortex-mem-mcp

**Purpose**: Model Context Protocol server for AI assistant integration.

### Key Files

| File | Purpose |
|------|---------|
| `main.rs` | MCP server entry point |
| `service.rs` | Tool registration and execution |

### Integration

Works with:
- Claude Desktop
- Cursor IDE
- Other MCP-compatible AI assistants

---

## cortex-mem-tools

**Purpose**: MCP tool schemas and operation wrappers.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Tool exports |
| `tools/*.rs` | Individual tool definitions |
| `operations.rs` | Operation implementations |
| `types.rs` | Shared types |
| `mcp/*.rs` | MCP-specific types |

### Tools Provided

| Tool | Description |
|------|-------------|
| `cortex_search` | Semantic search with layered retrieval |
| `cortex_recall` | Recall with extended context |
| `cortex_add_memory` | Store messages |
| `cortex_close_session` | Close session & trigger extraction |
| `cortex_ls` | List directory contents |
| `cortex_get_abstract` | Get L0 abstract |
| `cortex_get_overview` | Get L1 overview |
| `cortex_get_content` | Get L2 full content |

---

## cortex-mem-rig

**Purpose**: Integration with Rig agent framework.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Rig tool registration |
| `tools/*.rs` | Rig-specific tool implementations |

---

## cortex-mem-config

**Purpose**: Configuration file parsing and management.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Config struct, TOML parsing |

### Key Types

```rust
pub struct Config {
    pub qdrant: QdrantConfig,
    pub llm: LLMConfig,
    pub embedding: EmbeddingConfig,
    pub cortex: CortexConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}
```

---

## cortex-mem-insights

**Purpose**: Web dashboard for monitoring and management.

### Tech Stack

- Svelte 5
- TypeScript
- Vite
- Tailwind CSS

### Key Files

| File | Purpose |
|------|---------|
| `src/App.svelte` | Main application |
| `src/lib/` | Utility functions and components |
| `server.ts` | Development server with proxy |

### Features

- Tenant management
- Memory browser
- Semantic search UI
- Health monitoring
