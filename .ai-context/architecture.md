# Architecture

## System Overview

Cortex Memory implements a **hybrid storage architecture** combining:
1. **Virtual Filesystem** - Durable markdown storage with `cortex://` URI scheme
2. **Vector Index** - Semantic search via Qdrant

## Core Components

### 1. CortexMem (Main Runtime)

Location: `cortex-mem-core/src/lib.rs`

The central orchestrator that coordinates all components:

```rust
pub struct CortexMem {
    filesystem: Arc<dyn FilesystemOperations>,
    session_manager: Arc<RwLock<SessionManager>>,
    vector_store: Option<Arc<dyn VectorStore>>,
    embedding: Option<Arc<EmbeddingClient>>,
    llm_client: Option<Arc<dyn LLMClient>>,
    // ...
}
```

### 2. Virtual Filesystem

Location: `cortex-mem-core/src/filesystem/`

Maps `cortex://` URIs to physical files:

```
cortex://session/{id}/timeline/{date}/{time}.md
    → {data_dir}/session/{id}/timeline/{date}/{time}.md
```

Key types:
- `CortexUri` - Parsed URI representation
- `UriParser` - URI parsing logic
- `CortexFilesystem` - File operations implementation

### 3. Session Manager

Location: `cortex-mem-core/src/session/`

Manages conversation sessions:
- Create/close sessions
- Add messages to timeline
- Track session metadata

### 4. Memory Extraction Pipeline

Location: `cortex-mem-core/src/layers/`

Extracts structured memories from conversations:
1. **Extractor** - LLM-powered memory extraction
2. **Layer Generator** - Creates L0 (abstract) and L1 (overview) layers
3. **Incremental Updater** - Event-driven layer updates

### 5. Vector Search Engine

Location: `cortex-mem-core/src/search/`

Implements semantic search:
- Generates embeddings for content
- Stores vectors in Qdrant
- Performs similarity search with L0/L1/L2 weighted scoring

### 6. Event-Driven Automation

Location: `cortex-mem-core/src/automation/`

Background processing:
- File watchers for change detection
- Auto-indexing for vector sync
- Layer regeneration triggers

## Data Flow

### Message Ingestion

```
1. User/Agent Message
       │
       ▼
2. Session Manager stores to filesystem
       │
       ▼
3. Timeline file created (L2 content)
       │
       ▼
4. Session closed → Extraction triggered
       │
       ▼
5. LLM extracts structured memories
       │
       ▼
6. L0/L1 layers generated
       │
       ▼
7. Vectors indexed to Qdrant
```

### Search Query

```
1. Search Query
       │
       ▼
2. Generate query embedding
       │
       ▼
3. Vector search in Qdrant
       │
       ▼
4. Retrieve L0 abstracts first
       │
       ▼
5. Optionally fetch L1/L2 for relevant results
       │
       ▼
6. Return weighted, ranked results
```

## Three-Tier Memory Hierarchy

| Layer | File | Tokens | Purpose |
|-------|------|--------|---------|
| L0 (Abstract) | `.abstract.md` | ~100 | Quick relevance filtering |
| L1 (Overview) | `.overview.md` | ~2000 | Context understanding |
| L2 (Detail) | `{name}.md` | Full | Complete original content |

### Layer Resolution

For a file `cortex://session/abc/timeline/2024-03/15/10_30_00.md`:
- L0: `{dir}/.abstract.md` (directory-level abstract)
- L1: `{dir}/.overview.md` (directory-level overview)
- L2: The actual `.md` file

### Search Weights

```
Final Score = 0.2 × L0_score + 0.3 × L1_score + 0.5 × L2_score
```

## Multi-Tenancy

Tenants provide complete isolation:

```
{data_dir}/tenants/{tenant_id}/
├── session/
├── user/
├── agent/
└── resources/
```

Each tenant has:
- Separate filesystem namespace
- Separate Qdrant collection (`{collection_name}_{tenant_id}`)
- Independent vector index

## Event System

Location: `cortex-mem-core/src/memory_events.rs`

Events trigger automated processing:

```rust
pub enum MemoryEvent {
    FileCreated { uri: String },
    FileModified { uri: String },
    FileDeleted { uri: String },
    SessionClosed { thread_id: String },
    // ...
}
```

Event coordinators:
- `MemoryEventCoordinator` - Routes events to handlers
- `CascadeLayerUpdater` - Updates L0/L1 layers when content changes
- `IncrementalMemoryUpdater` - Incremental extraction updates

## Caching

Location: `cortex-mem-core/src/llm_result_cache.rs`

LRU cache for LLM results:
- Reduces redundant API calls by 50-75%
- TTL-based expiration
- Key-based deduplication

## Memory Cleanup

Location: `cortex-mem-core/src/memory_cleanup.rs`

Based on Ebbinghaus forgetting curve:
- Archives low-strength memories
- Controls storage growth
- Configurable retention policies
