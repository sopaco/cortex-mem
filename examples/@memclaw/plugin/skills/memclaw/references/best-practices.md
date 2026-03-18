# MemClaw Best Practices

## Token Optimization Strategy

### The Layer Selection Decision Tree

```
Start → What do you need?
           │
           ├── Quick relevance check?
           │   └── Use L0 (cortex_get_abstract or cortex_search with return_layers=["L0"])
           │
           ├── Understanding gist or context?
           │   └── Use L1 (cortex_get_overview or return_layers=["L0","L1"])
           │
           └── Exact details, quotes, or full implementation?
               └── Use L2 (cortex_get_content or return_layers=["L0","L1","L2"])
```

### Token Budget Guidelines

| Layer | Tokens | Use Case |
|-------|--------|----------|
| L0 | ~100 | Filtering, quick preview, relevance check |
| L1 | ~2000 | Understanding context, moderate detail |
| L2 | Full | Exact quotes, complete code, full conversation |

**Recommended Pattern:**
1. Start with L0 to filter candidates
2. Use L1 for promising matches
3. Use L2 only when absolutely necessary

### Example: Efficient Search Flow

```typescript
// Step 1: Quick search with L0 only
const results = cortex_search({
  query: "database schema design",
  return_layers: ["L0"],
  limit: 10
});

// Step 2: Identify top 2-3 relevant URIs
const topUris = results.results
  .filter(r => r.score > 0.7)
  .slice(0, 3)
  .map(r => r.uri);

// Step 3: Get L1 overview for top candidates
for (const uri of topUris) {
  const overview = cortex_get_overview({ uri });
  // Process overview...
}

// Step 4: If needed, get L2 for the most relevant
const fullContent = cortex_get_content({ uri: mostRelevantUri });
```

## Tool Selection Patterns

### Pattern 1: Discovery (Don't know where information is)

```
cortex_search(query="...", return_layers=["L0"])
    ↓
Identify relevant URIs
    ↓
cortex_get_overview(uri="...") for more context
    ↓
cortex_get_content(uri="...") if needed
```

### Pattern 2: Browsing (Know the structure)

```
cortex_ls(uri="cortex://session")
    ↓
cortex_ls(uri="cortex://session/{id}", include_abstracts=true)
    ↓
cortex_get_abstract(uri="...") for quick check
    ↓
cortex_get_content(uri="...") for details
```

### Pattern 3: Guided Exploration

```
cortex_explore(query="...", start_uri="...", return_layers=["L0"])
    ↓
Review exploration_path for relevance scores
    ↓
Use matches with higher return_layers if needed
```

## Session Management Best Practices

### When to Close Sessions

**DO close sessions:**
- ✅ After completing a significant task or topic
- ✅ After user shares important preferences/decisions
- ✅ When conversation topic shifts significantly
- ✅ Every 10-20 exchanges during long conversations

**DON'T close sessions:**
- ❌ After every message (too frequent)
- ❌ Only at the very end (user might forget)

### Memory Metadata

Use metadata to enrich stored memories:

```typescript
cortex_add_memory({
  content: "User prefers functional programming style over OOP",
  role: "assistant",
  metadata: {
    tags: ["preference", "programming-style"],
    importance: "high",
    category: "technical-preference"
  }
})
```

**Recommended metadata fields:**
- `tags`: Array of searchable tags
- `importance`: "high", "medium", "low"
- `category`: Custom categorization
- `source`: Where the information came from

## Common Workflows

### Finding User Preferences

```typescript
// 1. Search for preference-related content
const results = cortex_search({
  query: "user preferences settings configuration",
  return_layers: ["L0"],
  limit: 10
});

// 2. Get overviews for relevant results
for (const result of results.results.slice(0, 3)) {
  if (result.snippet.includes("preference")) {
    const overview = cortex_get_overview({ uri: result.uri });
    // Extract preferences from overview
  }
}
```

### Error Context Retrieval

```typescript
// Search for error-related content
const results = cortex_search({
  query: "TypeError: Cannot read property",
  return_layers: ["L0"]
});

// If not found, try broader semantic search
if (results.results.length === 0) {
  const broader = cortex_search({
    query: "property access error undefined object",
    return_layers: ["L0", "L1"]
  });
}
```

### Timeline Reconstruction

```typescript
// 1. List timeline directory
const timeline = cortex_ls({
  uri: "cortex://session/{session_id}/timeline",
  recursive: true,
  include_abstracts: true
});

// 2. Sort by modified date (entries already sorted)
// 3. Get full content for key moments
for (const entry of timeline.entries) {
  if (entry.abstract_text?.includes("important decision")) {
    const content = cortex_get_content({ uri: entry.uri });
    // Process important moment
  }
}
```

## Performance Tips

### Minimize API Calls

Instead of:
```typescript
// Bad: Multiple L2 calls
for (const uri of uris) {
  cortex_get_content({ uri });
}
```

Do:
```typescript
// Good: Batch with search
cortex_search({
  query: "specific topic",
  scope: session_id,
  return_layers: ["L0", "L1", "L2"]
});
```

### Use Scope Effectively

```typescript
// Faster: Search within known session
cortex_search({
  query: "authentication",
  scope: "project-x-session",
  return_layers: ["L0"]
});

// Slower: Search all sessions
cortex_search({
  query: "authentication",
  return_layers: ["L0"]
});
```

### Leverage Abstracts for Filtering

```typescript
// Get abstracts while browsing
const entries = cortex_ls({
  uri: "cortex://session",
  include_abstracts: true
});

// Filter based on abstracts without additional calls
const relevantEntries = entries.entries.filter(e => 
  e.abstract_text?.includes("relevant keyword")
);
```