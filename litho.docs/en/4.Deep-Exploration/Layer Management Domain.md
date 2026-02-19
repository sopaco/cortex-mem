 **Layer Management Domain Technical Documentation**

**Generation time:** 2024-01-15 08:30:45 UTC  
**Timestamp:** 1705312245

---

## 1. Overview

The **Layer Management Domain** implements the three-tier memory hierarchy specification (L0/L1/L2) within the Cortex-Mem system. This domain provides intelligent, context-aware memory retrieval by organizing conversation data into graduated levels of semantic abstraction—from high-level concepts to granular details.

Located within `cortex-mem-core`, this domain serves as a critical optimization layer for the semantic search pipeline, enabling efficient scanning of large memory corpora while minimizing LLM inference costs through intelligent caching strategies.

---

## 2. Architectural Concepts

### 2.1 Three-Tier Memory Hierarchy

The domain implements the **TARS/OpenViking memory organization specification**, partitioning memory into three distinct semantic layers:

| Layer | File Suffix | Content Type | Semantic Purpose | Search Weight |
|-------|-------------|--------------|------------------|---------------|
| **L0 (Abstract)** | `.abstract.md` | Concise natural language summary (50-100 words) | Coarse-grained semantic filtering; captures core intent and topics | 0.2 |
| **L1 (Overview)** | `.overview.md` | Structured markdown with key points, entities, and decisions | Context refinement; bridges abstract concepts with specific details | 0.3 |
| **L2 (Detail)** | `.md` (raw) | Complete conversation transcripts and raw messages | Precise semantic matching; full content retrieval | 0.5 |

### 2.2 Lazy Loading & Caching Strategy

The architecture employs a **lazy loading pattern** to optimize computational resources:

1. **Cache-First Retrieval**: When layer content is requested, the system first checks the filesystem for existing summaries
2. **On-Demand Generation**: If cached summaries (L0/L1) are absent, the system dynamically generates them from L2 detail content using LLM inference
3. **Persistent Caching**: Generated summaries are immediately written to the filesystem using deterministic URI-to-path mapping, ensuring subsequent requests bypass LLM calls
4. **Tenant Isolation**: All layer storage respects tenant boundaries via scoped directory structures (`cortex://session/{tenant_id}/...`)

This approach reduces LLM API costs by ensuring expensive summarization operations occur only once per memory unit, while maintaining search performance through pre-computed vector embeddings of all three layers.

---

## 3. Core Components

### 3.1 Layer Manager (`/cortex-mem-core/src/layers/manager.rs`)

The **Layer Manager** serves as the primary orchestration component, coordinating access across all three memory tiers. It exposes a unified interface for layer retrieval while abstracting the complexity of generation and caching.

**Key Responsibilities:**
- Routing layer requests to appropriate loaders (`load_abstract`, `load_overview`, `load_detail`)
- Managing cache coherence between filesystem and memory
- Coordinating batch generation operations for timeline indexing
- Enforcing naming conventions via `get_layer_uri()` helper functions

**Primary Interface:**
```rust
pub async fn load(&self, uri: &Uri, layer: LayerType) -> Result<String, LayerError>
pub async fn generate_batch(&self, uris: &[Uri]) -> Result<BatchResult, LayerError>
pub async fn generate_timeline_layers(&self, timeline_uri: &Uri) -> Result<(), LayerError>
```

### 3.2 Summary Generators (`/cortex-mem-core/src/layers/generator.rs`)

The generator subcomponents handle LLM-powered content transformation:

**Abstract Generator**
- Transforms L2 detail content into L0 high-level summaries
- Utilizes system prompts optimized for semantic density and intent extraction
- Implements fallback mechanisms for markdown extraction if structured generation fails

**Overview Generator**
- Produces L1 structured markdown containing key entities, decisions, and temporal markers
- Preserves chronological structure while compressing conversational noise
- Generates content suitable for both human review and vector embedding

---

## 4. Operational Workflows

### 4.1 On-Demand Layer Retrieval

When the Search Engine or AutoIndexer requests a specific layer:

1. **Existence Check**: The Layer Manager queries the `CortexFilesystem` to verify if the target layer file exists (e.g., `message.abstract.md`)
2. **Cache Hit**: If present, content is read directly from the filesystem and returned immediately
3. **Cache Miss Handling**:
   - Load source L2 detail content from filesystem
   - Invoke appropriate generator (Abstract or Overview) with LLM client
   - Generator constructs prompt with system context and L2 content
   - LLM generates summary (async operation)
   - Write generated content to filesystem using deterministic naming
   - Return content to caller

### 4.2 Batch Generation for Indexing

During the **Memory Indexing and Synchronization Process**, the AutoIndexer triggers batch generation:

```
AutomationManager → AutoIndexer → LayerManager.generate_batch()
```

This workflow:
- Processes multiple memory URIs in a single operation
- Generates L0/L1 summaries for entire conversation timelines
- Upserts all three layers (L0, L1, L2) into the Vector Storage domain as separate embeddings
- Enables weighted semantic search across all abstraction levels

### 4.3 Search Integration

The Search Engine Domain utilizes Layer Management during the **Semantic Memory Search Process**:

1. **L0 Search**: Query embedding matched against abstract summaries for coarse filtering
2. **L1 Search**: Contextual refinement using overview layer embeddings
3. **L2 Search**: Precise matching against full detail content
4. **Weighted Aggregation**: Results combined using scoring formula: `(0.2 × L0_score) + (0.3 × L1_score) + (0.5 × L2_score)`

If L0/L1 layers are missing during search, the Layer Manager dynamically generates them, though this introduces latency. Production deployments typically pre-generate layers via the AutoIndexer to ensure sub-second search performance.

---

## 5. Technical Implementation Details

### 5.1 Resource Addressing

The domain uses the `cortex://` URI scheme for location transparency:

- **Session Messages**: `cortex://session/{session_id}/timeline/{message_id}`
- **Layer Variants**: 
  - L2: `cortex://session/{id}/timeline/{msg}.md`
  - L1: `cortex://session/{id}/timeline/{msg}.overview.md`
  - L0: `cortex://session/{id}/timeline/{msg}.abstract.md`

### 5.2 Dependencies & Integration

**Required Services:**
- **`Arc<CortexFilesystem>`**: Async filesystem abstraction for tenant-scoped I/O operations
- **`Arc<dyn LLMClient>`**: OpenAI-compatible LLM client for summarization tasks
- **`Arc<EmbeddingClient>`**: (Indirect via Vector Storage) For embedding generated summaries

**Domain Relationships:**
- **Consumer**: Search Engine Domain (primary consumer of layer content)
- **Trigger**: Automation Management Domain (AutoIndexer triggers batch generation)
- **Infrastructure**: Core Infrastructure Domain (provides filesystem and LLM clients)

### 5.3 Concurrency & Performance

- **Async/Await Pattern**: All I/O operations use Rust async patterns with `tokio` runtime
- **Error Handling**: Comprehensive error propagation using `Result<T, LayerError>` types, with specific handling for LLM timeout and filesystem permission errors
- **Resource Efficiency**: LLM calls are the primary bottleneck; the caching strategy ensures 99%+ of search requests hit filesystem cache in steady-state operation
- **Tenant Safety**: All path resolutions include tenant ID segments, preventing cross-tenant data leakage

---

## 6. Configuration & Operational Considerations

### 6.1 Performance Optimization

- **Pre-generation**: Configure `AutoIndexer` to generate L0/L1 layers immediately after message creation to prevent search-time generation latency
- **Storage Overhead**: L0/L1 layers typically add 15-20% storage overhead compared to L2 raw content, but reduce search-time LLM costs to zero
- **Cache Invalidation**: Layer summaries are immutable once generated; updates to L2 source content trigger new summary generation via filesystem watcher events

### 6.2 LLM Cost Management

The Layer Management Domain significantly reduces operational costs by:
- Eliminating redundant summarization through filesystem persistence
- Supporting local LLM deployments for layer generation (via configurable `LLMClient` endpoints)
- Enabling batch processing to maximize token efficiency

---

## 7. Usage Scenarios

### Scenario A: Semantic Search with Degraded Thresholds
When L0 returns insufficient results, the Search Engine applies degradation strategies (threshold reduction from 0.5 → 0.4 → 0.3). The Layer Manager ensures L1/L2 content is available for deeper semantic matching without requiring additional LLM calls.

### Scenario B: Multi-Tenant Session Analysis
For SaaS deployments, the Layer Manager respects tenant isolation:
```
Tenant A: cortex://session/tenant-a/uuid/timeline/msg.abstract.md
Tenant B: cortex://session/tenant-b/uuid/timeline/msg.abstract.md
```
Each tenant's layers are generated and stored in isolated paths, with Vector Storage using tenant-suffixed collections (`cortex-mem-{tenant_id}`).

---

## 8. Summary

The Layer Management Domain provides the foundational infrastructure for context-aware memory retrieval in Cortex-Mem. By implementing a three-tier semantic hierarchy with intelligent caching, it balances the computational cost of LLM inference against the performance requirements of real-time semantic search. The domain's lazy loading architecture ensures efficient resource utilization while maintaining data consistency across filesystem and vector database representations.

**Key Files:**
- `/cortex-mem-core/src/layers/manager.rs` - Layer orchestration and caching logic
- `/cortex-mem-core/src/layers/generator.rs` - LLM-based summary generation
- Associated integration points in `/cortex-mem-core/src/automation/indexer.rs` (AutoIndexer)