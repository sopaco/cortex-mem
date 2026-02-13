# Configuration Domain Documentation

## Overview

The **Configuration Domain** serves as the foundational infrastructure layer of the Cortex-Mem system, providing a centralized, type-safe, and extensible mechanism for managing all system-wide settings. It acts as the single source of truth for runtime configuration parameters across all components — from core memory services to external-facing interfaces — ensuring consistency, flexibility, and operational reliability.

Built using Rust’s ecosystem of serialization and environment variable handling libraries, the Configuration Domain enables declarative configuration through TOML files, with dynamic fallbacks derived from environment variables and system conventions. This design eliminates stringly-typed configuration, reduces runtime errors, and supports seamless deployment across development, staging, and production environments.

The domain is implemented in the `cortex-mem-config` crate and is consumed by every other domain in the system, including the Core Memory Domain, Tool Support Domain, and example applications. Its architecture follows the principle of *configuration as code*: settings are modeled as strongly-typed Rust structs, loaded once at startup, and immutably shared throughout the application lifecycle.

---

## Architecture and Design Principles

### Core Design Philosophy

The Configuration Domain adheres to the following architectural principles:

| Principle | Description |
|---------|-------------|
| **Type Safety** | All configuration values are represented as strongly-typed Rust structs with compile-time validation, preventing runtime misconfigurations (e.g., invalid URLs, non-numeric ports). |
| **Declarative Loading** | Configuration is loaded once via `Config::load()` from a TOML file, parsed into a complete hierarchical structure, and then cloned as needed. |
| **Fallback Hierarchy** | Configuration values are resolved in a prioritized order: Environment Variables > TOML File > Default Values, enabling environment-specific overrides without code changes. |
| **Immutable State** | Once loaded, the configuration object is immutable. Clone-derived cheap copies are used for passing configuration to subsystems, ensuring thread safety and consistency. |
| **Separation of Concerns** | Each subsystem (Qdrant, LLM, Logging, etc.) defines its own configuration struct, aggregated under a root `Config` struct. This avoids monolithic configuration and promotes modularity. |
| **Platform-Aware Defaults** | Paths and directories are resolved using system-aware heuristics (via the `directories` crate), ensuring cross-platform compatibility (Windows, macOS, Linux). |

### Component Hierarchy

The configuration structure is organized as a nested hierarchy, with the root `Config` struct aggregating all subsystem-specific configurations:

```rust
pub struct Config {
    pub cortex: CortexConfig,
    pub qdrant: QdrantConfig,
    pub llm: LLMConfig,
    pub embedding: EmbeddingConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}
```

Each nested struct (e.g., `QdrantConfig`, `LLMConfig`) encapsulates its own domain-specific parameters, with defaults derived from environment variables or system conventions.

---

## Implementation Details

### 1. Configuration File Format (`config.toml`)

The primary configuration source is a TOML file, typically located at `./config.toml` or specified via the `CORTEX_CONFIG_PATH` environment variable. The file structure is hierarchical and self-documenting:

```toml
# Root Configuration
[cortex]
data_dir = ""  # Optional: overrides system defaults

[qdrant]
url = "http://localhost:6333"
collection_name = "cortex_mem"
embedding_dimension = 768
timeout_ms = 5000

[llm]
provider = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4-turbo"
temperature = 0.3
max_tokens = 2048

[embedding]
model = "text-embedding-3-small"
batch_size = 16

[server]
host = "127.0.0.1"
port = 8080
cors_origins = ["http://localhost:5173"]

[logging]
level = "info"
format = "json"
file_path = ""  # Optional: log to file; defaults to stdout
```

> **Note**: All fields are optional. Missing values are filled via defaults defined in the `Default` trait implementation.

### 2. Configuration Structs and Derivations

Each configuration struct is annotated with `#[derive(Serialize, Deserialize)]` and `#[serde(default)]` to enable automatic deserialization from TOML and fallback to defaults:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    pub embedding_dimension: u64,
    pub timeout_ms: u64,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string()),
            collection_name: std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "cortex_mem".to_string()),
            embedding_dimension: std::env::var("EMBEDDING_DIMENSION")
                .map(|s| s.parse().unwrap_or(768))
                .unwrap_or(768),
            timeout_ms: std::env::var("QDRANT_TIMEOUT_MS")
                .map(|s| s.parse().unwrap_or(5000))
                .unwrap_or(5000),
        }
    }
}
```

> **Key Insight**: Environment variable defaults are implemented directly in `Default::default()` using `std::env::var()` with fallback strings. This ensures that configuration values are resolved at **load time**, not at struct instantiation, enabling dynamic overrides without recompilation.

### 3. Data Directory Resolution Logic

The `CortexConfig` struct implements a sophisticated, platform-aware resolution for the system’s data directory — a critical path used by the Filesystem Abstraction, Metadata Indexing, and Automation Engine.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CortexConfig {
    pub data_dir: String,
}

impl CortexConfig {
    pub fn data_dir(&self) -> PathBuf {
        // 1. Priority: Environment variable override
        if let Ok(env_path) = std::env::var("CORTEX_DATA_DIR") {
            return PathBuf::from(env_path);
        }

        // 2. Fallback: System application data directory (cross-platform)
        if let Some(dirs) = directories::ProjectDirs::from("com", "cortex", "mem") {
            return dirs.data_dir().to_path_buf();
        }

        // 3. Final fallback: Local ./cortex directory
        PathBuf::from("./cortex")
    }
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            data_dir: "".to_string(), // Will be resolved via data_dir() method at runtime
        }
    }
}
```

#### Resolution Priority Order:
1. **`CORTEX_DATA_DIR`** environment variable — for containerized or cloud deployments.
2. **System Application Data Directory** — resolved via the `directories` crate:
   - Linux: `~/.local/share/cortex/mem`
   - macOS: `~/Library/Application Support/com.cortex.mem`
   - Windows: `%LOCALAPPDATA%\cortex\mem`
3. **Fallback**: `./cortex` — for development, testing, or portable use.

This ensures that:
- **Developers** can work with local files without sudo or system permissions.
- **Production deployments** can override paths via environment variables.
- **Docker/Kubernetes** users can mount volumes to `/data/cortex` via `CORTEX_DATA_DIR`.

### 4. Root Configuration Loader: `Config::load()`

The entry point for configuration loading is the static method `Config::load(path: &str) -> Result<Config, ConfigError>`:

```rust
impl Config {
    pub fn load(config_path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| ConfigError::FileReadError(config_path.to_string(), e))?;

        let mut config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(config_path.to_string(), e))?;

        // Ensure data_dir is resolved after deserialization
        config.cortex.data_dir = config.cortex.data_dir(); // Calls the method to resolve path

        Ok(config)
    }
}
```

#### Key Behavior:
- Reads the TOML file from disk.
- Deserializes into a `Config` struct using `toml::from_str`.
- **Post-processes** `CortexConfig.data_dir()` to resolve the final path using the priority logic above.
- Returns a `Result` with detailed error types (`FileReadError`, `ParseError`, `ValidationError`).
- **Does not mutate** the config after loading — it is immutable and cloneable.

> **Note**: The `Config` struct is designed to be loaded **once per application lifecycle**. Subsystems receive a cloned copy via dependency injection (e.g., `AppState` in HTTP server).

### 5. Environment Variable Mapping

The following environment variables are recognized and used as configuration overrides:

| Variable | Purpose | Default (if unset) |
|----------|---------|-------------------|
| `CORTEX_DATA_DIR` | Override data directory path | Resolved via `directories` crate or `./cortex` |
| `QDRANT_URL` | Qdrant service endpoint | `http://localhost:6333` |
| `QDRANT_COLLECTION` | Vector collection name | `cortex_mem` |
| `QDRANT_TIMEOUT_MS` | HTTP timeout for Qdrant requests | `5000` |
| `EMBEDDING_DIMENSION` | Expected embedding vector size | `768` |
| `LLM_PROVIDER` | LLM provider (e.g., openai, anthropic) | `openai` |
| `LLM_API_BASE_URL` | Base URL for LLM API | `https://api.openai.com/v1` |
| `LLM_API_KEY` | Authentication token | (required) |
| `LLM_MODEL` | Model name for LLM | `gpt-4-turbo` |
| `LLM_TEMPERATURE` | Sampling temperature | `0.3` |
| `EMBEDDING_MODEL` | Embedding model name | `text-embedding-3-small` |
| `EMBEDDING_BATCH_SIZE` | Batch size for embedding generation | `16` |
| `SERVER_HOST` | HTTP server bind address | `127.0.0.1` |
| `SERVER_PORT` | HTTP server port | `8080` |
| `LOGGING_LEVEL` | Log level (trace, debug, info, warn, error) | `info` |
| `LOGGING_FORMAT` | Log format (text, json) | `text` |
| `LOGGING_FILE_PATH` | Optional file path for logs | (stdout) |

> **Best Practice**: Environment variables take precedence over TOML values. This allows secrets (e.g., API keys) to be injected via secrets managers (Vault, AWS Secrets Manager, Kubernetes Secrets) without exposing them in version-controlled files.

### 6. Error Handling and Validation

Configuration loading is guarded by a robust error enum:

```rust
#[derive(Debug)]
pub enum ConfigError {
    FileReadError(String, std::io::Error),
    ParseError(String, toml::de::Error),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileReadError(path, err) => write!(f, "Failed to read config file '{}': {}", path, err),
            ConfigError::ParseError(path, err) => write!(f, "Failed to parse config file '{}': {}", path, err),
            ConfigError::ValidationError(msg) => write!(f, "Configuration validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
```

- **FileReadError**: Occurs if `config.toml` does not exist or is unreadable.
- **ParseError**: Occurs if TOML syntax is invalid or types mismatch (e.g., string where int is expected).
- **ValidationError**: Used for semantic validation (e.g., invalid URL format, negative port).

> **Validation Strategy**: Validation is kept minimal at load time (e.g., no network connectivity checks). Critical validations (e.g., API key presence) are deferred to subsystem initialization, where failure can be handled gracefully with fallbacks or alerts.

---

## Interactions with Other Domains

The Configuration Domain is a **dependency sink** — it does not invoke other domains but is consumed by all of them. Below are key interaction patterns:

### 1. Core Memory Domain
- **Consumes**: `QdrantConfig`, `LLMConfig`, `CortexConfig`
- **Usage**: 
  - `QdrantVectorStore::new(config.qdrant)` initializes the vector store.
  - `LLMClientImpl::new(config.llm)` configures the LLM client with API key and model.
  - `CortexFilesystem::new(config.cortex.data_dir())` initializes the virtual filesystem with resolved path.

### 2. Tool Support Domain
- **CLI Interface**: Loads `Config` to resolve `data_dir`, `log_level`, and `qdrant_url` for command-line flags.
- **HTTP API Service**: Injects `Config` into `AppState` to configure Axum routes, CORS, and logging middleware.
- **MCP Server**: Uses `Config` to determine LLM provider and embedding model for agent memory queries.

### 3. Example Applications
- `cortex-mem-tars` uses a custom `BotConfig` struct derived from `Config`, demonstrating extensibility.
- Example configs are shipped with the project to enable quickstart workflows.

### 4. Testing and Mocking
- Unit tests use `Config::load("tests/config/test.toml")` to load isolated test configurations.
- Integration tests override `CORTEX_DATA_DIR` to use temporary directories (`tempdir` crate).
- Dependency injection allows mocking of `Config` in tests without filesystem access.

---

## Practical Usage Examples

### Example 1: CLI Command Initialization

```rust
// cortex-mem-cli/src/main.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::var("CORTEX_CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let config = Config::load(&config_path)?;

    let filesystem = CortexFilesystem::new(config.cortex.data_dir());
    let qdrant_store = QdrantVectorStore::new(config.qdrant);
    let llm_client = LLMClientImpl::new(config.llm);

    let retrieval_engine = RetrievalEngine::new(filesystem, qdrant_store, llm_client);

    // Execute CLI command
    match cli_command {
        "search" => execute_search(&retrieval_engine, &args),
        "add" => execute_add(&filesystem, &args),
        _ => unreachable!(),
    }

    Ok(())
}
```

### Example 2: HTTP API Server Setup

```rust
// cortex-mem-service/src/state.rs
pub struct AppState {
    pub config: Config,
    pub filesystem: CortexFilesystem,
    pub vector_store: QdrantVectorStore,
    pub embedding_client: EmbeddingClient,
    pub retrieval_engine: RetrievalEngine,
}

impl AppState {
    pub async fn new() -> Result<Self, ConfigError> {
        let config = Config::load("config.toml")?;
        let filesystem = CortexFilesystem::new(config.cortex.data_dir());
        let vector_store = QdrantVectorStore::new(config.qdrant).await?;
        let embedding_client = EmbeddingClient::new(config.embedding);
        let retrieval_engine = RetrievalEngine::new(filesystem.clone(), vector_store.clone(), embedding_client.clone());

        Ok(Self {
            config,
            filesystem,
            vector_store,
            embedding_client,
            retrieval_engine,
        })
    }
}
```

### Example 3: Environment-Driven Deployment

```bash
# Production deployment (Docker)
docker run -d \
  -e CORTEX_DATA_DIR=/data/cortex \
  -e QDRANT_URL=http://qdrant:6333 \
  -e LLM_API_KEY=sk-xxx \
  -e LOGGING_FORMAT=json \
  -v /host/data:/data/cortex \
  cortex-mem-service
```

> **Result**: The service uses `/data/cortex` for storage, connects to an external Qdrant instance, and outputs structured JSON logs — all without modifying code or config files.

---

## Best Practices and Recommendations

| Practice | Rationale |
|--------|-----------|
| **Always use `Config::load()`** | Never construct `Config` manually. Always load from TOML to ensure defaults and environment overrides are applied. |
| **Use environment variables for secrets** | Never commit API keys or tokens to `config.toml`. Use `.env` files or secrets managers. |
| **Validate paths at runtime** | Use `CortexConfig::data_dir()` to resolve paths — never assume they are valid. |
| **Clone, don’t mutate** | Configuration is immutable. Use `config.clone()` to pass to threads or services. |
| **Avoid deep nesting** | Keep configuration flat where possible. Avoid nested structs unless logically grouped (e.g., `server`, `logging`). |
| **Document defaults** | Every field in `config.toml` should have a comment explaining its purpose and default value. |
| **Use `CORTEX_CONFIG_PATH` for non-standard locations** | Enables configuration in `/etc/cortex/` or `/opt/cortex/` on Linux systems. |

---

## Extensibility and Future Enhancements

### Planned Enhancements

| Feature | Description |
|--------|-------------|
| **Schema Validation** | Integrate `serde-validate` or `jsonschema` to validate TOML against a JSON schema at load time. |
| **Hot Reloading** | Watch `config.toml` for changes and reload config in background (for long-running services). |
| **Config Diffing** | Log differences between defaults and loaded values for auditability. |
| **Config Export** | Add `Config::export()` to generate a complete `config.toml` from current runtime state. |
| **YAML Support** | Add optional YAML parser for users preferring YAML over TOML. |
| **Configuration Profiles** | Support `config.dev.toml`, `config.prod.toml` via `CORTEX_ENV=prod`. |

### Custom Configuration Extensions

The architecture supports custom configuration structs in external crates:

```rust
// In a plugin or example app
#[derive(Serialize, Deserialize, Default)]
pub struct BotConfig {
    pub personality: String,
    pub max_memory_age_days: u32,
}

impl From<&Config> for BotConfig {
    fn from(config: &Config) -> Self {
        Self {
            personality: config.llm.model.clone(),
            max_memory_age_days: 30,
        }
    }
}
```

This pattern enables modular, plugin-based systems where third-party tools can extend configuration without modifying core code.

---

## Conclusion

The **Configuration Domain** is the silent backbone of the Cortex-Mem system. By combining Rust’s type system, TOML’s human-readable syntax, and environment variable precedence, it delivers a configuration solution that is:

- **Safe**: Compile-time guarantees prevent misconfigurations.
- **Flexible**: Environment overrides enable deployment agility.
- **Portable**: Cross-platform path resolution ensures consistent behavior.
- **Scalable**: Modular structure supports future subsystems and plugins.

Its design exemplifies modern Rust application architecture: **configuration as data, not code**. By centralizing and formalizing configuration, the domain reduces cognitive load on developers, minimizes runtime failures, and enables reliable operation across diverse environments — from local development to cloud-native deployments.

> **Final Note**: The Configuration Domain is not merely a utility — it is a **systemic enabler** of reliability, observability, and maintainability across the entire Cortex-Mem ecosystem.