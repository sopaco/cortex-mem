# Technical Documentation: Memory Core Domain

**Generation Time:** 2025-12-15 20:09:55 UTC  
**Document Version:** 1.0

---

## 1. Overview

The **Memory Core Domain** is the central business logic component of the `cortex-mem` system, responsible for managing the full lifecycle of AI agent memories. It orchestrates interactions between vector storage, LLM processing, and application-level memory semantics to enable persistent, context-aware intelligence in AI agents.

This domain provides advanced capabilities including semantic memory storage and retrieval, automatic metadata enrichment, intelligent deduplication, classification, importance scoring, and optimization. Built with a modular, dependency-injected architecture, it serves as the foundational engine powering all higher-level interfaces and services.

### Key Characteristics
- **Core Business Logic**: Implements the primary value proposition of persistent AI memory.
- **High Complexity (8.5/10)**: Integrates multiple AI and data engineering patterns.
- **Central Orchestration Role**: Coordinates vector store, LLM client, and auxiliary processors.
- **Configuration-Driven Behavior**: Supports flexible tuning via external configuration.

---

## 2. Architecture and Design Principles

### 2.1 Architectural Pattern

The Memory Core Domain follows a **Dependency Injection (DI) + Strategy Pattern** architecture:

```rust
pub struct MemoryManager {
    vector_store: Box<dyn VectorStore>,
    llm_client: Box<dyn LLMClient>,
    config: MemoryConfig,
    fact_extractor: Box<dyn FactExtractor>,
    memory_updater: Box<dyn MemoryUpdater>,
    // ... other components
}
```

At initialization, the `MemoryManager` accepts core dependencies and constructs specialized sub-components using cloned references. This enables:
- Clear separation of concerns
- Testability through mocking
- Runtime flexibility in behavior selection

### 2.2 Component Interaction Model

The domain uses well-defined traits (interfaces) to decouple functionality:

| Interface | Purpose |
|--------|-------|
| `VectorStore` | Abstracts vector database operations |
| `LLMClient` | Standardizes interaction with LLM APIs |
| `FactExtractor` | Encapsulates conversation-to-fact transformation |
| `MemoryClassifier` | Handles memory categorization logic |
| `DuplicateDetector` | Manages similarity detection and merging |

This abstraction allows pluggable implementations while maintaining consistent behavior across the system.

---

## 3. Core Components

### 3.1 Memory Manager (`manager.rs`)

#### Description
The central orchestrator that coordinates all memory operations. It acts as the single entry point for CRUD operations, search, and enhancement workflows.

#### Key Functions
| Function | Description |
|--------|-----------|
| `create_memory()` | Creates a new memory with embedding and metadata |
| `add_memory()` | Processes conversation messages into structured facts |
| `search()` | Performs semantic search with importance-weighted ranking |
| `get()` / `update()` / `delete()` | Standard CRUD operations |
| `enhance_memory()` | Enriches memory with keywords, summary, entities, etc. |

#### Lifecycle Management
- **ID Generation**: UUID v4
- **Content Hashing**: SHA-256 for duplicate detection
- **Timestamps**: `created_at`, `updated_at` in UTC
- **Validation**: Ensures non-empty content before persistence

#### Enhancement Pipeline
When `auto_enhance = true`, the manager applies an automated enrichment process:
1. **Keyword Extraction** via LLM
2. **Summarization** for long content (>200 chars)
3. **Classification** into types (Conversational, Procedural, Factual, etc.)
4. **Entity & Topic Extraction**
5. **Importance Scoring** (hybrid rule-based + LLM)
6. **Duplicate Detection & Merging**

> **Note**: The pipeline is conditional and configurable via `MemoryConfig`.

#### Search Ranking Algorithm
Combines two scores for result ordering:
```
final_score = (similarity_score × 0.7) + (importance_score × 0.3)
```
Supports filtering by:
- User/Agent ID
- Memory type
- Date ranges
- Custom metadata

---

### 3.2 Classification System (`classification.rs`)

#### Description
Automatically categorizes memories based on content semantics using hybrid strategies.

#### Implementation
Two primary implementations:
- **`LLMMemoryClassifier`**: Uses prompt-based LLM inference
- **`RuleBasedMemoryClassifier`**: Fast keyword-driven fallback

#### Classification Categories
| Type | Use Case |
|------|--------|
| `Conversational` | Dialogue history, chat logs |
| `Procedural` | Instructions, workflows, how-tos |
| `Factual` | Objective information, data points |
| `Semantic` | Concepts, definitions, knowledge |
| `Episodic` | Events, experiences, temporal data |
| `Personal` | Preferences, identity, personal info |

#### Entity and Topic Extraction
Uses dedicated prompts to extract:
- Named entities (people, organizations, locations)
- Thematic topics (technology, health, finance)

Provides fallback parsing for robustness when LLM extraction fails.

---

### 3.3 Importance Evaluator (`importance.rs`)

#### Description
Assesses the significance of a memory on a scale from 0.0 to 1.0.

#### Evaluation Criteria
Considers:
- Relevance to user identity/preferences
- Factual uniqueness and accuracy
- Potential future utility
- Emotional or actionable weight

#### Hybrid Strategy
- **LLM-Based**: For high-precision evaluation when `auto_enhance = true`
- **Rule-Based**: Faster alternative using heuristics like:
  - Content length
  - Memory type weighting
  - Keyword presence (e.g., "important", "remember")

Used in search ranking and optimization decisions.

---

### 3.4 Deduplication System (`deduplication.rs`)

#### Description
Detects and resolves redundant memories using multi-modal similarity analysis.

#### Similarity Metrics
Combines several signals:
| Metric | Method |
|-------|--------|
| **Semantic** | Cosine similarity of embeddings |
| **Lexical** | Jaccard overlap of word sets |
| **Metadata** | Overlap in entities, topics, user/agent IDs |

#### Threshold Configuration
- `similarity_threshold`: Minimum score to consider duplicates
- `merge_threshold`: Controls aggressiveness of merging

#### Merge Process
1. Identifies candidate memories via vector search
2. Validates similarity across multiple dimensions
3. Uses LLM to generate consolidated content
4. Updates target memory and deletes sources

Enables lossless consolidation of related information.

---

### 3.5 Fact Extractor (`extractor.rs`)

#### Description
Transforms raw conversation messages into structured, actionable facts.

#### Input Processing
Handles preprocessing steps:
- Language detection
- Code block removal
- Role-based message filtering

#### Extraction Strategies
Adaptive approach based on context:
| Strategy | Trigger Condition |
|--------|------------------|
| `DualChannel` | Balanced extraction from both roles |
| `UserOnly` | Focus on user preferences/information |
| `AssistantOnly` | Capture procedural knowledge |
| `ProceduralMemory` | Agent workflow continuity |

#### Output Format
Each extracted fact includes:
```rust
struct ExtractedFact {
    content: String,
    importance: f32,
    category: FactCategory,
    entities: Vec<String>,
    language: Option<LanguageInfo>,
    source_role: String,
}
```

Uses JSON-formatted LLM responses with strict schema enforcement.

---

### 3.6 Memory Updater (`updater.rs`)

#### Description
Determines optimal actions when integrating new facts with existing memories.

#### Decision Framework
For each incoming fact, evaluates one of four actions:
| Action | When Used |
|-------|----------|
| `CREATE` | Novel, unrelated information |
| `UPDATE` | Substantial addition to existing memory |
| `MERGE` | Related but redundant content |
| `IGNORE` | Redundant or trivial information |

Prioritizes `IGNORE > MERGE > UPDATE > CREATE` to minimize fragmentation.

#### Smart Update Workflow
1. Builds decision prompt with facts and top-k similar memories
2. Invokes LLM to determine action plan
3. Parses structured JSON response
4. Executes atomic operations via vector store

Prevents uncontrolled memory bloat through intelligent curation.

---

### 3.7 Optimization Engine

A comprehensive subsystem for improving memory quality over time.

#### Analyzer (`optimization_analyzer.rs`)
Identifies issues such as:
- Duplicates
- Low-quality entries
- Outdated information
- Space inefficiencies

Generates optimization plans based on strategy:
- `Full`: All issues
- `Incremental`: High-severity only
- `Deduplication`: Only merge candidates
- `Relevance`: Filter outdated memories

#### Optimizer (`optimizer.rs`)
Executes end-to-end optimization jobs:
1. **Detect Issues** (20% progress)
2. **Create Plan** (40%)
3. **Execute Actions** (80%)
4. **Report Results** (100%)

Supports dry-run mode for safe previewing.

#### Execution Engine (`execution_engine.rs`)
Performs actual operations in batches:
- Resilient error handling
- Retry mechanisms
- Resource throttling
- Progress tracking

Ensures reliability during large-scale optimizations.

#### Result Reporter (`result_reporter.rs`)
Generates detailed reports including:
- Summary statistics
- Action breakdown
- Performance metrics
- Quality improvements

Logs results via tracing framework and generates audit trails.

---

## 4. Integration Points

### 4.1 With Storage Layer
- **Interface**: `VectorStore` trait
- **Primary Adapter**: Qdrant integration
- **Operations**: Upsert, search, delete, list
- **Health Check**: Built-in connectivity validation

### 4.2 With AI Layer
- **Interface**: `LLMClient` trait
- **Capabilities Used**:
  - Embedding generation
  - Text completion
  - Structured extraction (keywords, entities, classifications)
- **Fallback Handling**: Graceful degradation on API failure

### 4.3 With Configuration System
All behaviors are controlled via `MemoryConfig`:
```toml
[core]
auto_enhance = true
auto_summary_threshold = 200
similarity_threshold = 0.85
merge_threshold = 0.9
```

Enables environment-specific tuning without code changes.

---

## 5. Usage Example

### Creating and Storing a Memory
```rust
let memory = memory_manager.create_memory(
    "I love hiking in the mountains".to_string(),
    MemoryMetadata::new("user_123", None),
).await?;

memory_manager.vector_store.insert(&memory).await?;
```

### Semantic Search with Filtering
```rust
let filters = Filters {
    user_id: Some("user_123".to_string()),
    memory_type: Some(MemoryType::Personal),
    ..Default::default()
};

let results = memory_manager.search("outdoor activities", &filters, 5).await?;
```

### Adding Conversation Memory
```rust
let messages = vec![
    Message { role: "user".to_string(), content: "I prefer Rust over Python".to_string() },
    Message { role: "assistant".to_string(), content: "That's great! Rust offers strong memory safety.".to_string() }
];

let results = memory_manager.add_memory(&messages, metadata).await?;
```

---

## 6. Best Practices and Recommendations

### 6.1 Configuration Tuning
- Set `similarity_threshold` between 0.8–0.9 for balanced deduplication
- Enable `auto_enhance` in production for richer metadata
- Adjust `auto_summary_threshold` based on average content length

### 6.2 Performance Considerations
- Batch operations where possible
- Use filters aggressively to reduce search scope
- Monitor LLM call volume due to cost implications

### 6.3 Data Integrity
- Always validate input content length
- Handle empty embeddings gracefully
- Implement retry logic for transient failures

### 6.4 Testing Strategy
- Use the Evaluation Framework for recall testing
- Validate classification accuracy with labeled datasets
- Benchmark search latency under load

---

## 7. Conclusion

The Memory Core Domain represents a sophisticated implementation of AI-native memory management, combining vector databases, LLM reasoning, and intelligent data curation. Its modular design enables extensibility while maintaining strong encapsulation of business logic.

By providing reliable, semantically aware memory storage and retrieval, this domain directly supports the core mission of enabling intelligent, context-preserving AI agents. Future enhancements could include real-time streaming updates, cross-agent memory sharing, and adaptive forgetting mechanisms.

This documentation covers all major aspects of the domain based on available source code and architectural research. For deeper exploration of specific modules, refer to inline code comments and unit tests within the respective `.rs` files.