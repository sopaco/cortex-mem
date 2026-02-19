 **Technical Documentation: Core Infrastructure Domain**

**Document Version:** 1.0  
**System:** Cortex-Mem  
**Domain Classification:** Core Business Domain  
**Criticality:** High (Foundation for all memory operations)

---

## 1. Domain Overview

The **Core Infrastructure Domain** serves as the foundational service layer of the Cortex-Mem system, providing essential capabilities for memory persistence, semantic processing, and data abstraction. As the central dependency hub for all other domains, it encapsulates the core business logic required to bridge raw storage mechanisms with high-level memory operations.

**Primary Responsibilities:**
- Abstracting filesystem operations through a unified URI scheme (`cortex://`) with tenant isolation
- Managing external AI service integrations (LLM completions, embeddings)
- Providing vector storage and semantic search capabilities via Qdrant
- Orchestrating asynchronous event propagation for decoupled automation workflows
- Defining the canonical type system for memory entities, filters, and extraction results

This domain implements the **Strategy Pattern** for pluggable backends and maintains **strict backward compatibility** (V1) while supporting multi-tenant SaaS deployments.

---

## 2. Architectural Position

Within the Cortex-Mem workspace architecture, the Core Infrastructure Domain occupies the **Domain Layer**, serving as the primary dependency for:

- **Application Interface Domain** (CLI, HTTP API, MCP Server)
- **Automation Management Domain** (AutoIndexer, AutoExtractor)
- **Search Engine Domain** (Vector Search operations)
- **Layer Management Domain** (Summary generation)

```
Dependency Flow:
Application Interfaces → Core Infrastructure → External Systems (LLM, Qdrant, OS Filesystem)
```

**Design Philosophy:**
- **Async-First**: All I/O operations use Rust's async/await with Tokio runtime
- **Trait-Based Abstraction**: `LLMClient`, `VectorStore`, and `CortexFilesystem` traits enable testability and backend substitution
- **Location Transparency**: The `cortex://` URI scheme provides logical addressing independent of physical storage location
- **Tenant Isolation**: All components support tenant-scoped operations via collection suffixing and directory namespacing

---

## 3. Component Reference

### 3.1 Filesystem Abstraction Layer

**Location:** `cortex-mem-core/src/filesystem/`  
**Key Files:** `uri.rs`, `operations.rs`

Provides an asynchronous, trait-based virtual filesystem implementing the OpenViking/TARS memory organization specification.

**Capabilities:**
- **URI Scheme**: Hierarchical addressing (`cortex://{dimension}/{category}/{resource}`)
- **Tenant Isolation**: Automatic path mapping to `/data/tenants/{tenant_id}/`
- **Async CRUD**: Non-blocking read/write operations via `tokio::fs`
- **Markdown Persistence**: Native support for `.md` file formats with metadata headers

**Key Interfaces:**
```rust
// Async trait for filesystem operations
#[async_trait]
pub trait CortexFilesystem: Send + Sync {
    async fn read(&self, uri: &str) -> Result<String>;
    async fn write(&self, uri: &str, content: &str) -> Result<()>;
    async fn list(&self, uri: &str) -> Result<Vec<Metadata>>;
    // ...
}
```

### 3.2 LLM Client Services

**Location:** `cortex-mem-core/src/llm/`  
**Key Files:** `client.rs`, `prompts.rs`, `extractor_types.rs`

Wraps OpenAI-compatible API providers with robust error handling and structured output parsing.

**Capabilities:**
- **Completion Interface**: Standard chat completions with configurable models
- **Structured Extraction**: JSON-mode fact/decision/entity extraction with schema validation
- **Fallback Parsing**: Multi-layered parsing strategy (code block detection, object boundary scanning) for unreliable LLM outputs
- **Prompt Management**: Template-based prompt generation for memory abstraction (L0/L1) and extraction

**Resilience Features:**
- JSON repair for malformed responses
- Confidence scoring for extracted memories
- Timeout and retry logic for external API calls

### 3.3 Embedding Generation Service

**Location:** `cortex-mem-core/src/embedding/client.rs`

Manages vectorization of text content for semantic search indexing.

**Capabilities:**
- **Batch Processing**: Efficient embedding of multiple documents in single API calls
- **Chunked Processing**: Handles large texts exceeding token limits via intelligent chunking
- **Dimension Management**: Validates embedding dimensions against vector store requirements
- **Provider Agnostic**: OpenAI-compatible API support (OpenAI, Azure, local LLMs)

**Performance Optimizations:**
- Request batching to minimize API round-trips
- Async concurrency for parallel embedding generation
- Vector caching to avoid redundant processing

### 3.4 Vector Storage Engine

**Location:** `cortex-mem-core/src/vector_store/`  
**Key Files:** `qdrant.rs`, `mod.rs`

Implements semantic search capabilities using Qdrant vector database with tenant-aware isolation.

**Capabilities:**
- **Similarity Search**: Cosine similarity search with metadata filtering
- **Tenant Isolation**: Dynamic collection naming (`cortex-mem-{tenant_id}`)
- **Layered Storage**: Supports L0 (abstract), L1 (overview), and L2 (detail) vector indices
- **Metadata Filtering**: Translation of domain `Filters` to Qdrant `Condition` system
- **Deterministic IDs**: URI-to-vector-ID mapping (`uri_to_vector_id`) for idempotent updates

**Key Operations:**
- `upsert`: Insert or update memory vectors with payload metadata
- `search`: Hybrid vector + metadata queries with scoring
- `scroll`: Pagination for large result sets
- `delete`: Removal by filter or ID

### 3.5 Event Bus System

**Location:** `cortex-mem-core/src/events.rs`

Provides asynchronous, decoupled communication between system components using Tokio MPSC channels.

**Event Types:**
- `FilesystemEvent`: File creation, modification, deletion (triggers AutoIndexer)
- `SessionEvent`: Session lifecycle (creation, closure triggers AutoExtractor)
- `SystemEvent`: Configuration changes, health status

**Architecture Pattern:**
- **Publisher-Subscriber**: Components emit events without knowledge of consumers
- **Backpressure Handling**: Bounded channels with graceful degradation
- **Type Safety**: Strongly typed event payloads prevent serialization errors

### 3.6 Data Models & Type System

**Location:** `cortex-mem-core/src/types.rs`, `src/extraction/types.rs`

Defines the canonical domain models ensuring type safety across the system boundary.

**Core Types:**
- `Memory`: Primary entity with content, metadata, timestamp, and URI
- `Dimension`: Enum for memory scopes (User, Agent, Session, Resource)
- `Filters`: Query constraints (time range, entities, tenant_id)
- `ScoredMemory`: Search result with relevance score and distance metric

**Extraction Types:**
- `ExtractedFact`: Structured knowledge with confidence and source
- `ExtractedDecision`: Agent decisions with context and rationale
- `ExtractedEntity`: Named entities with type classification
- `UserProfile`/`AgentProfile`: Persistent characterizations with merge semantics

**Serialization:**
- Serde-based JSON serialization for persistence
- Backward compatibility mappings (e.g., legacy "agents" string → `Dimension::Agent`)

---

## 4. Implementation Details

### 4.1 Concurrency Model
The domain employs **Tokio** for asynchronous runtime management:
- **IO Bound**: Filesystem and HTTP operations use `async_trait` for trait object safety
- **CPU Bound**: JSON parsing and similarity calculations use `tokio::task::spawn_blocking` when necessary
- **Resource Pools**: HTTP clients (reqwest) maintain connection pooling for LLM and Qdrant endpoints

### 4.2 Error Handling Strategy
Centralized error management using **`thiserror`**:
- Domain-specific `Error` enum converting low-level errors (IO, Serde, Qdrant client)
- Structured error propagation preserving context across async boundaries
- Graceful degradation for non-critical failures (e.g., individual file processing errors don't halt batch operations)

### 4.3 URI Parsing & Validation
The `UriParser` implements strict validation for the `cortex://` scheme:
```rust
cortex://[tenant/]session/{id}/timeline/{timestamp}.md
cortex://[tenant/]user/{id}/profile/facts.json
cortex://[tenant/]agent/{id}/decisions.md
```
- **Hierarchical**: Mirrors filesystem structure for intuitive navigation
- **Tenant Optional**: Supports both single-tenant and multi-tenant deployments
- **Deterministic**: Provides consistent mapping to vector IDs and file paths

### 4.4 Multi-Tenancy Implementation
Tenant isolation is enforced at the infrastructure boundary:
1. **Filesystem**: Path prefixing (`/data/tenants/{id}/`)
2. **Qdrant**: Collection suffixing ensures separate vector spaces
3. **Configuration**: Per-tenant overrides for LLM models and embedding dimensions

---

## 5. Integration Patterns

### 5.1 Dependency Injection
The `MemoryOperations` facade (composed of Core Infrastructure services) is constructed via the **Builder Pattern**:
1. Configuration loads LLM/Qdrant credentials
2. Clients are initialized with connection pooling
3. Services are wired into a unified interface
4. Injected into CLI commands, HTTP handlers, or MCP service

### 5.2 Event-Driven Workflows
Components interact through the Event Bus to maintain loose coupling:
```
FilesystemWatcher → FilesystemEvent → AutomationManager → AutoIndexer → VectorStorage
SessionManager → SessionEvent::Closed → AutomationManager → AutoExtractor → ProfileManager
```

### 5.3 Data Flow: Memory Ingestion
1. **Write**: Client writes markdown to `cortex://` URI via Filesystem Abstraction
2. **Notify**: Event Bus publishes `FilesystemEvent::FileCreated`
3. **Extract**: AutoIndexer reads content, generates L0/L1 via LLM Client
4. **Vectorize**: Embedding Client creates vectors for L0/L1/L2
5. **Store**: Vector Storage upserts to Qdrant with tenant-aware collection naming

### 5.4 Data Flow: Semantic Search
1. **Embed**: Search Engine sends query to Embedding Client
2. **Layered Retrieval**: Vector Storage queries L0 (coarse) → L1 (context) → L2 (precise)
3. **Scoring**: Weighted combination (0.2×L0 + 0.3×L1 + 0.5×L2)
4. **Hydrate**: Filesystem Abstraction retrieves full content for top results
5. **Return**: Ranked `ScoredMemory` objects with source URIs

---

## 6. Configuration & Deployment

### 6.1 Environment Requirements
- **LLM Endpoint**: OpenAI-compatible API (OpenAI, Azure, local)
- **Vector Database**: Qdrant instance (local or cluster) accessible via HTTP/gRPC
- **Data Directory**: Writeable filesystem path for markdown storage

### 6.2 Key Configuration Parameters
```toml
[llm]
model = "gpt-4"
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"

[embedding]
model = "text-embedding-3-small"
dimensions = 1536

[qdrant]
url = "http://localhost:6334"
collection_name = "cortex-mem"
```

### 6.3 Scalability Considerations
- **Horizontal**: Stateless design allows multiple service instances behind load balancer (requires shared Qdrant and filesystem)
- **Vertical**: Async architecture efficiently utilizes CPU cores for concurrent request processing
- **Storage**: Vector Storage supports Qdrant clustering for large-scale deployments

### 6.4 Monitoring & Observability
- **Health Checks**: Vector Storage provides connectivity status to Qdrant
- **Event Metrics**: Event Bus tracks queue depths and processing latencies
- **Error Tracking**: Structured logging of LLM API failures and filesystem errors

---

## 7. Summary

The Core Infrastructure Domain provides the **foundational substrate** for all Cortex-Mem operations, abstracting the complexity of distributed storage, AI service integration, and concurrent processing. Through its trait-based design and event-driven architecture, it enables the higher-level domains (Automation, Search, Session Management) to operate with clean, testable interfaces while maintaining enterprise-grade capabilities for multi-tenant isolation and semantic retrieval.

**Key Success Factors:**
- Strict adherence to the `cortex://` URI contract ensures data portability
- Async-first implementation maximizes throughput for I/O-bound operations
- Trait abstractions enable testing without external dependencies (LLM/Qdrant mocking)
- Event-driven decoupling allows independent scaling of ingestion and processing pipelines