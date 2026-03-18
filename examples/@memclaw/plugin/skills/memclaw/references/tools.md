# MemClaw Tools Reference

Complete reference for all MemClaw tools.

## Search Tools

### cortex_search

Layered semantic search with L0/L1/L2 tiered retrieval.

**Parameters:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| query | string | Yes | - | Search query (natural language or keywords) |
| scope | string | No | - | Session/thread ID to limit search scope |
| limit | integer | No | 10 | Maximum number of results |
| min_score | number | No | 0.6 | Minimum relevance score (0-1) |
| return_layers | ("L0" \| "L1" \| "L2")[] | No | ["L0"] | Which layers to return |

**Response:**
```json
{
  "results": [
    {
      "uri": "cortex://session/abc123/timeline/...",
      "score": 0.85,
      "snippet": "L0 abstract text...",
      "overview": "L1 overview text (if requested)...",
      "content": "L2 full content (if requested)...",
      "layers": ["L0", "L1"],
      "source": "layered_vector"
    }
  ],
  "total": 5
}
```

**Example:**
```typescript
// Minimal tokens - just abstracts
cortex_search({ query: "API design", return_layers: ["L0"] })

// More context needed
cortex_search({ query: "authentication flow", return_layers: ["L0", "L1"] })

// Full content retrieval
cortex_search({ query: "exact error message", return_layers: ["L0", "L1", "L2"] })
```

---

### cortex_recall

Convenience wrapper returning L0 + L2 content.

**Parameters:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| query | string | Yes | - | Search query |
| scope | string | No | - | Session/thread ID to limit scope |
| limit | integer | No | 10 | Maximum number of results |

**Response:** Same as `cortex_search` with `return_layers: ["L0", "L2"]`

---

## Filesystem Tools

### cortex_ls

List directory contents to browse memory structure.

**Parameters:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| uri | string | No | "cortex://session" | Directory URI to list |
| recursive | boolean | No | false | Recursively list subdirectories |
| include_abstracts | boolean | No | false | Include L0 abstracts for files |

**Response:**
```json
{
  "uri": "cortex://session",
  "total": 3,
  "entries": [
    {
      "uri": "cortex://session/abc123",
      "name": "abc123",
      "is_directory": true,
      "size": 0,
      "modified": "2024-01-15T10:30:00Z"
    },
    {
      "uri": "cortex://session/xyz789",
      "name": "xyz789",
      "is_directory": true,
      "size": 0,
      "modified": "2024-01-14T15:00:00Z",
      "abstract_text": "Discussion about API design..."
    }
  ]
}
```

**Example:**
```typescript
// List all sessions
cortex_ls({ uri: "cortex://session" })

// Browse with abstracts
cortex_ls({ 
  uri: "cortex://session/abc123/timeline", 
  include_abstracts: true 
})

// Recursive listing
cortex_ls({ 
  uri: "cortex://session/abc123", 
  recursive: true 
})
```

---

### cortex_explore

Smart exploration combining search and browsing.

**Parameters:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| query | string | Yes | - | Exploration query |
| start_uri | string | No | "cortex://session" | Starting URI |
| return_layers | ("L0" \| "L1" \| "L2")[] | No | ["L0"] | Layers to return in matches |

**Response:**
```json
{
  "query": "authentication",
  "exploration_path": [
    {
      "uri": "cortex://session/abc123/timeline",
      "relevance_score": 0.92,
      "abstract_text": "Discussion about OAuth..."
    }
  ],
  "matches": [
    {
      "uri": "cortex://session/abc123/timeline/...",
      "score": 0.88,
      "snippet": "OAuth implementation...",
      "layers": ["L0"]
    }
  ],
  "total_explored": 5,
  "total_matches": 2
}
```

---

## Tiered Access Tools

### cortex_get_abstract (L0)

Get ~100 token summary for quick relevance check.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| uri | string | Yes | Content URI |

**Response:**
```json
{
  "uri": "cortex://session/abc123/timeline/...",
  "content": "Short abstract...",
  "layer": "L0",
  "token_count": 95
}
```

---

### cortex_get_overview (L1)

Get ~2000 token overview with key information.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| uri | string | Yes | Content URI |

**Response:**
```json
{
  "uri": "cortex://session/abc123/timeline/...",
  "content": "Detailed overview...",
  "layer": "L1",
  "token_count": 1850
}
```

---

### cortex_get_content (L2)

Get full original content.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| uri | string | Yes | Content URI (file only) |

**Response:**
```json
{
  "uri": "cortex://session/abc123/timeline/...",
  "content": "Full original content...",
  "layer": "L2",
  "token_count": 5420
}
```

---

## Storage Tools

### cortex_add_memory

Store a message in memory.

**Parameters:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| content | string | Yes | - | Message content |
| role | string | No | "user" | Message role: "user", "assistant", "system" |
| session_id | string | No | default | Session/thread ID |
| metadata | object | No | - | Optional metadata (tags, importance, etc.) |

**Response:**
```json
{
  "success": true,
  "message_uri": "cortex://session/default/timeline/2024-01/15/10_30_00_abc123.md"
}
```

**Example:**
```typescript
cortex_add_memory({
  content: "User prefers dark mode in all applications",
  role: "assistant",
  metadata: {
    tags: ["preference", "ui"],
    importance: "medium"
  }
})
```

---

### cortex_close_session

Trigger memory extraction pipeline.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| session_id | string | No | Session/thread ID (uses default if not specified) |

**Response:**
```json
{
  "success": true,
  "session": {
    "thread_id": "abc123",
    "status": "closed",
    "message_count": 42
  }
}
```

**Important:** Call this periodically, not just at conversation end.

---

## Maintenance Tools

### cortex_maintenance

Perform periodic maintenance.

**Parameters:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| dryRun | boolean | No | false | Preview changes without executing |
| commands | string[] | No | ["prune", "reindex", "ensure-all"] | Commands to run |

---

### cortex_migrate

Migrate from OpenClaw native memory.

**Parameters:** None

**Response:**
```json
{
  "dailyLogsMigrated": 15,
  "memoryMdMigrated": true,
  "sessionsCreated": ["migrated_2024-01-15"],
  "errors": []
}
```