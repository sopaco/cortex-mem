 **Cortex-Mem Web UI Domain Technical Documentation**

**Version:** 1.0  
**Generation Time:** 2026-02-19 04:09:40 (UTC)  
**Domain:** Web UI (cortex-mem-insights)  
**Technology Stack:** Svelte 5, TypeScript, Vanilla History API  
**Architecture Pattern:** Component-Driven Reactive Architecture  

---

## 1. Executive Overview

The **Web UI Domain** (`cortex-mem-insights`) serves as the primary visual interface for the Cortex-Mem system, enabling System Administrators and developers to visualize, browse, and manage multi-dimensional memory data across tenants. Built on **Svelte 5** with a modern reactive architecture, the domain implements client-side routing, centralized state management, and seamless HTTP API integration to provide real-time insights into memory utilization, session data, and semantic search capabilities.

The domain adheres to a strict separation of concerns: UI components handle rendering and user interactions, reactive stores manage application state and asynchronous logic, and a dedicated API client manages HTTP communication with the backend service layer.

---

## 2. Architectural Design

### 2.1 High-Level Architecture

The Web UI Domain follows a **layered component architecture** with unidirectional data flow:

```
┌─────────────────────────────────────────────────────────────┐
│                    Presentation Layer                        │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐        │
│  │  Dashboard   │ │   Memories   │ │    Search    │        │
│  │   .svelte    │ │   .svelte    │ │   .svelte    │        │
│  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘        │
│         └─────────────────┼─────────────────┘                │
│                           │                                  │
│              ┌────────────▼────────────┐                   │
│              │   TenantSelector.svelte  │                   │
│              └────────────┬────────────┘                   │
└───────────────────────────┼─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    State Management Layer                    │
│              ┌────────────────────────┐                     │
│              │   tenant.ts (Store)    │                     │
│              │  - currentTenant       │                     │
│              │  - tenants[]           │                     │
│              │  - tenantInfo          │                     │
│              └───────────┬────────────┘                     │
└───────────────────────────┼─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    Service Layer                             │
│              ┌────────────────────────┐                     │
│              │      apiClient         │                     │
│              │  (HTTP/REST Wrapper)   │                     │
│              └───────────┬────────────┘                     │
└───────────────────────────┼─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    Backend Interface                         │
│              cortex-mem-service (HTTP API)                   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Core Design Principles

1. **Reactive State Management**: Utilizes Svelte 5 runes (`$state`, `$effect`) for fine-grained reactivity, eliminating traditional store subscription boilerplate while maintaining optimal rendering performance.

2. **Client-Side Routing**: Implements lightweight routing using the vanilla History API (`window.history.pushState`) without external dependencies, reducing bundle size and complexity.

3. **Tenant-Centric Design**: All operations are scoped to a selected tenant, with the `TenantStore` serving as the single source of truth for isolation boundaries.

4. **Filesystem Abstraction**: Presents memory data through a virtual file system interface using the `cortex://` URI scheme, enabling intuitive navigation of session, user, and agent memory dimensions.

---

## 3. Module Structure & Components

### 3.1 Core Application Shell (`App.svelte`)

The root component orchestrates application-level concerns:

- **Route Management**: Maintains `currentPath` state and conditionally renders page components based on URL path (`/`, `/memories`, `/search`)
- **Layout Composition**: Provides consistent navigation shell and tenant context
- **Global Initialization**: Triggers tenant store initialization on mount

**Key Implementation Detail:**
```typescript
// Routing logic using Svelte 5 runes
let currentPath = $state(window.location.pathname);

function navigate(path: string) {
    window.history.pushState({}, '', path);
    currentPath = path;
}
```

### 3.2 State Management (`lib/stores/tenant.ts`)

The **Tenant Store** implements a centralized state management pattern using Svelte writable and derived stores:

**State Variables:**
- `currentTenant`: Writable store holding the active tenant ID
- `tenants`: Array of available tenant identifiers
- `tenantInfo`: Map containing metadata and statistics for each tenant
- `tenantLoading`: Boolean indicating async operation status

**Key Operations:**
- `initTenants()`: Fetches available tenant list from `/api/v2/tenants`
- `switchTenant(tenantId: string)`: Updates current tenant, triggers context switch via API, and notifies subscribers
- `loadTenantInfo(tenantId: string)`: Fetches aggregated metrics for dashboard visualization

**Implementation Pattern:**
The store exports reactive references that components consume via Svelte's `$` prefix syntax, enabling automatic UI updates on state mutations.

### 3.3 Page Components

#### 3.3.1 Dashboard (`Dashboard.svelte`)

Aggregates system-wide metrics and tenant health indicators:

- **Data Aggregation**: Iterates through all tenants, calling `loadTenantInfo()` to collect storage metrics
- **Caching Strategy**: Stores results in a local Map to minimize API calls during navigation
- **Visualization**: Displays memory usage statistics, file counts across dimensions (user, agent, session), and system health status

#### 3.3.2 Memory Browser (`Memories.svelte`)

Implements a virtual file system explorer for memory data:

- **Directory Navigation**: Supports traversal of `cortex://` URIs with breadcrumb navigation
- **File Operations**: 
  - `listDirectory()`: Fetches directory entries from `/api/v2/filesystem/list`
  - `readFile()`: Retrieves file content for preview
  - `writeFile()`: Persists edited content back to storage
- **Content Rendering**: Custom markdown renderer for memory file preview
- **State Synchronization**: Reactive `$effect` blocks reload data when `currentTenant` changes

#### 3.3.3 Search Interface (`Search.svelte`)

Provides semantic search capabilities across memory layers:

- **Search Parameters**: Keyword input, scope filtering (user/agent/session), result limit configuration
- **Degraded Experience**: Gracefully handles vector search unavailability with fallback messaging
- **Result Presentation**: Paginated result cards displaying relevance scores, source URIs, and content snippets
- **Real-time Updates**: Clears results and resets state on tenant switches to prevent cross-tenant data leakage

### 3.4 Reusable Components (`lib/components/`)

**TenantSelector (`TenantSelector.svelte`):**
- Dropdown interface for tenant selection
- Subscribes to `currentTenant` and `tenants` stores
- Emits `switchTenant` events on user selection
- Visual indicator for active tenant context

---

## 4. API Integration Layer (`lib/api.ts`)

The **API Client** abstracts HTTP communication with the Cortex-Mem service layer:

**Core Capabilities:**
- **Tenant Context Propagation**: Automatically includes tenant ID in request headers or query parameters
- **Error Handling**: Standardized error format parsing and user-friendly message generation
- **Type Safety**: TypeScript interfaces mirroring backend DTOs (Data Transfer Objects)

**Key Endpoints Consumed:**
- `GET /api/v2/tenants/stats` - Dashboard metrics aggregation
- `GET /api/v2/directory/{uri}` - Filesystem listing operations
- `GET /api/v2/files/{uri}` - File content retrieval
- `POST /api/v2/files/{uri}` - File write operations
- `GET /api/v2/search` - Semantic memory search with query parameters (`query`, `scope`, `limit`, `threshold`)

**Implementation Pattern:**
Async/await patterns with loading state management, ensuring UI responsiveness during network operations.

---

## 5. Multi-Tenancy Implementation

The Web UI enforces **strict tenant isolation** at the presentation layer:

### 5.1 Tenant Context Flow

1. **Initialization**: On application startup, `initTenants()` populates available tenants; first tenant becomes default
2. **Selection**: User selects tenant via `TenantSelector`, triggering `switchTenant()`
3. **Propagation**: API client updates request context with new tenant ID
4. **Refresh**: All active page components react to `currentTenant` changes via `$effect` blocks, reloading data with new context
5. **Isolation**: Search results and file listings are automatically scoped to selected tenant, preventing cross-tenant data exposure

### 5.2 State Isolation

- **Search State**: Cleared immediately on tenant switch to prevent stale results
- **File Browser**: Resets to root directory (`cortex://`) of new tenant
- **Dashboard**: Refetches aggregated metrics for selected tenant scope

---

## 6. Technical Implementation Details

### 6.1 Reactive State with Svelte 5 Runes

The domain leverages Svelte 5's rune system for state management:

```typescript
// Local component state
let entries = $state<FileEntry[]>([]);
let loading = $state(false);

// Reactive side effects
$effect(() => {
    // Automatically re-run when currentTenant changes
    if ($currentTenant) {
        loadDirectory(currentPath);
    }
});
```

### 6.2 Styling & Theming

- **CSS Custom Properties**: System-wide theming using CSS variables for colors, spacing, and typography
- **Responsive Layouts**: Flexbox and Grid layouts adapting to viewport changes
- **Component Scoping**: Svelte's scoped CSS preventing style leakage between components

### 6.3 Error Handling Strategy

- **Global Error Boundaries**: Catch unhandled promise rejections in API calls
- **User Feedback**: Toast notifications or inline error states for operation failures
- **Graceful Degradation**: Search component continues functioning (in limited capacity) if vector backend is unavailable

---

## 7. Integration Points

### 7.1 Backend Dependencies

The Web UI Domain depends on the **Application Interface Domain** (`cortex-mem-service`) for:

- **Data Retrieval**: RESTful endpoints for memory CRUD operations
- **Authentication**: Tenant identification via HTTP headers or context propagation
- **Real-time Data**: Polling-based updates for filesystem changes (future enhancement: WebSocket integration)

### 7.2 Domain Relationships

| Related Domain | Relationship Type | Data Flow |
|---------------|------------------|-----------|
| Application Interface Domain | Service Call | HTTP Requests to REST API |
| Configuration Management Domain | Data Dependency | Tenant ID resolution |
| Core Infrastructure Domain | Data Dependency | Type definitions for Memory, Dimension, Filters |

---

## 8. Deployment & Runtime Considerations

### 8.1 Static Hosting

The Web UI is compiled to static assets (HTML, CSS, JS) suitable for deployment on:
- Static site hosting (Netlify, Vercel, GitHub Pages)
- Embedded in `cortex-mem-service` binary (served via Axum static file handler)
- CDN distribution for edge caching

### 8.2 Configuration

Runtime configuration via:
- **Environment Variables**: API endpoint URLs (`VITE_API_BASE_URL`)
- **Build-time Constants**: Feature flags for experimental UI components

---

## 9. Best Practices & Guidelines

### 9.1 Component Development
- **Props Interface**: Explicit TypeScript interfaces for component props ensuring type safety
- **Event Dispatching**: Standardized CustomEvent patterns for parent-child communication
- **Cleanup**: Proper cleanup of `$effect` subscriptions and event listeners to prevent memory leaks

### 9.2 State Management
- **Store is Source of Truth**: Always read tenant state from store, never cache locally without synchronization
- **Async Action Pattern**: Use `tenantLoading` state to prevent concurrent operations
- **Optimistic Updates**: Optional optimistic UI updates for file operations with rollback on error

---

## 10. Summary

The Web UI Domain provides a sophisticated yet lightweight interface for the Cortex-Mem system, leveraging modern Svelte 5 capabilities to deliver a reactive, tenant-isolated user experience. Its architecture emphasizes **separation of concerns**, **type safety**, and **responsive design**, making it suitable for both development debugging and production monitoring scenarios.

The domain's tight integration with the backend HTTP API, combined with its robust tenant management capabilities, ensures that administrators can effectively visualize and manage multi-dimensional memory data across isolated organizational boundaries.