# Tools Reference

Detailed documentation for MemClaw tools.

## cortex_search

Semantic search using L0/L1/L2 hierarchical retrieval.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | Search query — natural language or keywords |
| `scope` | string | No | - | Session/thread ID to limit search scope |
| `limit` | integer | No | 10 | Maximum number of results |
| `min_score` | number | No | 0.6 | Minimum relevance score (0-1) |

**Use Cases:**
- Find past conversations or decisions
- Search for specific information across all sessions
- Discover related memories through semantic similarity

**Example:**
```json
{
  "query": "database architecture decisions",
  "limit": 5,
  "min_score": 0.6
}
```

**Response Format:**
- Returns results sorted by relevance
- Each result contains `uri`, `score`, and `snippet`

---

## cortex_recall

Retrieve memories with more context (summary + full content).

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | Search query |
| `scope` | string | No | - | Session/thread ID to limit search scope |
| `limit` | integer | No | 10 | Maximum number of results |

**Use Cases:**
- Need memories with full context, not just summaries
- Want to see original content
- Performing detailed memory analysis

**Example:**
```json
{
  "query": "user code style preferences",
  "limit": 10
}
```

**Response Format:**
- Returns results with `snippet` (summary) and `content` (full text)
- Content is truncated when too long (preview >300 characters)

---

## cortex_add_memory

Store messages for later retrieval.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `content` | string | Yes | - | Memory content to store |
| `role` | string | No | `user` | Message sender role: `user`, `assistant`, or `system` |
| `session_id` | string | No | `default` | Session/thread ID the memory belongs to |

**Use Cases:**
- Persist important information for later retrieval
- Store user preferences or decisions
- Save context that should be searchable

**Example:**
```json
{
  "content": "User prefers TypeScript with strict mode enabled",
  "role": "assistant",
  "session_id": "default"
}
```

**Execution Effects:**
- Message is stored with timestamp
- Vector embedding is automatically generated
- L0/L1 layers are generated asynchronously

---

## cortex_list_sessions

List all memory sessions and their status.

**Parameters:** None

**Use Cases:**
- Verify sessions exist before searching
- Check which sessions are active or closed
- Audit memory usage

**Response Format:**
- Session ID, status, message count
- Creation and update timestamps

---

## cortex_close_session

Close a session and trigger the memory extraction process.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `session_id` | string | No | `default` | Session/thread ID to close |

**Use Cases:**
- When a conversation is complete
- Preparing to extract structured memories
- Wanting to finalize a session's memory content

**Execution Effects:**
1. Extracts structured memories (user preferences, entities, decisions)
2. Generates complete L0/L1 layer summaries
3. Indexes all extracted memories into the vector database

**Note:** This can be a longer operation (30-60 seconds).

**Example:**
```json
{
  "session_id": "default"
}
```

> **Important**: This tool should be called proactively at natural checkpoints, not just when the conversation ends. Ideal timing: after completing important tasks, topic transitions, or accumulating enough conversation content.

---

## cortex_migrate

Migrate from OpenClaw's native memory system to MemClaw.

**Parameters:** None

**Use Cases:**
- First-time use with existing OpenClaw memories
- Want to preserve previous conversation history
- Switching from built-in memory to MemClaw

**Execution Effects:**
1. Finds OpenClaw memory files (`memory/*.md` and `MEMORY.md`)
2. Converts to MemClaw's L2 format
3. Generates L0/L1 layers and vector indices

**Prerequisites:**
- OpenClaw workspace exists at `~/.openclaw/workspace`
- Memory files exist at `~/.openclaw/workspace/memory/`

**Run only once during initial setup.**

---

## cortex_maintenance

Perform periodic maintenance on MemClaw data.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `dryRun` | boolean | No | false | Preview changes without executing |
| `commands` | array | No | `["prune", "reindex", "ensure-all"]` | Maintenance commands to execute |

**Use Cases:**
- Search results are incomplete or outdated
- Recovering from crash or data corruption
- Need to clean up disk space

**Available Commands:**
- `prune` — Remove vectors whose source files no longer exist
- `reindex` — Rebuild vector indices and remove stale entries
- `ensure-all` — Generate missing L0/L1 layer files

**Example:**
```json
{
  "dryRun": false,
  "commands": ["prune", "reindex", "ensure-all"]
}
```

> **Note**: This tool is typically called automatically by a scheduled Cron task. Manual invocation is for troubleshooting or on-demand maintenance.