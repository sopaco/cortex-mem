# MemClaw Best Practices

This guide provides proven strategies and decision frameworks for using MemClaw effectively in OpenClaw.

## Tool Selection Decision Tree

```
┌─────────────────────────────────────────────────────────────────┐
│                    What do you need to do?                      │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
   Find Info            Save Info             Manage Sessions
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│ Need context? │    │   What kind?  │    │    What?      │
└───────────────┘    └───────────────┘    └───────────────┘
        │                     │                     │
   ┌────┴────┐           ┌────┴────┐          ┌────┴────┐
   ▼         ▼           ▼         ▼          ▼         ▼
 Quick    Full        Facts    Conversation  List    Close &
 Search   Context               History     Sessions  Extract
   │         │           │         │           │         │
   ▼         ▼           ▼         ▼           ▼         ▼
cortex_   cortex_   cortex_   cortex_     cortex_   cortex_
search    recall    add_      add_        list_     close_
                    memory    memory      sessions  session
```

### Quick Reference

| Scenario | Tool | Why |
|----------|------|-----|
| Quick lookup, need summary only | `cortex_search` | Fast, returns snippets |
| Need full context/details | `cortex_recall` | Returns content + snippet |
| User stated preference/decision | `cortex_add_memory` | Explicit persistence |
| Important conversation content | Let it accumulate | Auto-stored in session |
| Task/topic completed | `cortex_close_session` | Trigger extraction |
| Check if memories exist | `cortex_list_sessions` | Verify before search |

## Session Lifecycle Management

### The Golden Rule

> **OpenClaw does NOT automatically trigger memory extraction.** You must proactively call `cortex_close_session` at natural checkpoints.

### When to Close a Session

```
┌────────────────────────────────────────────────────────────────┐
│                    Timing Decision Flow                        │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ Task completed? │
                    └─────────────────┘
                         │      │
                        Yes     No
                         │      │
                         ▼      ▼
                  ┌──────────┐  ┌─────────────────┐
                  │ CLOSE IT │  │ Topic shifted?  │
                  └──────────┘  └─────────────────┘
                                     │      │
                                    Yes     No
                                     │      │
                                     ▼      ▼
                              ┌──────────┐  ┌─────────────────┐
                              │ CLOSE IT │  │ 10+ exchanges?  │
                              └──────────┘  └─────────────────┘
                                                 │      │
                                                Yes     No
                                                 │      │
                                                 ▼      ▼
                                          ┌──────────┐  ┌────────┐
                                          │ CLOSE IT │  │ WAIT   │
                                          └──────────┘  └────────┘
```

### Rhythm Guidelines

| Conversation Type | Close Frequency | Reason |
|-------------------|-----------------|--------|
| Quick Q&A | End of conversation | Minimal content to extract |
| Task-oriented | After each task completion | Captures task-specific memories |
| Long discussion | Every 10-20 exchanges | Prevents memory loss |
| Exploratory chat | When topic shifts | Organizes memories by topic |

### Anti-Patterns to Avoid

| ❌ Don't Do This | ✅ Do This Instead |
|-------------------|-------------------|
| Call `close_session` after every message | Call at natural checkpoints |
| Wait until conversation ends (user may forget) | Proactively close during conversation |
| Close without accumulating content | Let 5-10 exchanges happen first |
| Never close sessions | Establish a rhythm |

## Memory Storage Strategy

### What to Explicitly Store

Use `cortex_add_memory` for:

1. **Explicit User Preferences**
   ```
   "User prefers dark theme in all editors"
   "User wants commit messages in conventional format"
   ```

2. **Important Decisions**
   ```
   "Decided to use PostgreSQL instead of MySQL for this project"
   "User chose React over Vue for the frontend"
   ```

3. **Key Information That May Be Lost**
   ```
   "User's timezone is UTC+8"
   "Project deadline: March 30, 2026"
   ```

### What to Let Accumulate

Don't use `cortex_add_memory` for:

- Regular conversation content (auto-stored in session)
- Contextual information (captured on close_session)
- Temporary preferences (not worth persisting)

### Role Parameter Usage

| Role | When to Use |
|------|-------------|
| `user` | User's statements, preferences, questions (default) |
| `assistant` | Your responses, explanations, code you wrote |
| `system` | Important context, rules, constraints |

## Search Strategies

### Query Formulation

```
┌────────────────────────────────────────────────────────────────┐
│                    Query Formulation Tips                      │
└────────────────────────────────────────────────────────────────┘

BAD:  "it"                          — Too vague
GOOD: "database choice"             — Specific topic

BAD:  "the user said something"     — Unfocused
GOOD: "user preference for testing" — Clear intent

BAD:  "code"                        — Too broad
GOOD: "authentication implementation" — Specific domain
```

### Score Threshold Guidelines

| Score | Use Case |
|-------|----------|
| 0.8+ | Need high-confidence matches only |
| 0.6 (default) | Balanced precision/recall |
| 0.4-0.5 | Exploratory search, finding related items |
| <0.4 | Usually too noisy, not recommended |

### Scope Parameter Usage

```
# Search across all sessions (default)
{ "query": "database decisions" }

# Search within specific session
{ "query": "preferences", "scope": "project-alpha" }
```

Use `scope` when:
- You know the relevant session ID
- Working within a specific project context
- Want to limit noise from other sessions

## Common Pitfalls

### 1. Memory Not Found After Close

**Symptom:** You closed a session but search returns nothing.

**Cause:** Memory extraction is asynchronous and may take 30-60 seconds.

**Solution:** Wait briefly after close_session before searching, or:
```
1. Close session
2. Continue with other work
3. Memory will be indexed automatically
```

### 2. Duplicate Memories

**Symptom:** Same information appears multiple times.

**Cause:** Both explicit `add_memory` and `close_session` extraction captured the same content.

**Solution:** Use `add_memory` only for information that:
- Won't naturally be captured in conversation
- Needs explicit emphasis
- Is a correction or override of previous information

### 3. Irrelevant Search Results

**Symptom:** Search returns unrelated content.

**Cause:** Query too vague or score threshold too low.

**Solution:**
- Make queries more specific
- Increase `min_score` threshold
- Use `scope` to limit search range

### 4. Lost Session Content

**Symptom:** Important conversation not in memory.

**Cause:** Session was never closed.

**Solution:** Establish a habit of closing sessions at checkpoints. If you realize too late, the raw messages may still exist in the session - close it now.

### 5. Configuration Issues

**Symptom:** Tools return errors about service/API.

**Cause:** LLM/Embedding credentials not configured.

**Solution:** See SKILL.md → Troubleshooting section.

## Workflow Examples

### Example 1: New Project Discussion

```
1. User starts discussing a new project
   → Just listen and respond naturally

2. User makes architecture decisions
   → Optionally: cortex_add_memory for explicit decisions

3. Discussion shifts to another topic
   → cortex_close_session (captures project discussion memories)

4. Continue with new topic
   → Fresh start, memories from step 3 are now searchable
```

### Example 2: Finding Previous Context

```
1. User asks: "What did we decide about auth?"

2. cortex_search({ query: "authentication decision" })

3. If results show snippets but need details:
   cortex_recall({ query: "authentication implementation details" })

4. Summarize findings to user
```

### Example 3: User States Preference

```
1. User: "I always want TypeScript strict mode"

2. cortex_add_memory({
     content: "User requires TypeScript strict mode in all projects",
     role: "user"
   })

3. Acknowledge and remember for future
```

## Memory Architecture Reference

### L0/L1/L2 Tier System

| Tier | Content | Size | Purpose | Search Role |
|------|---------|------|---------|-------------|
| L0 | Abstract summary | ~100 tokens | Quick filtering | First pass |
| L1 | Key points + context | ~2000 tokens | Context refinement | Second pass |
| L2 | Full original content | Complete | Exact matching | Final retrieval |

### How Search Works Internally

```
Query → L0 Filter → L1 Refine → L2 Retrieve → Ranked Results
          │             │             │
          ▼             ▼             ▼
      Quick scan    Contextual    Full content
      by summary    refinement    for precision
```

### Automatic Processes

| Process | Trigger | Duration |
|---------|---------|----------|
| Vector embedding | On `add_memory` | Seconds |
| L0/L1 generation | On `add_memory` (async) | Seconds |
| Full extraction | On `close_session` | 30-60s |
| Maintenance | Every 3 hours (auto) | Minutes |

## Summary Checklist

Before ending a conversation or topic transition, ask yourself:

- [ ] Have we accumulated meaningful content?
- [ ] Did the user share important preferences or decisions?
- [ ] Is this a natural checkpoint?
- [ ] Should I close the session now?

If yes to any, call `cortex_close_session`.
