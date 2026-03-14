---
name: memclaw-guide
description: Use MemClaw memory tools to store, search, and recall memories with L0/L1/L2 layered retrieval. Use this skill when the user refers to past conversations, preferences, or needs context from previous sessions.
---

# MemClaw Memory Guide

This skill teaches you how to use MemClaw memory tools for intelligent, layered memory retrieval.

## How Memory Works

MemClaw provides **three-layer semantic memory** with tiered retrieval:

| Layer | Tokens | Content | Role in Search |
|-------|--------|---------|----------------|
| **L0 (Abstract)** | ~100 | High-level summary | Quick filtering by relevance |
| **L1 (Overview)** | ~2000 | Key points + context | Context refinement |
| **L2 (Full)** | Complete | Original content | Precise matching |

**How search works**: The search engine queries all three layers internally (L0→L1→L2) and returns unified results with:
- `snippet`: A brief summary for quick preview
- `content`: Full content when available

**Automatic recall**: At session start, relevant memories are auto-injected into context.

## Tools

### cortex_search

**What it does**: Semantic search across all stored memories.

**When to call**:
- Need to find past conversations or decisions
- Looking for specific information across sessions
- Auto-recall returned nothing

**Parameters**: `query` (required), `scope`, `limit`, `min_score`

**Example**:
```json
{ "query": "database architecture decisions", "limit": 5 }
```

---

### cortex_recall

**What it does**: Recall memories using L0/L1/L2 tiered retrieval internally.

**When to call**:
- `cortex_search` results are not detailed enough
- Need memories with more context (includes snippet and full content)

**Parameters**: `query` (required), `scope`, `limit`

**Example**:
```json
{ "query": "user preferences for code style", "limit": 10 }
```

---

### cortex_add_memory

**What it does**: Store a message in memory for future retrieval.

**When to call**:
- After learning important facts about the user
- User explicitly asks to remember something
- Capturing key outcomes of a conversation

**Parameters**: `content` (required), `role`, `session_id`

**Example**:
```json
{
  "content": "User prefers TypeScript with strict mode",
  "role": "assistant",
  "session_id": "project-frontend"
}
```

---

### cortex_list_sessions

**What it does**: List all memory sessions with status.

**When to call**:
- Want to see available sessions before searching
- Need to pick a specific session scope

**Parameters**: None

---

### cortex_close_session

**What it does**: Close a session and trigger full memory extraction.

**This triggers**:
1. Extract preferences, entities, decisions → user profile
2. Generate L0/L1 summaries for entire session
3. Index all memories into vector database

**When to call**:
- End of a meaningful conversation
- Want user preferences to be permanently stored

**Note**: Takes 30-60 seconds due to LLM calls.

---

### cortex_migrate

**What it does**: Migrate memories from OpenClaw's native memory to MemClaw.

**When to call**: Once during initial setup to preserve existing memories.

**Parameters**: None

---

## Quick Decision Flow

1. **No memories in context or need to find something**
   → Call `cortex_search` with a short, focused query

2. **Search results not detailed enough**
   → Call `cortex_recall` for more context

3. **Want to save something important**
   → Call `cortex_add_memory`

4. **Conversation is complete or before close**
   → Call `cortex_close_session`

5. **First time using MemClaw with existing OpenClaw memory**
   → Call `cortex_migrate`

---

## Writing Good Search Queries

- **Short and focused**: A few words or one clear question
- **Concrete terms**: Names, topics, tools, decisions
- **Derive sub-queries**: If user's message is long, extract key concepts

**Good**: `"database choice for project X"`
**Bad**: `"remember that time we talked about databases and stuff"`

---

## Usage Patterns

### Pattern 1: Context-aware Response

```
1. User asks a question
2. Call cortex_search for relevant context
3. Use returned snippets to enrich response
4. After conversation, cortex_add_memory for new insights
5. When complete, cortex_close_session
```

### Pattern 2: Progressive Detail Retrieval

```
1. Start with cortex_search for quick results
2. If need more context → cortex_recall (returns snippet + content)
```

### Pattern 3: Session-scoped Memory

```
1. cortex_list_sessions to find relevant session
2. Pass session_id as scope in search/recall
3. Store new memories with matching session_id
```

---

## Notes

- Memories are organized by **tenant** and **session**
- L0/L1 layers are generated asynchronously — may not be immediately available
- User preferences are only extracted when `cortex_close_session` is called
- `min_score` of 0.6 is a good starting point
