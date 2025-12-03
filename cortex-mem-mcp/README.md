# Cortex Mem MCP Server

An MCP (Model Context Protocol) server that exposes the memo-core memory management capabilities through the MCP stdio protocol.

## Overview

This server allows AI agents and applications using the MCP stdio protocol to interact with the Cortex Mem memory system. It provides tools for:

- **Storing memories**: Save new memories with content, metadata, and associated topics
- **Searching memories**: Find memories using natural language queries with various filters
- **Recalling context**: Retrieve relevant memories to provide context for AI operations
- **Getting specific memories**: Retrieve memories by their unique ID

## Features

- Full MCP stdio protocol support
- Memory management compatible with memo-core and memo-rig tool interfaces
- Support for different memory types (conversational, procedural, factual, semantic, episodic, personal)
- Topic-based memory organization
- Natural language search with similarity scoring
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
      "args": ["--config", "/path/to/your/config.toml"]
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
      "args": ["--config", "/path/to/your/config.toml"]
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

### Available Tools

#### Store Memory

Store a new memory in the system.

Parameters:
- `content` (required, string): The content of the memory to store
- `user_id` (required, string): User ID associated with the memory
- `agent_id` (optional, string): Agent ID associated with the memory
- `memory_type` (optional, string): Type of memory (conversational, procedural, factual, semantic, episodic, personal)
- `topics` (optional, array): Topics to associate with the memory

#### Search Memory

Search for memories using a natural language query.

Parameters:
- `query` (required, string): Search query to find relevant memories
- `user_id` (optional, string): User ID to filter memories
- `agent_id` (optional, string): Agent ID to filter memories
- `memory_type` (optional, string): Memory type to filter by
- `topics` (optional, array): Topics to filter memories by
- `limit` (optional, integer): Maximum number of results to return (default: 10)

#### Recall Context

Recall relevant context based on a query.

Parameters:
- `query` (required, string): Query for context retrieval
- `user_id` (optional, string): User ID to filter memories
- `agent_id` (optional, string): Agent ID to filter memories
- `limit` (optional, integer): Maximum number of context memories to return (default: 5)

#### Get Memory

Retrieve a specific memory by its ID.

Parameters:
- `memory_id` (required, string): ID of the memory to retrieve

## Development

### Project Structure

```
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

## License

This project is licensed under the MIT License.