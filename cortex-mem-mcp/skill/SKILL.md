---
name: cortex-mem-mcp
description: Persistent memory enhancement for AI agents. Store conversations, search memories with semantic retrieval, and recall context across sessions. Use this skill when you need to remember user preferences, past conversations, project context, or any information that should persist beyond the current session. Provides tiered access (abstract/overview/content) for efficient context management.
license: MIT
compatibility: Requires cortex-mem-mcp MCP server running with configured LLM and vector database (Qdrant). Needs API keys for LLM and embedding services.
metadata:
  author: Sopaco
  version: 2.7.0
  homepage: https://github.com/sopaco/cortex-mem
  category: memory
allowed-tools: store search recall ls explore abstract overview content commit delete layers index
---

# Cortex Memory MCP Skill

This skill enables persistent memory capabilities for AI agents, allowing them to store, search, and recall information across sessions using semantic retrieval.

## Prerequisites Check

Before configuring this skill, verify if `cortex-mem-mcp` is available in your system:

```bash
# Check if cortex-mem-mcp is in PATH
which cortex-mem-mcp || where cortex-mem-mcp  # Linux/macOS || Windows
```

If the command returns a path, the binary is already installed. If not, proceed to the installation section below.

## Installation

### Option 1: Install from crates.io (Recommended)

```bash
cargo install cortex-mem-mcp
```

After installation, verify:

```bash
cortex-mem-mcp --version
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem

# Build the release binary
cargo build --release --bin cortex-mem-mcp

# The binary will be at:
# ./target/release/cortex-mem-mcp (Linux/macOS)
# .\target\release\cortex-mem-mcp.exe (Windows)
```

### Option 3: Download Pre-built Binary

Download the latest release from GitHub:

- **GitHub Releases**: https://github.com/sopaco/cortex-mem/releases

Choose the appropriate binary for your platform:
- `cortex-mem-mcp-linux-x86_64` (Linux x64)
- `cortex-mem-mcp-darwin-arm64` (macOS Apple Silicon)
- `cortex-mem-mcp-darwin-x86_64` (macOS Intel)
- `cortex-mem-mcp-windows-x86_64.exe` (Windows x64)

## Configuration

### Step 1: Create Configuration File

Create a `config.toml` file (e.g., `~/.config/cortex-mem/config.toml`):

```toml
[cortex]
# Data directory for storing memories
data_dir = "~/.cortex-data"

[llm]
# LLM API configuration
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key"
model_efficient = "gpt-5-mini"
temperature = 0.1
max_tokens = 65536

[embedding]
# Embedding configuration
api_base_url = "https://api.openai.com/v1"
api_key = "your-embedding-api-key"
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

[qdrant]
# Vector database configuration
url = "http://localhost:6334"
collection_name = "cortex_memories"
embedding_dim = 1536
timeout_secs = 30
```

### Step 2: Start Qdrant (Vector Database)

```bash
# Using Docker
docker run -d -p 6333:6333 qdrant/qdrant

# Verify Qdrant is running
curl http://localhost:6333
```

### Step 3: Configure MCP Client

Configure your MCP client (e.g., Claude Desktop, Cursor, etc.) to use cortex-mem-mcp.

#### Claude Desktop

Edit the configuration file:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

Add the following configuration:

```json
{
  "mcpServers": {
    "cortex-memory": {
      "command": "cortex-mem-mcp",
      "args": [
        "--config", "/path/to/config.toml",
        "--tenant", "default"
      ],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

If you built from source, use the full path to the binary:

```json
{
  "mcpServers": {
    "cortex-memory": {
      "command": "/path/to/cortex-mem/target/release/cortex-mem-mcp",
      "args": [
        "--config", "/path/to/config.toml",
        "--tenant", "default"
      ]
    }
  }
}
```

#### Cursor IDE

Add to your Cursor MCP settings:

```json
{
  "mcpServers": {
    "cortex-memory": {
      "command": "cortex-mem-mcp",
      "args": ["--config", "/path/to/config.toml"]
    }
  }
}
```

### Step 4: Restart Your MCP Client

After configuration, restart Claude Desktop or your MCP client to load the new server.

### Step 5: Verify Installation

Test the MCP server manually:

```bash
# Run with debug logging
RUST_LOG=debug cortex-mem-mcp --config /path/to/config.toml --tenant default
```

## Command-line Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `--config` / `-c` | `config.toml` | Path to configuration file |
| `--tenant` | `default` | Tenant ID for memory isolation |
| `--auto-trigger-threshold` | `10` | Message count to auto-trigger memory extraction |
| `--auto-trigger-interval` | `300` | Min seconds between auto-trigger executions |
| `--auto-trigger-inactivity` | `120` | Inactivity timeout to trigger extraction |
| `--no-auto-trigger` | `false` | Disable auto-trigger feature entirely |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CORTEX_DATA_DIR` | Override data directory path |
| `RUST_LOG` | Logging level (debug, info, warn, error) |

## When to Use This Skill

Use this skill when you need to:
- **Remember user preferences** - Store and recall user-specific settings, preferences, and context
- **Persist conversation context** - Keep important information from past conversations accessible
- **Build project knowledge** - Accumulate and retrieve project-specific information over time
- **Track user-agent interactions** - Maintain a history of interactions for better personalization
- **Search memories semantically** - Find relevant information using natural language queries

## Available Tools

### Storage Tools

#### `store`
Add a message to memory for a specific session.

```json
{
  "content": "The user prefers dark mode in all applications",
  "thread_id": "project-alpha",
  "role": "user"
}
```

- `content`: The message content to store
- `thread_id`: Optional session/thread identifier (defaults to "default")
- `role`: Message role - "user", "assistant", or "system"

#### `commit`
Commit accumulated conversation content and trigger memory extraction.

```json
{
  "thread_id": "project-alpha"
}
```

This triggers:
- Memory extraction (session → user/agent memories)
- L0/L1 layer generation
- Vector indexing

### Search Tools

#### `search`
Layered semantic search across memory using L0/L1/L2 tiered retrieval.

```json
{
  "query": "user preferences for UI",
  "scope": "project-alpha",
  "limit": 10,
  "min_score": 0.5,
  "return_layers": ["L0", "L1"]
}
```

#### `recall`
Recall memories with full context (L0 snippet + L2 content).

```json
{
  "query": "what did we discuss about authentication",
  "scope": "project-alpha",
  "limit": 5
}
```

### Navigation Tools

#### `ls`
List directory contents to browse the memory space.

```json
{
  "uri": "cortex://session",
  "recursive": true,
  "include_abstracts": true
}
```

Common URIs:
- `cortex://session` - List all sessions
- `cortex://user` - List user-level memories
- `cortex://user/{user_id}/preferences` - User preference memories

#### `explore`
Smart exploration of memory space, combining search and browsing.

```json
{
  "query": "authentication implementation details",
  "start_uri": "cortex://session",
  "return_layers": ["L0"]
}
```

### Tiered Access Tools

Memory is organized in layers for efficient context management:

| Layer | Size | Purpose |
|-------|------|---------|
| L0 | ~100 tokens | Quick relevance checking (abstract) |
| L1 | ~2000 tokens | Understanding core information (overview) |
| L2 | Full content | Complete original content |

#### `abstract`
Get L0 abstract layer for quick relevance checking.

```json
{
  "uri": "cortex://session/project-alpha/timeline/2024-03/15/10_30_45_abc123.md"
}
```

#### `overview`
Get L1 overview layer for understanding core information.

```json
{
  "uri": "cortex://session/project-alpha/timeline/2024-03/15/10_30_45_abc123.md"
}
```

#### `content`
Get L2 full content layer - the complete original content.

```json
{
  "uri": "cortex://session/project-alpha/timeline/2024-03/15/10_30_45_abc123.md"
}
```

### Management Tools

#### `delete`
Delete a memory by its URI.

```json
{
  "uri": "cortex://session/old-project/timeline/2024-03/15/10_30_45_xyz.md"
}
```

#### `layers`
Generate L0/L1 layer files for memories.

```json
{
  "thread_id": "project-alpha"
}
```

#### `index`
Index memory files for vector search.

```json
{
  "thread_id": "project-alpha"
}
```

## Memory URI Structure

Memories are organized using a URI scheme:

```
cortex://session/{thread_id}/timeline/{YYYY-MM}/{DD}/{HH_MM_SS}_{id}.md
cortex://user/{user_id}/preferences/{topic}.md
cortex://user/{user_id}/entities/{name}.md
cortex://user/{user_id}/events/{name}.md
cortex://agent/{agent_id}/cases/{name}.md
cortex://agent/{agent_id}/skills/{name}.md
```

**Note**: Session dimension stores conversation timeline; extracted memories (preferences, entities, etc.) are stored in user/agent dimensions after `commit`.

## Best Practices

1. **Use meaningful thread IDs** - Use descriptive names like `project-alpha` or `user-123-support` instead of generic IDs

2. **Commit periodically** - Call `commit` after significant conversation milestones to ensure memory extraction

3. **Start with search** - Before storing new information, search to avoid duplication

4. **Use tiered access** - Start with `abstract` or `search` to find relevant memories, then use `overview` or `content` for details

5. **Scope your searches** - Use the `scope` parameter to limit searches to relevant sessions

## Example Workflow

### Storing a User Preference

```
1. Store the preference:
   store(content="User prefers TypeScript over JavaScript for all new projects", role="user")

2. Commit to persist:
   commit()
```

### Recalling Past Context

```
1. Search for relevant memories:
   search(query="TypeScript preferences", limit=5)

2. Get overview of most relevant result:
   overview(uri="cortex://user/default/preferences/typescript.md")
```

### Building Project Knowledge

```
1. Store project decisions:
   store(content="Decided to use PostgreSQL for the main database", thread_id="project-x", role="assistant")

2. Later, recall project decisions:
   recall(query="database decisions", scope="project-x")
```

## Auto-Trigger Feature

The MCP server supports automatic memory processing:
- Triggers after configurable message count threshold (default: 10)
- Triggers after inactivity timeout (default: 2 minutes)
- Can be disabled with `--no-auto-trigger` flag

## Configuration

The MCP server requires a `config.toml` with:

```toml
[cortex]
data_dir = "./cortex-data"

[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key"
model_efficient = "gpt-5-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key"
model_name = "text-embedding-3-small"

[qdrant]
url = "http://localhost:6333"
collection_name = "cortex_mem"
embedding_dim = 1536
```
