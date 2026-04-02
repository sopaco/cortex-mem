# Project Overview

## What is Cortex Memory?

Cortex Memory is a **high-performance, AI-native memory framework** written in Rust. It provides persistent, intelligent long-term memory for AI agents and applications.

## Core Value Proposition

Transform stateless AI into context-aware, intelligent partners that:
- Remember user preferences across sessions
- Learn and adapt over time
- Maintain context across multiple conversations
- Build personalized experiences

## Key Features

| Feature | Description |
|---------|-------------|
| **Virtual Filesystem** | Memory stored as markdown files via `cortex://` URI scheme |
| **Three-Tier Hierarchy** | L0 Abstract → L1 Overview → L2 Detail for token-efficient retrieval |
| **Vector Search** | Semantic search via Qdrant with weighted L0/L1/L2 scoring |
| **Memory Extraction** | LLM-powered extraction of structured memories from conversations |
| **Multi-Tenancy** | Isolated memory spaces for different users/agents |
| **Multi-Modal Access** | REST API, CLI, MCP protocol, Rust library |

## Architecture Highlights

```
Input (User/Agent Messages)
         │
         ▼
┌─────────────────────────────────────┐
│         cortex-mem-core             │
│  ┌──────────┐  ┌─────────────────┐  │
│  │ Session  │  │ Memory Extractor│  │
│  │ Manager  │  │ (LLM-powered)   │  │
│  └────┬─────┘  └────────┬────────┘  │
│       │                  │          │
│       ▼                  ▼          │
│  ┌──────────────────────────────┐   │
│  │    Virtual Filesystem        │   │
│  │    (cortex:// URI)           │   │
│  └──────────────┬───────────────┘   │
│                 │                   │
│       ┌────────┴────────┐           │
│       ▼                 ▼           │
│  ┌─────────┐     ┌───────────────┐  │
│  │ Layer   │     │ Vector Search │  │
│  │ Generator│     │ Engine        │  │
│  └─────────┘     └───────┬───────┘  │
└──────────────────────────┼──────────┘
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
   ┌─────────┐       ┌──────────┐       ┌─────────┐
   │ Filesystem│      │ Qdrant   │       │ LLM API │
   │ (Markdown)│      │ (Vector) │       │         │
   └──────────┘       └──────────┘       └─────────┘
```

## Project Ecosystem

### Core Crates

| Crate | Purpose |
|-------|---------|
| `cortex-mem-core` | Core business logic, filesystem, search, extraction |
| `cortex-mem-service` | REST API server (Axum, port 8085) |
| `cortex-mem-cli` | Command-line interface |
| `cortex-mem-mcp` | Model Context Protocol server |
| `cortex-mem-tools` | MCP tool schemas and operations |
| `cortex-mem-rig` | Rig framework integration |
| `cortex-mem-config` | Configuration management |

### Example Applications

| Example | Description |
|---------|-------------|
| `examples/@memclaw/plugin` | OpenClaw memory plugin (MemClaw) |
| `examples/cortex-mem-tars` | TUI AI assistant with voice memory |
| `examples/locomo-evaluation` | Benchmark evaluation scripts |

### Frontend

| Component | Description |
|-----------|-------------|
| `cortex-mem-insights` | Svelte 5 SPA dashboard for monitoring |

## Performance Highlights

Based on LoCoMo10 benchmark (152 questions):

| Metric | Value |
|--------|-------|
| Overall Score | 68.42% |
| Multi-hop Reasoning | 84.29% |
| Token Efficiency | 11× fewer than OpenClaw+LanceDB |
| Score per 1K Tokens | 23.6 |

## Technology Stack

- **Language**: Rust 1.86+ (Edition 2024)
- **Async Runtime**: Tokio
- **Web Framework**: Axum 0.7
- **Vector Database**: Qdrant
- **Serialization**: serde, serde_json
- **CLI**: clap 4.5
- **Frontend**: Svelte 5, TypeScript, Vite

## Use Cases

1. **AI Chatbots & Assistants** - Long-term memory for personalized interactions
2. **Agent Frameworks** - Memory backbone for AI agents (Rig, MCP)
3. **OpenClaw Integration** - Enhanced memory via MemClaw plugin
4. **Knowledge Management** - Structured memory extraction from conversations
