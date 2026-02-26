# Cortex Memory CLI

`cortex-mem-cli` is the command-line interface for the Cortex Memory system, providing complete terminal access to memory management functionality.

## ✨ Features

- 🗣️ **Session Management**: Create and list sessions
- 💬 **Message Operations**: Add, search, get, and delete messages
- 🔍 **Semantic Search**: Vector-based search with scope filtering
- 📊 **Layer Management**: Generate and manage L0/L1 layer files
- 📈 **Statistics**: View system status and usage statistics
- 🎨 **Friendly Output**: Colored terminal output with configurable verbosity

## 🚀 Quick Start

### Installation

```bash
# Build from source
cd cortex-mem
cargo build --release --bin cortex-mem

# Or run directly
cargo run --bin cortex-mem -- --help
```

### Basic Usage

```bash
# Create a new session
./cortex-mem session create tech-discussion --title "Technical Discussion"

# Add a message
./cortex-mem add --thread tech-discussion "How to implement OAuth authentication?"

# Search for relevant content
./cortex-mem search "OAuth" --thread tech-discussion

# View statistics
./cortex-mem stats
```

## 📖 Command Reference

### Global Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--config` | `-c` | `config.toml` | Path to configuration file |
| `--tenant` | | `default` | Tenant identifier for memory isolation |
| `--verbose` | `-v` | false | Enable verbose/debug logging |

### Session Commands

#### Create Session

```bash
cortex-mem session create <thread-id> [--title <title>]

# Examples
cortex-mem session create project-planning --title "Project Planning Discussion"
cortex-mem session create 2024-01-15-review  # Without title
```

#### List Sessions

```bash
cortex-mem session list
```

Output displays: thread_id, status, created_at, updated_at

### Message Commands

#### Add Message

```bash
cortex-mem add --thread <thread-id> [--role <role>] <content>

# Role options: user, assistant, system (default: user)
cortex-mem add --thread tech-support --role user "I forgot my password, what should I do?"
cortex-mem add --thread tech-support --role assistant "Please visit the password reset page..."
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--thread` | `-t` | (required) | Thread ID for the message |
| `--role` | `-r` | `user` | Message role: `user`, `assistant`, or `system` |
| `content` | | (required) | Message content text |

#### Search Messages

```bash
cortex-mem search <query> [--thread <thread-id>] [-n <limit>] [-s <min-score>] [--scope <scope>]

# Examples
cortex-mem search "password"
cortex-mem search "OAUTH" -n 5 -s 0.7
cortex-mem search "API" --thread tech-support
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `query` | | (required) | Search query text |
| `--thread` | `-t` | None | Thread ID to search within |
| `--limit` | `-n` | `10` | Maximum number of results |
| `--min-score` | `-s` | `0.4` | Minimum relevance score (0.0-1.0) |
| `--scope` | | `session` | Search scope: `session`, `user`, or `agent` |

#### List Memories

```bash
cortex-mem list [--uri <uri>] [--include-abstracts]

# Examples
cortex-mem list
cortex-mem list --uri cortex://user
cortex-mem list --include-abstracts
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--uri` | `-u` | `cortex://session` | URI path to list |
| `--include-abstracts` | | false | Show L0 abstracts in results |

#### Get Memory

```bash
cortex-mem get <uri> [--abstract-only]

# Examples
cortex-mem get cortex://session/tech-support/timeline/2024/01/15/14_30_00_abc123.md
cortex-mem get cortex://session/tech-support/timeline/2024/01/15/14_30_00_abc123.md --abstract-only
```

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `uri` | | (required) | Memory URI to retrieve |
| `--abstract-only` | `-a` | false | Show L0 abstract instead of full content |

#### Delete Memory

```bash
cortex-mem delete <uri>

# Example
cortex-mem delete cortex://session/tech-support/timeline/2024/01/15/14_30_00_abc123.md
```

### Layer Commands

#### Ensure All Layers

Generate missing `.abstract.md` (L0) and `.overview.md` (L1) files for all memories.

```bash
cortex-mem layers ensure-all
```

#### Layer Status

Show L0/L1 file coverage status.

```bash
cortex-mem layers status
```

#### Regenerate Oversized Abstracts

Regenerate `.abstract.md` files exceeding the size limit.

```bash
cortex-mem layers regenerate-oversized
```

### Statistics

```bash
cortex-mem stats
```

Displays:
- Number of sessions
- Number of user memories
- Number of agent memories
- Total message count
- Data directory path

## ⚙️ Configuration

### Configuration File

Create a `config.toml` file with the following structure:

```toml
[cortex]
data_dir = "/path/to/cortex-data"  # Optional, has smart defaults

[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key"
model_efficient = "gpt-4o-mini"
temperature = 0.7
max_tokens = 4096

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "your-embedding-api-key"
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

[qdrant]
url = "http://localhost:6333"
collection_name = "cortex-mem"
embedding_dim = 1536
timeout_secs = 30
```

### Environment Variables

Configuration can be overridden via environment variables:

```bash
export CORTEX_DATA_DIR="/custom/path"
export LLM_API_KEY="your-api-key"
export QDRANT_URL="http://localhost:6333"

cortex-mem session create test
```

### Data Directory Resolution

The data directory is resolved in the following priority order:
1. `cortex.data_dir` config value
2. `CORTEX_DATA_DIR` environment variable
3. System app data directory (e.g., `%APPDATA%/tars/cortex` on Windows)
4. Fallback: `./.cortex` in current directory

## 📝 Complete Workflow Example

```bash
# 1. Create a session
cortex-mem session create customer-support --title "Customer Support Session"

# 2. Add conversation messages
cortex-mem add --thread customer-support "What's my order status?"
cortex-mem add --thread customer-support --role assistant "Let me check your order status..."

# 3. Search for relevant information
cortex-mem search "order" --thread customer-support

# 4. View extracted memories
cortex-mem list --uri cortex://user

# 5. Get a specific memory with abstract
cortex-mem get cortex://session/customer-support/timeline/... --abstract-only

# 6. View system statistics
cortex-mem stats

# 7. Generate missing layer files
cortex-mem layers ensure-all
```

## 🎨 Output Format

CLI uses color coding for better readability:

- 🔵 **Blue**: Session IDs and file URIs
- 🟢 **Green**: Successful operations
- 🟡 **Yellow**: Warning messages
- 🔴 **Red**: Error messages
- ⚪ **White**: General information

## 🔍 Troubleshooting

### Common Issues

**Data directory permission error**
```bash
chmod 755 ./cortex-data
```

**LLM service unavailable**
```bash
export LLM_API_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="your-key"
```

**Vector search failure**
```bash
# Start Qdrant
docker run -p 6333:6333 qdrant/qdrant

# Configure connection
export QDRANT_URL="http://localhost:6333"
```

### Debug Mode

```bash
# Enable verbose logging
cortex-mem --verbose session create debug-test

# View full error stack trace
RUST_BACKTRACE=1 cortex-mem search "test"
```

## 📚 Related Resources

- [Cortex Memory Main Project](../README.md)
- [Core Library Documentation](../cortex-mem-core/README.md)
- [HTTP API Service](../cortex-mem-service/README.md)
- [Architecture Overview](../../litho.docs/en/2.Architecture.md)

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

## 📄 License

MIT License - see the [LICENSE](../../LICENSE) file for details.

---

**Built with ❤️ using Rust and the Cortex Memory Core**