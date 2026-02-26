# Cortex Memory Insights - Observability Dashboard

`cortex-mem-insights` is a web-based dashboard built with Svelte 5 that provides visual monitoring and management of the Cortex Memory system.

## 🌟 Features

### 📊 Dashboard
- **Tenants Overview**: View all tenants with statistics (user memories, sessions, agents, resources, files, storage)
- **Service Status**: Monitor health status, version, and LLM availability
- **Storage Card**: Display total size and file counts by category

### 💾 Memory Browser
- **File Browser**: Two-panel layout with file tree and content preview
- **Directory Navigation**: Tabs for User/Session/Agent/Resources roots
- **File Operations**: View, edit, and save memory files
- **Markdown Rendering**: Basic markdown-to-HTML conversion for content preview

### 🔍 Vector Search
- **Search Form**: Keyword input with scope selector and limit control
- **Scopes**: All, User, Session, Agent
- **Results Display**: Cards showing URI, relevance score, snippet, and expandable full content

### 📱 Responsive Design
- **Desktop Optimized**: Optimized desktop experience with wide screen support
- **Mobile Friendly**: Responsive layout for tablet and mobile devices

## 🚀 Quick Start

### Prerequisites

- Node.js 18+ or Bun
- Modern browser (Chrome 90+, Firefox 88+, Safari 14+)
- Cortex Memory Service running on port 8085

### Installation

```bash
# Navigate to directory
cd cortex-mem/cortex-mem-insights

# Install dependencies
bun install  # or npm install

# Start development server
bun run dev  # or npm run dev

# Build for production
bun run build  # or npm run build

# Preview production build
bun run preview  # or npm run preview
```

Access the dashboard at http://localhost:8082

### Standalone Server

The project includes a standalone Bun server that can be compiled to a native executable:

```bash
# Run the server
bun run serve

# Compile to standalone executable
bun run compile

# Cross-compile for all platforms
bun run compile:all
```

## 📖 Usage Guide

### Dashboard Overview

The main dashboard provides:
- **Tenants Table**: Display all tenants with statistics
- **Service Status Card**: Health status, version, LLM availability
- **Storage Card**: Total size and file counts

### Browsing Memories

1. Click "Memories" in the navigation
2. Select a dimension tab (User, Session, Agent, Resources)
3. Browse the file tree structure
4. Click on files to view content
5. Edit content directly in the editor
6. Save changes with the Save button

### Searching

1. Navigate to "Search" page
2. Enter search query
3. Select scope (All, User, Session, Agent)
4. Set result limit
5. View results with relevance scores
6. Expand to see full content

## 🛠️ Architecture

### Project Structure

```
src/
├── lib/
│   ├── api.ts              # API client for cortex-mem-service
│   ├── types.ts            # TypeScript interfaces
│   ├── components/
│   │   └── TenantSelector.svelte  # Tenant selection dropdown
│   ├── pages/
│   │   ├── Dashboard.svelte       # Main dashboard
│   │   ├── Memories.svelte        # Memory browser
│   │   └── Search.svelte          # Search interface
│   └── stores/
│       └── tenant.ts              # Global tenant state
├── app.css                 # Global styles
├── App.svelte              # Main router
└── main.ts                 # App bootstrap
```

### Frontend Tech Stack

- **Svelte 5**: Reactive UI framework with runes API (`$state`, `$derived`, `$effect`)
- **Vite**: Fast build tool and development server
- **TypeScript**: Type-safe JavaScript
- **Bun**: Runtime and package manager

### Server (server.ts)

A standalone Bun server with:

**Command-line Options:**
```
-p, --port <number>       Port to run server on (default: 8159)
--api-target <url>        API target URL (default: http://localhost:8085)
--no-browser              Don't open browser automatically
--headless                Same as --no-browser
-h, --help                Show help message
```

**Environment Variables:**
- `PORT`: Server port
- `API_TARGET`: Backend API URL for proxying

## 📡 API Reference

### API Client Methods

| Method | Endpoint | Description |
|--------|----------|-------------|
| `getHealth()` | `GET /health` | Health check |
| `listTenants()` | `GET /api/v2/tenants/tenants` | List all tenants |
| `switchTenant(tenantId)` | `POST /api/v2/tenants/tenants/switch` | Switch active tenant |
| `listDirectory(path)` | `GET /api/v2/filesystem/list?uri=` | List directory contents |
| `readFile(path)` | `GET /api/v2/filesystem/read/` | Read file content |
| `writeFile(path, content)` | `POST /api/v2/filesystem/write` | Write file content |
| `getDirectoryStats(uri)` | `GET /api/v2/filesystem/stats?uri=` | Get directory statistics |
| `getSessions()` | `GET /api/v2/sessions` | Get session list |
| `search(keyword, scope, limit)` | `POST /api/v2/search` | Vector search |

### Type Definitions

```typescript
interface HealthStatus {
  status: string;
  service: string;
  version: string;
  llm_available: boolean;
}

interface FileEntryResponse {
  uri: string;
  name: string;
  is_directory: boolean;
  size: number;
  modified: string;
}

interface SearchResult {
  uri: string;
  score: number;
  snippet: string;
  content?: string;
  source: string;
}

interface TenantInfo {
  user_memories: number;
  sessions: number;
  agents: number;
  resources: number;
  files: number;
  storage_bytes: number;
}
```

## 🔧 Configuration

### Vite Configuration (vite.config.ts)

```typescript
export default defineConfig({
  server: {
    port: 8082,
    proxy: {
      '/api/v2': 'http://localhost:8085',
      '/health': 'http://localhost:8085',
    },
  },
});
```

### Environment Variables

Create `.env` file:

```bash
# API server address (optional, uses proxy by default)
VITE_API_BASE_URL=http://localhost:8085/api/v2
```

### Custom Styles

Edit `src/app.css` to modify theme variables:

```css
:root {
  --primary-color: #6366f1;
  --secondary-color: #22d3ee;
  --background-color: #0f172a;
  --text-color: #e2e8f0;
  --border-color: #334155;
}
```

## 🧪 Development

### Adding New Pages

1. Create component in `src/lib/pages/`
2. Add route in `src/App.svelte`
3. Add navigation link

### State Management

Use Svelte 5 runes:

```typescript
// Reactive state
let memories = $state<FileEntry[]>([]);

// Derived state
const filteredMemories = $derived(
  memories.filter(m => m.name.includes(searchQuery))
);

// Side effects
$effect(() => {
  console.log('Memories updated:', memories.length);
});
```

## 🚨 Troubleshooting

### Connection Failed

**Problem**: Cannot connect to Cortex Memory Service

**Solution**:
1. Check if service is running (`http://localhost:8085/health`)
2. Verify CORS settings
3. Check firewall configuration

### Data Display Issues

**Problem**: Memory content displays incorrectly

**Solution**:
1. Refresh page to reload data
2. Check server logs
3. Verify data format compatibility

## 📦 Scripts

| Script | Purpose |
|--------|---------|
| `dev` | Start Vite dev server |
| `build` | Build production bundle |
| `preview` | Preview production build |
| `serve` | Run Bun standalone server |
| `compile` | Compile to standalone executable |
| `compile:all` | Cross-compile for all platforms |

## 📄 License

MIT License - see the [LICENSE](../../LICENSE) file for details.

## 🔗 Related Resources

- [Cortex Memory Main Documentation](../README.md)
- [Cortex Memory Service](../cortex-mem-service/README.md)
- [Cortex Memory Core](../cortex-mem-core/README.md)
- [Svelte Documentation](https://svelte.dev/docs)
- [Vite Documentation](https://vitejs.dev/)

---

**Built with ❤️ using Svelte 5 and TypeScript**
