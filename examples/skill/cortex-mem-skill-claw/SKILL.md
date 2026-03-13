---
name: cortex-mem
description: Layered semantic memory for OpenClaw using Cortex Memory — supports L0/L1/L2 tiered recall, semantic search, session-isolated memory, and persistent memory capture across conversations.
homepage: https://github.com/sopaco/cortex-mem
user-invocable: true
metadata: { "openclaw": { "emoji": "🧠", "homepage": "https://github.com/sopaco/cortex-mem", "requires": { "config": ["plugins.entries.cortex-mem.enabled"] } } }
---

# Cortex Memory Skill

This skill teaches the agent how to use the Cortex Memory plugin (`cortex-mem`) to store, search, and recall memories across conversations.

## Agent Autonomy: Setup and Dependency Management

As an agent using this skill, you have the autonomy and responsibility to **detect, install, and configure** the required dependencies before calling any of the memory tools. **Do not** ask the user to do this manually unless an error occurs that you cannot resolve.

### Step 1: Check and Setup Qdrant Vector Database
The memory service requires Qdrant to store vector embeddings.
1. **Detect**: Check if Qdrant is already running locally on port `6333` (e.g., via `curl -s http://127.0.0.1:6333` or checking running processes).
2. **Install & Run**: If it is not running, run Qdrant via Docker in the background:
   ```bash
   docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant
   ```

### Step 2: Check and Setup `cortex-mem-service`
The tools communicate with the local `cortex-mem-service`.
1. **Detect**: Check if the service is already running on port `8085` (e.g., via `curl -s http://127.0.0.1:8085/health`).
2. **Check Binary**: If not running, check if `cortex-mem-service` is available in the environment (e.g., `which cortex-mem-service`).
3. **Install**: If the binary is missing, install it using Cargo:
   ```bash
   cargo install cortex-mem-service
   ```
4. **Run**: Start the service in the background. **Crucially**, you must specify a fixed data directory suitable for OpenClaw (`~/.openclaw/cortex-data`):
   ```bash
   mkdir -p ~/.openclaw/cortex-data
   nohup cortex-mem-service --data-dir ~/.openclaw/cortex-data > ~/.openclaw/cortex-data/service.log 2>&1 &
   ```
   *Wait a few seconds and verify it started successfully on port `8085`.*

### Step 3: Verify OpenClaw Plugin Configuration
Ensure the plugin is configured in `~/.openclaw/openclaw.json` (you can read and update this file if needed).

---

## Plugin Setup

The Cortex Memory plugin provides **five tools**. All tools communicate with the `cortex-mem-service` running locally over HTTP.

**Required:** The `cortex-mem` plugin must be enabled in your `~/.openclaw/openclaw.json`:

```json5
{
  plugins: {
    entries: {
      "cortex-mem": {
        enabled: true,
        config: {
          serviceUrl: "http://127.0.0.1:8085",
          tenantId: "tenant_claw",
          defaultSessionId: "default",
          searchLimit: 10,
          minScore: 0.6
        }
      }
    }
  }
}
```

**Start the service** before using any memory tools:
```bash
cargo run -p cortex-mem-service -- --data-dir ./cortex-data
```

## Available Tools

### `cortex_search` — Semantic Memory Search

Search across all stored memories using natural language. Returns results ranked by relevance.

**When to use:**
- Finding past decisions, discussions, or facts from previous sessions
- Discovering related context for the current task
- Quick lookup of stored knowledge

**Parameters:**
- `query` (required): Natural language search query
- `scope` (optional): Limit search to a specific session/thread ID
- `limit` (optional): Max results, default `10`
- `min_score` (optional): Relevance threshold (0–1), default `0.6`

**Example:**
```json
{ "query": "database architecture decisions", "limit": 5 }
```

---

### `cortex_recall` — Layered Memory Recall

Retrieve memories with configurable detail levels using the L0/L1/L2 layer system.

**Layer system:**
| Layer | Tokens | Content |
|-------|--------|---------|
| L0 | ~100 | Abstract — quick relevance summary |
| L1 | ~2000 | Overview — key points and context |
| L2 | Full | Complete original content |

**When to use:**
- When `cortex_search` results are not detailed enough
- When you need to understand the full context of a past conversation
- Start with `L0`, escalate to `L1` or `L2` only if needed (to save tokens)

**Parameters:**
- `query` (required): Search query
- `layers` (optional): Array of layers to return, default `["L0"]`
- `scope` (optional): Session/thread ID filter
- `limit` (optional): Max results, default `5`

**Example:**
```json
{ "query": "user preferences for code style", "layers": ["L0", "L1"] }
```

---

### `cortex_add_memory` — Store Memory

Persist important information to the Cortex Memory store. Content is automatically vectorized for future semantic search.

**When to use:**
- After learning important facts about the user (preferences, project details, decisions)
- When the user explicitly asks to remember something
- To capture key outcomes of a conversation for future reference

**Parameters:**
- `content` (required): The text to store
- `role` (optional): `"user"` | `"assistant"` | `"system"`, default `"user"`
- `session_id` (optional): Target session; uses `defaultSessionId` if omitted

**Example:**
```json
{
  "content": "User prefers TypeScript with strict mode and functional patterns",
  "role": "assistant",
  "session_id": "project-frontend"
}
```

---

### `cortex_list_sessions` — List Memory Sessions

List all available memory sessions with their status and message counts.

**When to use:**
- To see what sessions/contexts exist before searching
- To pick a specific session scope for `cortex_search` or `cortex_recall`

**Parameters:** None

---

### `cortex_close_session` — Close Session & Extract Memories

Close a session and trigger the **full memory extraction pipeline**:
1. LLM extracts user preferences, entities, decisions → stored in `user/` directory
2. Complete L0/L1 summaries regenerated for the entire session
3. All extracted memories indexed into the vector database

**When to use:**
- At the end of a meaningful conversation or task
- When you want user preferences/entities to be persistently stored
- When you need complete L0/L1 summaries (not just the per-message incremental ones)

**Parameters:**
- `session_id` (optional): Session to close; uses `defaultSessionId` if omitted

**Example:**
```json
{ "session_id": "project-frontend" }
```

> ⚠️ This triggers LLM calls and may take 30–60 seconds. Call it once when a conversation is complete, not after every message.

---

## Usage Patterns

### Pattern 1: Context-aware response

Before answering a complex question, search for relevant past context:

1. Call `cortex_search` with the topic as query
2. Use returned snippets to enrich your response
3. After the conversation, call `cortex_add_memory` to store new insights
4. When the conversation is complete, call `cortex_close_session` to extract and consolidate all memories

### Pattern 2: Progressive detail retrieval

When you need more context without loading everything:

1. Call `cortex_recall` with `layers: ["L0"]` to scan for relevant memories (minimal tokens)
2. If you find a relevant memory, call `cortex_recall` again with `layers: ["L1"]` for overview
3. Only use `layers: ["L2"]` when you truly need the full original content

### Pattern 3: Session-scoped memory

For project-specific or topic-specific memory management:

1. Use `cortex_list_sessions` to find existing session IDs
2. Pass the matching session ID as `scope` in search/recall calls
3. Store new information with `session_id` set to the relevant session

---

## Layer Architecture

```
Memory Query
    │
    ▼
L0 Abstract (~100 tokens)
    │  quick relevance scan
    ▼
L1 Overview (~2000 tokens)
    │  key context
    ▼
L2 Full Content
    │  complete details (use sparingly)
    ▼
cortex-mem-service (Rust)
    ├─ Qdrant (vector search)
    ├─ Filesystem (content store)
    └─ LLM (layer generation)
```

## Notes

- Memories are organized by **tenant** (`tenantId` config) and **session** (`session_id`)
- L0 and L1 layers are generated asynchronously after `cortex_add_memory` — they may not be immediately available
- **User preferences and entities** are only extracted when `cortex_close_session` is called
- `minScore` of `0.6` is a good starting point; lower it if results are too sparse, raise it for higher precision
- The service must be running at `serviceUrl` before any tool calls will succeed
