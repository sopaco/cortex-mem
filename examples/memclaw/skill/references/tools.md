# Tools Reference

Detailed documentation for MemClaw tools.

## cortex_search

Semantic search across all memories using L0/L1/L2 tiered retrieval.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | The search query - natural language or keywords |
| `scope` | string | No | - | Session/thread ID to limit search scope |
| `limit` | integer | No | 10 | Maximum number of results |
| `min_score` | number | No | 0.6 | Minimum relevance score (0-1) |

**When to use:**
- Find past conversations or decisions
- Search for specific information across all sessions
- Discover related memories by semantic similarity

**Example:**
```json
{
  "query": "database architecture decisions",
  "limit": 5,
  "min_score": 0.6
}
```

**Response format:**
- Returns ranked results with relevance scores
- Each result includes `uri`, `score`, and `snippet`

---

## cortex_recall

Recall memories with more context (snippet + full content).

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | The search query |
| `scope` | string | No | - | Session/thread ID to limit search scope |
| `limit` | integer | No | 10 | Maximum number of results |

**When to use:**
- Need memories with full context, not just summaries
- Want to see the original content, not just snippets
- Conducting detailed memory analysis

**Example:**
```json
{
  "query": "user preferences for code style",
  "limit": 10
}
```

**Response format:**
- Returns results with both `snippet` (summary) and `content` (full text)
- Content is truncated if very long (>300 chars preview)

---

## cortex_add_memory

Store a message for future retrieval.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `content` | string | Yes | - | The content to store in memory |
| `role` | string | No | `user` | Role of the message sender: `user`, `assistant`, or `system` |
| `session_id` | string | No | `default` | Session/thread ID for the memory |

**When to use:**
- Persist important information for future retrieval
- Store user preferences or decisions
- Save context that should be searchable later

**Example:**
```json
{
  "content": "User prefers TypeScript with strict mode enabled and explicit return types",
  "role": "assistant",
  "session_id": "default"
}
```

**What happens:**
- Message is stored with timestamp
- Vector embedding is generated automatically
- L0/L1 layers are generated asynchronously

---

## cortex_list_sessions

List all memory sessions with their status.

**Parameters:** None

**When to use:**
- Verify sessions exist before searching
- Check which sessions are active or closed
- Audit memory usage

**Response format:**
- Session IDs, status, message counts
- Creation and update timestamps

---

## cortex_close_session

Close a session and trigger memory extraction pipeline.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `session_id` | string | No | `default` | Session/thread ID to close |

**When to use:**
- Conversation is complete
- Ready to extract structured memories
- Want to finalize the session's memory content

**What happens:**
1. Extracts structured memories (user preferences, entities, decisions)
2. Generates complete L0/L1 layer summaries
3. Indexes all extracted memories into the vector database

**Note:** This is a potentially long-running operation (30-60 seconds).

**Example:**
```json
{
  "session_id": "default"
}
```

---

## cortex_migrate

Migrate memories from OpenClaw's native memory system to MemClaw.

**Parameters:** None

**When to use:**
- First time setup with existing OpenClaw memory
- Want to preserve previous conversation history
- Switching from built-in memory to MemClaw

**What happens:**
1. Finds OpenClaw memory files (`memory/*.md` and `MEMORY.md`)
2. Converts them to MemClaw's L2 format
3. Generates L0/L1 layers and vector index

**Prerequisites:**
- OpenClaw workspace exists at `~/.openclaw/workspace`
- Memory files exist in `~/.openclaw/workspace/memory/`

**Run only once during initial setup.**
