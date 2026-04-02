# URI Structure Reference

## Overview

Cortex Memory uses a virtual filesystem with the `cortex://` URI scheme. All memory resources are addressed using this format.

## URI Format

```
cortex://{dimension}/{category}/{subcategory}/{resource}
```

## Dimensions

| Dimension | Purpose | Examples |
|-----------|---------|----------|
| `session` | Conversation memories | Timeline, session metadata |
| `user` | User-specific memories | Preferences, entities, events |
| `agent` | Agent-specific memories | Cases, skills, instructions |
| `resources` | General knowledge | Facts, documentation |

---

## Complete URI Structure

```
cortex://
├── session/{session_id}/
│   ├── timeline/
│   │   ├── {YYYY-MM}/                    # Year-month directory
│   │   │   ├── {DD}/                     # Day directory
│   │   │   │   ├── {HH_MM_SS}_{id}.md    # L2: Original message
│   │   │   │   ├── .abstract.md          # L0: Day-level abstract
│   │   │   │   └── .overview.md          # L1: Day-level overview
│   │   │   └── .abstract.md              # L0: Month-level abstract
│   │   ├── .abstract.md                  # L0: Session-level abstract
│   │   └── .overview.md                  # L1: Session-level overview
│   └── .session.json                     # Session metadata
│
├── user/{user_id}/                       # User-specific data (default user_id: "default")
│   ├── preferences/{name}.md             # User preferences
│   ├── entities/{name}.md                # People, projects, concepts
│   ├── events/{name}.md                  # Decisions, milestones
│   ├── personal_info/{name}.md           # User profile info
│   ├── goals/{name}.md                   # User goals
│   ├── relationships/{name}.md           # User relationships
│   └── work_history/{name}.md            # Work history
│
├── agent/{agent_id}/                     # Agent-specific data
│   ├── cases/{name}.md                   # Problem-solution cases
│   ├── skills/{name}.md                  # Acquired skills
│   └── instructions/{name}.md            # Learned instructions
│
└── resources/{resource_name}/            # General knowledge
```

**Important**: The `user_id` in `cortex://user/{user_id}/...` is required. In most scenarios (e.g., MemClaw plugin), the default value is `"default"`.

---

## Layer Files

Each memory can have three representation layers:

| Layer | Filename | Tokens | Purpose |
|-------|----------|--------|---------|
| L0 (Abstract) | `.abstract.md` | ~100 | Quick relevance check |
| L1 (Overview) | `.overview.md` | ~2000 | Understanding gist |
| L2 (Detail) | `{name}.md` | Full | Exact quotes, complete content |

### Layer Resolution Rules

1. **For files** (ending with `.md`):
   - Layer files are in the **same directory** as the content file
   - Example: `cortex://session/abc/timeline/2024-03/15/10_30_00.md`
     - L0: `timeline/2024-03/15/.abstract.md`
     - L1: `timeline/2024-03/15/.overview.md`
     - L2: `timeline/2024-03/15/10_30_00.md`

2. **For directories**:
   - Layer files are **directly in** that directory
   - Example: `cortex://session/abc/timeline`
     - L0: `timeline/.abstract.md`
     - L1: `timeline/.overview.md`

---

## Common URI Examples

### Session Operations

```
# List all sessions
cortex://session

# Browse a specific session
cortex://session/{session_id}

# View timeline messages
cortex://session/{session_id}/timeline

# View timeline for a specific month
cortex://session/{session_id}/timeline/2024-03

# View timeline for a specific day
cortex://session/{session_id}/timeline/2024-03/15

# Access a specific message
cortex://session/{session_id}/timeline/2024-03/15/10_30_45_abc123.md
```

### User Memory Operations

```
# List users (shows user directories)
cortex://user

# List user preferences (requires user_id)
cortex://user/{user_id}/preferences
# Example: cortex://user/default/preferences

# Access a specific preference
cortex://user/{user_id}/preferences/typescript.md
# Example: cortex://user/default/preferences/pref_abc123.md

# List user entities
cortex://user/{user_id}/entities
# Example: cortex://user/default/entities

# Access a specific entity
cortex://user/{user_id}/entities/project_alpha.md
# Example: cortex://user/default/entities/entity_xyz.md

# List user events
cortex://user/{user_id}/events

# Access a specific event
cortex://user/{user_id}/events/launch_decision.md
```

### Agent Memory Operations

```
# List agent cases
cortex://agent/{agent_id}/cases

# Access a specific case
cortex://agent/{agent_id}/cases/case_123.md

# List agent skills
cortex://agent/{agent_id}/skills

# Access a specific skill
cortex://agent/{agent_id}/skills/rust_programming.md
```

---

## API Mapping

| URI | API Endpoint |
|-----|--------------|
| `cortex://session` | `GET /api/v2/filesystem/list?uri=cortex://session` |
| `cortex://session/{id}/timeline` | `GET /api/v2/filesystem/list?uri=cortex://session/{id}/timeline` |
| L0 access | `GET /api/v2/filesystem/abstract?uri={uri}` |
| L1 access | `GET /api/v2/filesystem/overview?uri={uri}` |
| L2 access | `GET /api/v2/filesystem/content?uri={uri}` |

---

## Physical File Mapping

URI to filesystem path mapping:

```
cortex://session/abc/timeline/2024-03/15/msg.md
    ↓
{data_dir}/tenants/{tenant_id}/session/abc/timeline/2024-03/15/msg.md
```

Without tenant:
```
cortex://session/abc/timeline/2024-03/15/msg.md
    ↓
{data_dir}/session/abc/timeline/2024-03/15/msg.md
```

---

## Timeline File Naming

Timeline messages follow the pattern:
```
{HH_MM_SS}_{random_id}.md
```

Example: `14_30_45_abc123.md`
- `14_30_45` - Time (14:30:45)
- `abc123` - Random ID for uniqueness

---

## Session Metadata

Each session has a `.session.json` file:

```json
{
  "thread_id": "session_id",
  "status": "active|closed|archived",
  "created_at": "2024-03-15T10:30:00Z",
  "updated_at": "2024-03-15T12:45:00Z",
  "closed_at": null,
  "message_count": 25,
  "participants": ["user_001", "agent_001"],
  "tags": ["typescript", "api-design"],
  "title": "Optional session title"
}
```

---

## Important Notes

1. **No `memories` subdirectory**: Unlike some documentation might suggest, session memories are NOT under `cortex://session/{id}/memories/`. Extracted memories are stored in user/agent dimensions based on their type.

2. **Layer files are hidden**: `.abstract.md` and `.overview.md` are hidden files. Use `include_layers=true` parameter in `cortex_ls` to see them.

3. **Tenant isolation**: Each tenant has completely separate storage. Switch tenants via API before accessing their data.
