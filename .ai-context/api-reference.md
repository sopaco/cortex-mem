# API Reference

## Base URL

```
http://localhost:8085
```

## API Version

All endpoints are under `/api/v2/`.

## Response Format

All responses follow this structure:

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2024-03-15T10:30:00Z"
}
```

Error responses:
```json
{
  "success": false,
  "data": null,
  "error": "Error message",
  "timestamp": "2024-03-15T10:30:00Z"
}
```

---

## Health Endpoints

### GET /health

Basic health check.

**Response:**
```json
{
  "service": "cortex-mem-service",
  "status": "healthy",
  "version": "2.7.0",
  "llm_available": true,
  "timestamp": "2024-03-15T10:30:00Z"
}
```

---

## Filesystem Endpoints

### GET /api/v2/filesystem/list

List directory contents.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `uri` | string | `cortex://session` | Directory URI to list |
| `recursive` | boolean | `false` | List subdirectories recursively |
| `include_abstracts` | boolean | `false` | Include L0 abstracts for files |
| `include_layers` | boolean | `false` | Show `.abstract.md` and `.overview.md` files |

**Example:**
```bash
curl "http://localhost:8085/api/v2/filesystem/list?uri=cortex://session&recursive=true"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "uri": "cortex://session",
    "total": 2,
    "entries": [
      {
        "uri": "cortex://session/default",
        "name": "default",
        "is_directory": true,
        "size": 192,
        "modified": "2024-03-15T10:30:00Z",
        "abstract_text": null
      }
    ]
  }
}
```

### GET /api/v2/filesystem/read/{path}

Read file content directly.

**Example:**
```bash
curl "http://localhost:8085/api/v2/filesystem/read/session/abc/timeline/2024-03/15/msg.md"
```

### POST /api/v2/filesystem/write

Write content to a file.

**Request Body:**
```json
{
  "path": "cortex://user/default/preferences/typescript.md",
  "content": "# TypeScript Preferences\n\nUser prefers strict mode."
}
```

### GET /api/v2/filesystem/stats

Get directory statistics.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `uri` | string | Directory URI |

**Response:**
```json
{
  "success": true,
  "data": {
    "file_count": 42,
    "total_size": 128000
  }
}
```

---

## Layered Access Endpoints

### GET /api/v2/filesystem/abstract

Get L0 abstract layer (~100 tokens).

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `uri` | string | Content URI (file or directory) |

**Example:**
```bash
curl "http://localhost:8085/api/v2/filesystem/abstract?uri=cortex://session/abc/timeline"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "uri": "cortex://session/abc/timeline",
    "content": "Discussion about TypeScript project setup...",
    "layer": "L0",
    "token_count": 95
  }
}
```

### GET /api/v2/filesystem/overview

Get L1 overview layer (~2000 tokens).

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `uri` | string | Content URI (file or directory) |

### GET /api/v2/filesystem/content

Get L2 full content layer.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `uri` | string | Content URI (file only) |

---

## Search Endpoint

### POST /api/v2/search

Semantic search with layered retrieval.

**Request Body:**
```json
{
  "query": "user preferences for TypeScript",
  "thread": "optional-session-id",
  "limit": 10,
  "min_score": 0.6,
  "return_layers": ["L0", "L1"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `query` | string | (required) | Natural language query |
| `thread` | string | null | Filter by session ID |
| `limit` | integer | 10 | Max results |
| `min_score` | float | 0.6 | Minimum relevance score (0-1) |
| `return_layers` | string[] | `["L0"]` | Layers to return: `["L0"]`, `["L0","L1"]`, `["L0","L1","L2"]` |

**Example:**
```bash
curl -X POST "http://localhost:8085/api/v2/search" \
  -H "Content-Type: application/json" \
  -d '{"query": "database decisions", "return_layers": ["L0", "L1"]}'
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "uri": "cortex://session/abc/timeline/2024-03/15/10_30_00.md",
      "score": 0.85,
      "snippet": "Decided to use PostgreSQL...",
      "overview": "Key points: PostgreSQL chosen for...",
      "content": null,
      "source": "search",
      "layers": ["L0", "L1"]
    }
  ]
}
```

---

## Explore Endpoint

### POST /api/v2/filesystem/explore

Smart exploration combining search and browsing.

**Request Body:**
```json
{
  "query": "authentication flow",
  "start_uri": "cortex://session",
  "return_layers": ["L0"]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "query": "authentication flow",
    "exploration_path": [
      {
        "uri": "cortex://session/abc/timeline",
        "relevance_score": 0.82,
        "abstract_text": "Discussion about auth..."
      }
    ],
    "matches": [...],
    "total_explored": 5,
    "total_matches": 2
  }
}
```

---

## Session Endpoints

### GET /api/v2/sessions

List all sessions.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "thread_id": "session-abc",
      "status": "active",
      "message_count": 25,
      "created_at": "2024-03-15T10:30:00Z",
      "updated_at": "2024-03-15T12:45:00Z"
    }
  ]
}
```

### POST /api/v2/sessions

Create a new session.

**Request Body:**
```json
{
  "thread_id": "my-session",
  "title": "Optional title"
}
```

### POST /api/v2/sessions/{thread_id}/messages

Add a message to a session.

**Request Body:**
```json
{
  "role": "user",
  "content": "This is my message content",
  "metadata": {
    "tags": ["important"],
    "importance": "high"
  }
}
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `role` | string | `user`, `assistant`, or `system` |
| `content` | string | Message content |
| `metadata` | object | Optional metadata |

**Response:** Returns the URI of the created message.

### POST /api/v2/sessions/{thread_id}/close

Close a session and trigger memory extraction pipeline.

**Response:**
```json
{
  "success": true,
  "data": {
    "thread_id": "my-session",
    "status": "closed",
    "message_count": 25
  }
}
```

### POST /api/v2/sessions/{thread_id}/close-and-wait

Close session and wait for extraction to complete.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `timeout_secs` | integer | 120 | Max wait time |
| `poll_interval_ms` | integer | 500 | Poll interval |

---

## Tenant Endpoints

### GET /api/v2/tenants

List all available tenants.

**Response:**
```json
{
  "success": true,
  "data": ["tenant_claw", "locomo-v4-001-conv-26"]
}
```

### POST /api/v2/tenants/switch

Switch to a different tenant.

**Request Body:**
```json
{
  "tenant_id": "my-tenant"
}
```

---

## Automation Endpoints

### POST /api/v2/automation/extract/{thread_id}

Trigger memory extraction for a specific thread.

### POST /api/v2/automation/sync

Trigger vector synchronization for all files.

---

## Error Codes

| HTTP Code | Description |
|-----------|-------------|
| 200 | Success |
| 400 | Bad Request - Invalid parameters |
| 404 | Not Found - Resource doesn't exist |
| 500 | Internal Server Error |

---

## Notes

1. **API Version**: Always use `/api/v2/` prefix. `/api/v1/` is deprecated.

2. **Tenant Context**: Most operations require a tenant context. Switch tenants first using `POST /api/v2/tenants/switch`.

3. **Layer Access**: Use the appropriate layer endpoint based on your token budget:
   - L0 (~100 tokens) → Quick filtering
   - L1 (~2000 tokens) → Context understanding
   - L2 (full) → Complete content

4. **Search Weights**: Search uses weighted scoring:
   ```
   Score = 0.2 × L0 + 0.3 × L1 + 0.5 × L2
   ```
