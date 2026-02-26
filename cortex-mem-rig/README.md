# Cortex Memory Rig Integration

`cortex-mem-rig` provides integration with the [Rig](https://github.com/0xPlaygrounds/rig) AI framework, enabling AI agents to interact with the Cortex Memory system through tool calls.

## 🧠 Overview

Cortex Memory Rig implements OpenViking-style tiered access tools, allowing AI agents to efficiently retrieve and manipulate memories:

### Three-Tier Access Architecture

| Layer | Size | Purpose | Tool |
|-------|------|---------|------|
| **L0 Abstract** | ~100 tokens | Quick relevance judgment | `abstract_tool` |
| **L1 Overview** | ~2000 tokens | Partial context understanding | `overview_tool` |
| **L2 Full** | Complete content | Deep analysis and processing | `read_tool` |

### Tool Categories

- 📊 **Tiered Access Tools**: `abstract`, `overview`, `read`
- 🔍 **Search Tools**: `search`, `find`
- 📁 **Filesystem Tools**: `ls`, `explore`
- 💾 **Storage Tools**: `store`

## 🚀 Quick Start

### Installation

```toml
[dependencies]
cortex-mem-rig = { path = "../cortex-mem-rig" }
cortex-mem-tools = { path = "../cortex-mem-tools" }
cortex-mem-core = { path = "../cortex-mem-core" }
rig-core = "0.11"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use cortex_mem_rig::{MemoryTools, create_memory_tools_with_tenant_and_vector};
use cortex_mem_core::llm::{LLMClientImpl, LLMConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM client
    let llm_config = LLMConfig {
        api_base_url: "https://api.openai.com/v1".to_string(),
        api_key: "your-api-key".to_string(),
        model_efficient: "gpt-4o-mini".to_string(),
        temperature: 0.1,
        max_tokens: 4096,
    };
    let llm_client = Arc::new(LLMClientImpl::new(llm_config)?);
    
    // Create memory tools with vector search support
    let memory_tools = create_memory_tools_with_tenant_and_vector(
        "./cortex-data",
        "default",
        llm_client,
        "http://localhost:6333",
        "cortex_memories",
        "https://api.openai.com/v1",
        "your-embedding-key",
        "text-embedding-3-small",
        Some(1536),
        None,
    ).await?;
    
    // Get individual tools for Rig agent
    let abstract_tool = memory_tools.abstract_tool();
    let overview_tool = memory_tools.overview_tool();
    let read_tool = memory_tools.read_tool();
    let search_tool = memory_tools.search_tool();
    let store_tool = memory_tools.store_tool();
    
    // Use with Rig agent...
    
    Ok(())
}
```

## 📚 API Reference

### MemoryTools

Main struct providing access to all memory tools.

```rust
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
}

impl MemoryTools {
    /// Create from existing MemoryOperations
    pub fn new(operations: Arc<MemoryOperations>) -> Self
    
    /// Get underlying operations
    pub fn operations(&self) -> &Arc<MemoryOperations>
    
    // Tiered Access Tools
    pub fn abstract_tool(&self) -> AbstractTool
    pub fn overview_tool(&self) -> OverviewTool
    pub fn read_tool(&self) -> ReadTool
    
    // Search Tools
    pub fn search_tool(&self) -> SearchTool
    pub fn find_tool(&self) -> FindTool
    
    // Filesystem Tools
    pub fn ls_tool(&self) -> LsTool
    pub fn explore_tool(&self) -> ExploreTool
    
    // Storage Tools
    pub fn store_tool(&self) -> StoreTool
}
```

### Factory Functions

```rust
/// Create MemoryTools from existing MemoryOperations
pub fn create_memory_tools(operations: Arc<MemoryOperations>) -> MemoryTools

/// Create MemoryTools with tenant isolation and vector search support
pub async fn create_memory_tools_with_tenant_and_vector(
    data_dir: &str,
    tenant_id: &str,
    llm_client: Arc<dyn LLMClient>,
    qdrant_url: &str,
    qdrant_collection: &str,
    embedding_api_base_url: &str,
    embedding_api_key: &str,
    embedding_model_name: &str,
    embedding_dim: Option<usize>,
    user_id: Option<String>,
) -> Result<MemoryTools, Box<dyn std::error::Error>>
```

## 🛠️ Tool Definitions

### Tiered Access Tools

#### AbstractTool (`"abstract"`)

Get L0 abstract (~100 tokens) for quick relevance checking.

**Parameters:**
```rust
pub struct AbstractArgs {
    pub uri: String,  // Required: Memory URI
}
```

**Response:**
```rust
pub struct AbstractResponse {
    pub uri: String,
    pub abstract_text: String,
    pub layer: String,       // "L0"
    pub token_count: usize,
}
```

#### OverviewTool (`"overview"`)

Get L1 overview (~2000 tokens) for partial context.

**Parameters:**
```rust
pub struct OverviewArgs {
    pub uri: String,  // Required: Memory URI
}
```

**Response:**
```rust
pub struct OverviewResponse {
    pub uri: String,
    pub overview_text: String,
    pub layer: String,       // "L1"
    pub token_count: usize,
}
```

#### ReadTool (`"read"`)

Get L2 full content for deep analysis.

**Parameters:**
```rust
pub struct ReadArgs {
    pub uri: String,  // Required: Memory URI
}
```

**Response:**
```rust
pub struct ReadResponse {
    pub uri: String,
    pub content: String,
    pub layer: String,       // "L2"
    pub token_count: usize,
    pub metadata: Option<FileMetadata>,
}
```

### Search Tools

#### SearchTool (`"search"`)

Intelligent vector search with LLM query rewriting and layered retrieval.

**Parameters:**
```rust
pub struct SearchArgs {
    pub query: String,                    // Required: Search query
    pub recursive: Option<bool>,          // Default: true
    pub return_layers: Option<Vec<String>>, // ["L0", "L1", "L2"]
    pub scope: Option<String>,            // Search scope URI
    pub limit: Option<usize>,             // Default: 10
}
```

**Response:**
```rust
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub engine_used: String,
}

pub struct SearchResult {
    pub uri: String,
    pub score: f32,
    pub snippet: String,
    pub content: Option<String>,
}
```

#### FindTool (`"find"`)

Quick search returning only L0 abstracts.

**Parameters:**
```rust
pub struct FindArgs {
    pub query: String,          // Required: Search query
    pub scope: Option<String>,  // Search scope URI
    pub limit: Option<usize>,   // Default: 10
}
```

**Response:**
```rust
pub struct FindResponse {
    pub query: String,
    pub results: Vec<FindResult>,
    pub total: usize,
}

pub struct FindResult {
    pub uri: String,
    pub score: f32,
    pub abstract_text: String,
}
```

### Filesystem Tools

#### LsTool (`"ls"`)

List directory contents.

**Parameters:**
```rust
pub struct LsArgs {
    pub uri: String,                    // Default: "cortex://session"
    pub recursive: Option<bool>,        // Default: false
    pub include_abstracts: Option<bool>, // Default: false
}
```

**Response:**
```rust
pub struct LsResponse {
    pub uri: String,
    pub entries: Vec<LsEntry>,
    pub total: usize,
}

pub struct LsEntry {
    pub name: String,
    pub uri: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub abstract_text: Option<String>,
}
```

#### ExploreTool (`"explore"`)

Intelligent memory exploration.

**Parameters:**
```rust
pub struct ExploreArgs {
    pub query: String,                   // Required: Exploration query
    pub start_uri: Option<String>,       // Default: "cortex://session"
    pub max_depth: Option<usize>,        // Default: 3
    pub return_layers: Option<Vec<String>>, // Default: ["L0"]
}
```

**Response:**
```rust
pub struct ExploreResponse {
    pub query: String,
    pub exploration_path: Vec<String>,
    pub matches: Vec<ExploreMatch>,
    pub total_explored: usize,
    pub total_matches: usize,
}
```

### Storage Tool

#### StoreTool (`"store"`)

Store content with automatic L0/L1 layer generation.

**Parameters:**
```rust
pub struct StoreArgs {
    pub content: String,                  // Required: Content to store
    pub thread_id: String,                // Default: ""
    pub metadata: Option<Value>,          // Optional metadata
    pub auto_generate_layers: Option<bool>, // Default: true
    pub scope: String,                    // "session", "user", or "agent"
    pub user_id: Option<String>,          // Required for user scope
    pub agent_id: Option<String>,         // Required for agent scope
}
```

**Response:**
```rust
pub struct StoreResponse {
    pub uri: String,
    pub layers_generated: Vec<String>,
    pub success: bool,
}
```

## 🔧 Rig Framework Integration

### Tool Trait Implementation

Each tool implements the `rig::tool::Tool` trait:

```rust
impl Tool for AbstractTool {
    const NAME: &'static str = "abstract";

    type Error = ToolsError;
    type Args = AbstractArgs;
    type Output = AbstractResponse;

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        async {
            ToolDefinition {
                name: Self::NAME.to_string(),
                description: "...".to_string(),
                parameters: json!({ /* JSON Schema */ }),
            }
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(self.operations.get_abstract(&args.uri).await?)
    }
}
```

### Agent Integration Example

```rust
use rig::providers::openai::{Client, GPT_4O_MINI};
use cortex_mem_rig::MemoryTools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenAI client
    let client = Client::from_env();
    
    // Create memory tools
    let memory_tools = create_memory_tools_with_tenant_and_vector(
        "./cortex-data",
        "default",
        llm_client,
        "http://localhost:6333",
        "cortex_memories",
        "https://api.openai.com/v1",
        "your-key",
        "text-embedding-3-small",
        Some(1536),
        None,
    ).await?;
    
    // Create agent with tools
    let agent = client
        .agent(GPT_4O_MINI)
        .preamble("You are an AI assistant with persistent memory capabilities.")
        .tool(memory_tools.abstract_tool())
        .tool(memory_tools.overview_tool())
        .tool(memory_tools.read_tool())
        .tool(memory_tools.search_tool())
        .tool(memory_tools.store_tool())
        .build();
    
    // Use the agent
    let response = agent.prompt(
        "Search for user preferences and store that they like dark theme."
    ).await?;
    
    println!("Agent response: {}", response);
    
    Ok(())
}
```

## 🎯 Best Practices

### Tiered Access Pattern

1. **Use `abstract` first** for quick relevance checking
2. **Use `overview` if relevant** for more context
3. **Use `read` only when necessary** for complete content

### Search Optimization

```rust
// Limit search scope for better performance
agent.prompt("Search 'error handling' in session 'rust-discussion'").await?;

// Use find for quick lookups (returns only L0 abstracts)
agent.prompt("Find memories about 'OAuth' and show abstracts").await?;

// Combine with tiered access
agent.prompt(
    "Search 'async programming', get abstracts for top 3, then read the most relevant one"
).await?;
```

### Memory Storage

```rust
// Store in session scope (default)
agent.prompt("Store 'User is learning Rust async' in current session").await?;

// Store in user scope for long-term memory
agent.prompt("Store 'User prefers dark mode' as a user preference").await?;
```

## 🧪 Testing

```bash
# Run Rig integration tests
cargo test -p cortex-mem-rig

# Run all tests
cargo test --all
```

## 🚨 Common Issues

### Tool Call Failed

Ensure:
- Cortex Memory Core is properly initialized
- Data directory has write permissions
- Search index is built

### Empty Abstract Content

Possible causes:
- File does not exist
- Content too short to generate summary
- LLM service unavailable

### Inaccurate Search Results

Optimization tips:
- Use more specific queries
- Limit search scope
- Use `search` instead of `find` for comprehensive results

## 📦 Dependencies

- `cortex-mem-tools` - High-level memory operations
- `cortex-mem-core` - Core library
- `rig-core` - Rig AI framework
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `anyhow` / `thiserror` - Error handling

## 📄 License

MIT License - see the [LICENSE](../../LICENSE) file for details.

## 🔗 Related Resources

- [Cortex Memory Core](../cortex-mem-core/README.md)
- [Cortex Memory Tools](../cortex-mem-tools/README.md)
- [Rig Framework](https://github.com/0xPlaygrounds/rig)
- [Rig Documentation](https://docs.rs/rig/)

---

**Built with ❤️ using Rust and Rig AI Framework**
