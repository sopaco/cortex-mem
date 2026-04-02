# Development Guide

## Prerequisites

- **Rust**: 1.86 or later (Edition 2024)
- **Qdrant**: 1.7+ (for vector search)
- **LLM API**: OpenAI-compatible endpoint
- **Embedding API**: OpenAI-compatible endpoint

## Getting Started

### 1. Clone and Build

```bash
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem
cargo build --workspace
```

### 2. Configure

Create `config.toml` in the project root:

```toml
[qdrant]
url = "http://localhost:6334"
collection_name = "cortex-memory"
embedding_dim = 1536

[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model_efficient = "gpt-5-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model_name = "text-embedding-3-small"

[server]
host = "localhost"
port = 8085
```

### 3. Run Services

```bash
# Terminal 1: Start Qdrant (if not already running)
qdrant

# Terminal 2: Start the service
cargo run --bin cortex-mem-service -- --config config.toml

# Terminal 3: Use the CLI
cargo run --bin cortex-mem -- --config config.toml --help
```

---

## Project Structure

```
cortex-mem/
├── cortex-mem-core/           # Core library
│   ├── src/
│   │   ├── lib.rs             # Main entry point
│   │   ├── builder.rs         # Builder pattern
│   │   ├── types.rs           # Core types
│   │   ├── filesystem/        # Virtual filesystem
│   │   ├── session/           # Session management
│   │   ├── search/            # Vector search
│   │   ├── layers/            # L0/L1 generation
│   │   ├── llm/               # LLM client
│   │   ├── embedding/         # Embedding client
│   │   ├── vector_store/      # Qdrant integration
│   │   └── automation/        # Background tasks
│   └── Cargo.toml
│
├── cortex-mem-service/        # REST API server
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   ├── state.rs           # App state
│   │   ├── models.rs          # API models
│   │   ├── routes/            # Route definitions
│   │   └── handlers/          # Request handlers
│   └── Cargo.toml
│
├── cortex-mem-cli/            # CLI tool
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   └── commands/          # Command implementations
│   └── Cargo.toml
│
├── cortex-mem-mcp/            # MCP server
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   └── service.rs         # Tool registration
│   └── Cargo.toml
│
├── cortex-mem-tools/          # MCP tools
│   ├── src/
│   │   ├── lib.rs             # Exports
│   │   ├── operations.rs      # Operations
│   │   ├── tools/             # Tool definitions
│   │   └── mcp/               # MCP types
│   └── Cargo.toml
│
├── cortex-mem-rig/            # Rig integration
│   ├── src/
│   │   ├── lib.rs             # Tool registration
│   │   └── tools/             # Rig tools
│   └── Cargo.toml
│
├── cortex-mem-config/         # Configuration
│   ├── src/
│   │   └── lib.rs             # Config parsing
│   └── Cargo.toml
│
└── cortex-mem-insights/       # Web dashboard
    ├── src/
    │   ├── App.svelte         # Main component
    │   ├── main.ts            # Entry point
    │   └── lib/               # Utilities
    └── package.json
```

---

## Coding Conventions

### Rust Style

1. **Async by default**: Use `async fn` for I/O operations
2. **Result handling**: Use `anyhow::Result` for application code, `thiserror` for library errors
3. **Arc for sharing**: Use `Arc<T>` for shared state
4. **RwLock for state**: Use `RwLock<T>` for mutable shared state

```rust
// Good
pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
    let engine = self.vector_engine.read().await;
    // ...
}

// Avoid blocking in async
pub async fn bad_example(&self) -> Result<()> {
    std::fs::read_to_string("file.txt")?; // Blocks!
    // Use tokio::fs instead
}
```

### Error Handling

```rust
// Define errors in error.rs
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    
    #[error("File not found: {0}")]
    NotFound(String),
}

// Use anyhow for application code
pub fn do_something() -> anyhow::Result<()> {
    // ...
}
```

### Trait Design

```rust
// Define traits for abstraction
#[async_trait]
pub trait FilesystemOperations: Send + Sync {
    async fn list(&self, uri: &str) -> Result<Vec<FileEntry>>;
    async fn read(&self, uri: &str) -> Result<String>;
    async fn write(&self, uri: &str, content: &str) -> Result<()>;
}
```

---

## Testing

### Unit Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p cortex-mem-core

# Run with output
cargo test --workspace -- --nocapture
```

### Integration Tests

Integration tests are in `tests/` directories:

```
cortex-mem-cli/
└── tests/
    └── cli_commands_test.rs
```

### Test Configuration

Create a test `config.toml` with test credentials.

---

## Adding New Features

### Adding a New API Endpoint

1. **Define route** in `cortex-mem-service/src/routes/mod.rs`:

```rust
pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // ... existing routes
        .route("/my-endpoint", get(handlers::my_module::my_handler))
}
```

2. **Create handler** in `cortex-mem-service/src/handlers/my_module.rs`:

```rust
pub async fn my_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MyParams>,
) -> Result<Json<ApiResponse<MyResponse>>> {
    // Implementation
    Ok(Json(ApiResponse::success(data)))
}
```

3. **Define models** in `cortex-mem-service/src/models.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct MyParams {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct MyResponse {
    pub result: String,
}
```

### Adding a New CLI Command

1. **Add command** in `cortex-mem-cli/src/commands/my_command.rs`:

```rust
pub async fn execute(config: &Config, args: MyArgs) -> Result<()> {
    // Implementation
}
```

2. **Register in main.rs**:

```rust
Commands::MyCommand(args) => {
    commands::my_command::execute(&config, args).await?;
}
```

### Adding a New MCP Tool

1. **Define tool schema** in `cortex-mem-tools/src/tools/my_tool.rs`:

```rust
pub const MY_TOOL: ToolSchema = ToolSchema {
    name: "cortex_my_tool",
    description: "Tool description",
    input_schema: json!({
        "type": "object",
        "properties": {
            "param": { "type": "string" }
        },
        "required": ["param"]
    }),
};
```

2. **Add operation** in `cortex-mem-tools/src/operations.rs`:

```rust
pub async fn my_operation(core: &CortexMem, param: &str) -> Result<MyResult> {
    // Implementation
}
```

3. **Register in MCP** in `cortex-mem-mcp/src/service.rs`:

```rust
server.tool(
    tools::MY_TOOL.name,
    tools::MY_TOOL.schema(),
    my_handler,
)?;
```

---

## Debugging

### Enable Debug Logging

```toml
# config.toml
[logging]
level = "debug"
```

Or via CLI:
```bash
RUST_LOG=debug cargo run --bin cortex-mem-service
```

### Common Debug Points

1. **URI Parsing**: `cortex-mem-core/src/filesystem/uri.rs`
2. **Search Flow**: `cortex-mem-core/src/search/engine.rs`
3. **Layer Generation**: `cortex-mem-core/src/layers/generator.rs`
4. **API Requests**: `cortex-mem-service/src/handlers/`

### Inspecting Data

```bash
# List sessions
curl http://localhost:8085/api/v2/sessions

# List files
curl "http://localhost:8085/api/v2/filesystem/list?uri=cortex://session&recursive=true"

# Check Qdrant
curl http://localhost:6333/collections
```

---

## Release Process

1. Update version in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Run tests: `cargo test --workspace`
4. Build release: `cargo build --workspace --release`
5. Create git tag: `git tag v2.x.x`
6. Push: `git push --tags`

---

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Run `cargo fmt` and `cargo clippy`
5. Submit pull request

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Check documentation
cargo doc --workspace --no-deps
```
