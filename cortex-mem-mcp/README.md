# Cortex Mem MCP Server

An MCP (Model Context Protocol) server that exposes the cortex-mem memory management capabilities through the MCP stdio protocol, aligned with OpenMemory MCP API design.

## Overview

This MCP server provides a standardized interface for AI agents to store, query, and retrieve memories. The API design follows the OpenMemory MCP specification for better compatibility and consistency across applications.

This server allows AI agents and applications using the MCP stdio protocol to interact with the Cortex Mem memory system with OpenMemory-aligned tools:

- **Storing memories**: Add new memories with content, type, and optional metadata
- **Querying memories**: Unified search interface with salience filtering and natural language queries
- **Listing memories**: Get summarized view of recent memories with filtering options
- **Getting specific memories**: Retrieve memories by their unique ID

## Features

- Full MCP stdio protocol support
- OpenMemory-aligned API design for better compatibility
- Memory management compatible with cortex-mem-core interfaces
- Support for different memory types (conversational, procedural, factual, semantic, episodic, personal)
- Topic-based memory organization
- Natural language search with similarity scoring
- Salience filtering for importance-based retrieval
- Advanced semantic search capabilities
- Configurable memory management parameters

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd cortex-mem/cortex-mem-mcp

# Build and install
cargo install --path .
```

### As a Binary Release

Download the appropriate binary for your platform and add it to your PATH.

## Configuration

The server uses the same configuration system as memo-core. By default, it will look for `config.toml` in the following locations in order:

1. Current working directory
2. User home directory (`~/.config/memo/config.toml`)
3. System configuration directory (platform-specific):
   - macOS: `/usr/local/etc/memo/config.toml`
   - Linux: `/etc/memo/config.toml`
   - Windows: `C:\ProgramData\memo\config.toml`

You can also specify a custom configuration file path:

```bash
# Use default search locations
cortex-mem-mcp

# Specify a custom configuration file
cortex-mem-mcp --config /path/to/your/config.toml

# Use a short form
cortex-mem-mcp -c ~/.memo/config.toml

# Configure agent for automatic agent_id and user_id
cortex-mem-mcp --config /path/to/your/config.toml --agent "my_agent"
```

Create a `config.toml` file with the following contents:

```tom
[llm]
api_key = "your-openai-api-key"
model = "gpt-3.5-turbo"

[embedding]
model = "text-embedding-ada-002"

[qdrant]
url = "http://localhost:6333"
collection_name = "memories"

[memory]
auto_enhance = true
deduplicate = true
similarity_threshold = 0.7
```

See the main Cortex Mem documentation for all configuration options.

## Usage

### With MCP Clients

#### Claude Desktop

Add to your Claude Desktop `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "cortex-mem": {
      "command": "cortex-mem-mcp",
      "args": ["--config", "/path/to/your/config.toml", "--agent", "my_agent"]
    }
  }
}
```

*Note:* Claude Desktop uses the MCP server's working directory (usually where Claude Desktop is running) to resolve relative paths. It's recommended to use absolute paths for the configuration file.

#### Cursor

Add to your `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "cortex-mem": {
      "command": "cortex-mem-mcp",
      "args": ["--config", "/path/to/your/config.toml", "--agent", "my_agent"]
    }
  }
}
```

*Note:* Similar to Claude Desktop, it's recommended to use absolute paths for the configuration file when using Cursor.

#### For Development

To run directly from source:

```bash
cd /path/to/cortex-mem/cortex-mem-mcp
cargo run

# Or with custom config
cargo run -- --config /path/to/config.toml
```

### OpenMemory-aligned API Reference

#### Query Memory
Unified interface for searching memories that replaces both *search_memory* and *recall_context*.

Parameters:
- `query` (required, string): Query string for semantic search
- `k` (optional, integer): Maximum number of results to return (default: 10)
- `memory_type` (optional, string): Type of memory to filter by (conversational, procedural, factual, semantic, episodic, personal)
- `min_salience` (optional, number): Minimum salience/importance score threshold (0-1)
- `topics` (optional, array): Topics to filter memories by
- `user_id` (optional, string): User ID to filter memories (defaults to configured agent's user)
- `agent_id` (optional, string): Agent ID to filter memories (defaults to configured agent)

Special Features:
- **Salience Filtering**: Filter results by importance score to focus on high-value memories

#### Store Memory
Store a new memory in the system. API remains the same.

Parameters:
- `content` (required, string): The content of the memory to store
- `user_id` (optional, string): User ID associated with the memory (required unless --agent was specified on startup)
- `agent_id` (optional, string): Agent ID associated with the memory (defaults to configured agent)
- `memory_type` (optional, string): Type of memory (conversational, procedural, factual, semantic, episodic, personal)
- `topics` (optional, array): Topics to associate with the memory

#### List Memories
Get a summarized view of recent memories with filtering options. This is a new tool aligned with OpenMemory's list functionality.

Parameters:
- `limit` (optional, integer): Maximum number of memories to return (default: 20)
- `memory_type` (optional, string): Type of memory to filter by (conversational, procedural, factual, semantic, episodic, personal)
- `user_id` (optional, string): User ID to filter memories (defaults to configured agent's user)
- `agent_id` (optional, string): Agent ID to filter memories (defaults to configured agent)

Returns simplified memory objects with preview text rather than full content.

#### Get Memory
Retrieve a specific memory by its ID. API remains the same but with improved implementation.

Parameters:
- `memory_id` (required, string): ID of the memory to retrieve

## Development

### Project Structure

```

## Agent Configuration

The `--agent` parameter allows you to configure a default agent identifier that will be automatically used for all memory operations. When configured:

- **agent_id**: Will be set to the value provided via `--agent`
- **user_id**: Will be automatically generated as `user_of_<agent_id>`

This simplifies memory operations by eliminating the need to specify these parameters in each tool call. For more details, see [AGENT_CONFIG.md](AGENT_CONFIG.md).
cortex-mem-mcp/
├── src/
│   ├── lib.rs           # Main implementation
│   └── main.rs          # Server entry point
├── Cargo.toml           # Dependencies and configuration
└── README.md            # This file
```

### Building

```bash
# Build for development
cargo build

# Build for release
cargo build --release

# Run tests
cargo test
```

## Troubleshooting

### Server Fails to Start

1. Check that your `config.toml` is present and valid
2. Verify that Qdrant is running and accessible at the configured URL
3. Check that your OpenAI API key is valid
4. Ensure all required dependencies are installed

### Memory Operations Fail

1. Check that the LLM service is accessible
2. Verify that the vector store is running
3. Check the server logs for detailed error messages

### Migration Guide (v0.1.0+)

If you were using the previous API with `search_memory` and `recall_context`, here's how to migrate:

#### Old API → New API

```json
// Old search_memory call
{
  "name": "search_memory",
  "arguments": {
    "query": "用户的爱好",
    "limit": 10,
    "user_id": "user123"
  }
}

// New query_memory call
{
  "name": "query_memory",
  "arguments": {
    "query": "用户的爱好",
    "k": 10,
    "user_id": "user123"
  }
}
```

```json
// Old recall_context call
{
  "name": "recall_context",
  "arguments": {
    "query": "最近的对话",
    "limit": 5,
    "user_id": "user123"
  }
}

// New query_memory call
{
  "name": "query_memory",
  "arguments": {
    "query": "最近的对话",
    "k": 5,
    "user_id": "user123"
  }
}
```

#### Key Changes

1. **Tool Names**: `search_memory` → `query_memory`, `recall_context` → `query_memory` (unified)
2. **Parameter Names**: `limit` → `k` (for consistency with OpenMemory)
3. **New Features**: Added `min_salience` parameter for importance filtering
4. **New Tool**: Added `list_memories` for browsing memory overview

### Example Usage

Query memories with salience filtering:

```json
{
  "name": "query_memory",
  "arguments": {
    "query": "用户的爱好和偏好",
    "k": 5,
    "min_salience": 0.7,
    "user_id": "user123"
  }
}
```

List memories by type:

```json
{
  "name": "list_memories",
  "arguments": {
    "memory_type": "episodic",
    "limit": 20,
    "user_id": "user123"
  }
}
```

## License

This project is licensed under the MIT License.