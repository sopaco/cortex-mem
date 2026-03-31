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
- `cortex://user/preferences` - User preference memories

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
  "uri": "cortex://session/project-alpha/conversation.md"
}
```

#### `overview`
Get L1 overview layer for understanding core information.

```json
{
  "uri": "cortex://session/project-alpha/conversation.md"
}
```

#### `content`
Get L2 full content layer - the complete original content.

```json
{
  "uri": "cortex://session/project-alpha/conversation.md"
}
```

### Management Tools

#### `delete`
Delete a memory by its URI.

```json
{
  "uri": "cortex://session/old-project/conversation.md"
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
cortex://session/{thread_id}/conversation.md
cortex://user/{user_id}/preferences/{topic}.md
cortex://user/{user_id}/memories/{memory_id}.md
```

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
model_efficient = "gpt-4o-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key"
model_name = "text-embedding-3-small"

[qdrant]
url = "http://localhost:6333"
collection_name = "cortex_mem"
embedding_dim = 1536
```
