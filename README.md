# Cortex Memory - Rust Agent Memory System

[![Crates.io](https://img.shields.io/crates/v/cortex-mem)](https://crates.io/crates/cortex-mem)
[![Documentation](https://docs.rs/cortex-mem/badge.svg)](https://docs.rs/cortex-mem)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/sopaco/cortex-mem/workflows/CI/badge.svg)](https://github.com/sopaco/cortex-mem/actions)

A high-performance memory system for AI agents written in Rust, inspired by [mem0](https://github.com/mem0ai/mem0). Cortex Memory provides intelligent memory storage, retrieval, and management capabilities for conversational AI systems.

## ✨ Features

- **Intelligent Memory Management**: Automatically extracts, enhances, and organizes key information from conversations
- **Vector-based Search**: Efficient memory retrieval powered by semantic similarity
- **Multiple Memory Types**: Support for conversational, procedural, and factual memories
- **LLM Integration**: Deep integration with Large Language Models for intelligent memory processing
- **Multiple Interfaces**: CLI tools, HTTP API, and Rig framework integration
- **High Performance**: Built with Rust for exceptional performance and memory safety

## 📦 Project Structure

```
cortex-mem/
├── memo-core/          # Core memory management library
├── memo-cli/           # Command-line interface
├── memo-service/       # HTTP API service
├── memo-rig/           # Rig framework integration
└── memo-config/        # Configuration management
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Qdrant vector database
- OpenAI API key (or compatible LLM service)

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cortex-mem = "0.1.0"
```

Or install via:

```bash
cargo install cortex-mem
```

### Basic Usage

```rust
use cortex_mem::{MemoryManager, Config, MemoryType};

// Initialize memory manager
let config = Config::default();
let manager = MemoryManager::new(config)?;

// Add a memory
let memory_id = manager.add_memory(
    "User prefers coffee in the morning",
    Some("user123".to_string()),
    MemoryType::Conversational
).await?;

// Search memories
let results = manager.search_memories(
    "beverage preferences",
    Some("user123".to_string()),
    5
).await?;

for memory in results {
    println!("Found: {} (Score: {})", memory.content, memory.score);
}
```

## 🔧 Configuration

### Environment Variables

```bash
# LLM Configuration
export OPENAI_API_KEY="your-openai-api-key"
export OPENAI_MODEL="gpt-3.5-turbo"
export EMBEDDING_MODEL="text-embedding-ada-002"

# Qdrant Configuration
export QDRANT_URL="http://localhost:6334"
export QDRANT_COLLECTION="memories"

# Optional Configuration
export MAX_TOKENS="1000"
export TEMPERATURE="0.7"
export AUTO_ENHANCE="true"
export DEDUPLICATE="true"
```

### Memory Types

- **Conversational**:对话型记忆，存储对话上下文和用户交互
- **Procedural**: 程序型记忆，存储操作步骤和流程信息
- **Factual**: 事实型记忆，存储客观事实和知识信息
- **Semantic**: 语义型记忆，存储概念和含义
- **Episodic**: 情景型记忆，存储特定事件和经历
- **Personal**: 个人型记忆，存储个人偏好和特征

## 🛠️ CLI Usage

```bash
# Add a memory
cortex-mem add --content "User likes coffee" --user-id "user123"

# Search memories
cortex-mem search --query "coffee" --user-id "user123"

# List memories
cortex-mem list --user-id "user123" --limit 10

# Delete a memory
cortex-mem delete <memory-id>
```

## 🌐 HTTP API

Start the HTTP service:

```bash
# Start service (default port 3000)
cortex-mem-service

# Custom port
PORT=8080 cortex-mem-service
```

API Examples:

```bash
# Health check
curl http://localhost:3000/health

# Create a memory
curl -X POST http://localhost:3000/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User likes coffee",
    "user_id": "user123",
    "memory_type": "conversational"
  }'

# Search memories
curl -X POST http://localhost:3000/memories/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "coffee",
    "user_id": "user123",
    "limit": 10
  }'
```

## 🔌 Rig Framework Integration

```rust
use cortex_mem_rig::{create_memory_tool, MemoryToolConfig};
use std::sync::Arc;

// Create memory tool
let memory_tool = create_memory_tool(
    Arc::new(memory_manager),
    Some(MemoryToolConfig {
        default_user_id: Some("user123".to_string()),
        max_search_results: 10,
        auto_enhance: true,
        ..Default::default()
    })
);

// Use in Rig agent
let agent = client
    .agent("gpt-4")
    .tool(memory_tool)
    .build();
```

## 🏗️ Architecture

### Core Components

1. **MemoryManager**: Central memory management interface
2. **FactExtractor**: Extracts key information from conversations
3. **MemoryUpdater**: Handles memory merging and updates
4. **VectorStore**: Semantic search powered by Qdrant
5. **LLMClient**: Text generation and embedding capabilities

### Data Flow

```
Input Text → Fact Extraction → Memory Update → Vectorization → Storage
                ↓
Search Query → Vector Retrieval → Similarity Ranking → Results
```

## 🧪 Development

```bash
# Clone repository
git clone https://github.com/sopaco/cortex-mem
cd cortex-mem

# Build all components
cargo build --release

# Run tests
cargo test

# Run examples
cargo run --example basic_usage
```

## 📚 Examples

See the [examples](./examples) directory for sample implementations:
- Basic memory operations
- HTTP API server
- Rig framework integration
- Multi-turn interactive chat

## 🤝 Contributing

We welcome contributions! Please ensure:

1. Code passes all tests
2. Follows Rust coding conventions
3. Includes appropriate documentation and tests
4. Updates relevant README sections

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Links

- [Documentation](https://docs.rs/cortex-mem)
- [Qdrant Documentation](https://qdrant.tech/documentation/)
- [OpenAI API Documentation](https://platform.openai.com/docs/)
- [Rig Framework](https://github.com/0xPlaygrounds/rig)

