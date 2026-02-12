# Cortex Memory 子项目模块文档

本文档详细介绍 Cortex Memory 项目中各个子项目（crate）的职责、接口和使用方式。

**版本**: V2.0.0  
**最后更新**: 2026-02-12

---

## 项目结构概览

```
cortex-mem/
├── cortex-mem-core/       # 核心库（13个模块）
├── cortex-mem-cli/        # 命令行工具
├── cortex-mem-mcp/        # MCP 服务器
├── cortex-mem-service/    # HTTP REST API 服务
├── cortex-mem-tools/      # 高级工具库
├── cortex-mem-rig/        # Rig 框架集成
├── cortex-mem-config/     # 配置管理
└── cortex-mem-insights/   # Web 管理界面（开发中）
```

---

## 1. cortex-mem-core

**类型**: Library  
**路径**: `cortex-mem-core/`  
**描述**: 核心库，提供所有基础功能

### 职责

- 虚拟文件系统实现
- 会话管理
- 三层抽象架构 (L0/L1/L2)
- 检索引擎
- 记忆提取
- LLM 集成
- 全文索引
- 向量存储（可选）

### 模块结构

```rust
// 核心模块（始终可用）
pub mod config;           // 配置管理
pub mod error;            // 错误处理
pub mod types;            // 通用类型
pub mod logging;          // 日志系统
pub mod filesystem;       // 虚拟文件系统
pub mod session;          // 会话管理
pub mod layers;           // 三层抽象
pub mod retrieval;        // 检索引擎
pub mod extraction;       // 记忆提取
pub mod llm;              // LLM 集成
pub mod automation;       // 自动化
pub mod index;            // 全文索引
pub mod init;             // 初始化工具

// 可选模块（需要 vector-search feature）
#[cfg(feature = "vector-search")]
pub mod vector_store;     // 向量存储
#[cfg(feature = "vector-search")]
pub mod embedding;        // Embedding 客户端
#[cfg(feature = "vector-search")]
pub mod search;           // 向量搜索
```

### 主要类型

```rust
// 文件系统
pub use filesystem::{CortexFilesystem, FilesystemOperations};

// 会话管理
pub use session::{SessionManager, SessionConfig, Message, MessageRole};

// 三层抽象
pub use layers::LayerManager;

// 检索
pub use retrieval::{RetrievalEngine, RetrievalOptions, RetrievalResult};

// 提取
pub use extraction::{MemoryExtractor, ExtractionConfig};

// LLM
pub use llm::LLMClient;

// 向量搜索（可选）
#[cfg(feature = "vector-search")]
pub use vector_store::{VectorStore, QdrantVectorStore};
```

### 使用示例

```rust
use cortex_mem_core::{CortexFilesystem, FilesystemOperations};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化文件系统
    let filesystem = Arc::new(CortexFilesystem::new("./cortex-data"));
    filesystem.initialize().await?;
    
    // 写入数据
    filesystem.write("cortex://test.md", "Hello, Cortex!").await?;
    
    // 读取数据
    let content = filesystem.read("cortex://test.md").await?;
    println!("Content: {}", content);
    
    Ok(())
}
```

---

## 2. cortex-mem-cli

**类型**: Binary  
**路径**: `cortex-mem-cli/`  
**描述**: 命令行工具，提供完整的 CLI 接口

### 职责

- 会话管理
- 消息操作
- 搜索记忆
- 记忆提取
- 统计信息

### 命令列表

```bash
# 会话管理
cortex-mem session create <id> [options]
cortex-mem session list [options]
cortex-mem session get <id>
cortex-mem session close <id>
cortex-mem session archive <id>
cortex-mem session delete <id>
cortex-mem session extract <id>

# 消息操作
cortex-mem add --thread <id> <content>
cortex-mem add --thread <id> --role assistant <content>

# 搜索
cortex-mem search <query> [--thread <id>]
cortex-mem list [--thread <id>]
cortex-mem get <uri>

# 统计
cortex-mem stats

# 其他
cortex-mem delete <uri>
```

### 使用示例

```bash
# 创建会话
cortex-mem session create my-session --title "技术讨论"

# 添加消息
cortex-mem add --thread my-session "如何实现 OAuth 2.0？"
cortex-mem add --thread my-session --role assistant "建议使用授权码流程"

# 搜索记忆
cortex-mem search "OAuth" --thread my-session

# 提取记忆
cortex-mem session extract my-session

# 查看统计
cortex-mem stats
```

---

## 3. cortex-mem-mcp

**类型**: Binary  
**路径**: `cortex-mem-mcp/`  
**描述**: MCP 服务器，与 Claude Desktop 集成

### 职责

- 实现 MCP 协议
- 提供记忆操作工具
- 与 Claude Desktop 通信

### 可用工具

| 工具名 | 功能 | 参数 |
|--------|------|------|
| store_memory | 存储记忆 | uri, content, metadata |
| list_memories | 列出记忆 | uri, limit |
| get_memory | 获取记忆 | uri, layer |
| delete_memory | 删除记忆 | uri |
| search_memories | 搜索记忆 | query, filters |
| query_memory | 语义搜索 | query, limit |

### 配置方式

编辑 Claude Desktop 配置文件：

```json
{
  "mcpServers": {
    "cortex-mem": {
      "command": "/path/to/cortex-mem-mcp",
      "args": ["--config", "/path/to/config.toml"],
      "env": {
        "CORTEX_DATA_DIR": "/path/to/cortex-data"
      }
    }
  }
}
```

### 使用示例

```bash
# 启动 MCP 服务器
cortex-mem-mcp --config config.toml --agent my-agent --user my-user
```

---

## 4. cortex-mem-service

**类型**: Binary  
**路径**: `cortex-mem-service/`  
**描述**: HTTP REST API 服务

### 职责

- 提供 REST API
- 支持三种搜索模式
- CORS 支持
- 日志追踪

### API 端点

#### 健康检查

```
GET /health
```

#### 会话管理

```
POST   /api/v2/sessions              # 创建会话
GET    /api/v2/sessions              # 列出会话
GET    /api/v2/sessions/{id}         # 获取会话
PUT    /api/v2/sessions/{id}         # 更新会话
DELETE /api/v2/sessions/{id}         # 删除会话
POST   /api/v2/sessions/{id}/close   # 关闭会话
POST   /api/v2/sessions/{id}/archive # 归档会话
```

#### 消息操作

```
POST /api/v2/sessions/{id}/messages  # 添加消息
GET  /api/v2/sessions/{id}/messages  # 获取消息
```

#### 搜索

```
POST /api/v2/search                  # 搜索记忆
POST /api/v2/query                   # 语义搜索
```

#### 记忆提取

```
POST /api/v2/automation/extract/{id} # 提取会话记忆
```

### 启动方式

```bash
# 基础启动
cortex-mem-service

# 自定义配置
cortex-mem-service --data-dir ./my-data --port 8080 --verbose
```

### 请求示例

```bash
# 创建会话
curl -X POST http://localhost:8080/api/v2/sessions \
  -H "Content-Type: application/json" \
  -d '{"id": "test-session", "title": "测试会话"}'

# 添加消息
curl -X POST http://localhost:8080/api/v2/sessions/test-session/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "Hello!"}'

# 搜索
curl -X POST http://localhost:8080/api/v2/search \
  -H "Content-Type: application/json" \
  -d '{"query": "Hello", "mode": "filesystem"}'
```

---

## 5. cortex-mem-tools

**类型**: Library  
**路径**: `cortex-mem-tools/`  
**描述**: 高级工具库，提供 8 个 OpenViking 风格工具

### 职责

- 封装核心功能为工具
- 提供统一的工具接口
- 支持 MCP 工具定义

### 工具列表

| 工具 | 结构体 | 功能 | 层级 |
|------|--------|------|------|
| abstract | `AbstractTool` | 读取 L0 摘要 | L0 |
| overview | `OverviewTool` | 读取 L1 概览 | L1 |
| read | `ReadTool` | 读取完整内容 | L2 |
| search | `SearchTool` | 全文搜索 | - |
| find | `FindTool` | 语义搜索 | - |
| ls | `LsTool` | 列出目录 | - |
| explore | `ExploreTool` | 探索目录结构 | - |
| store | `StoreTool` | 存储记忆 | - |

### 核心类型

```rust
pub struct MemoryOperations {
    filesystem: Arc<CortexFilesystem>,
    layer_manager: LayerManager,
    retrieval_engine: RetrievalEngine,
    #[cfg(feature = "vector-search")]
    vector_engine: Option<VectorSearchEngine>,
}

impl MemoryOperations {
    // 分层访问
    pub async fn load_abstract(&self, uri: &str) -> Result<String>;
    pub async fn load_overview(&self, uri: &str) -> Result<String>;
    pub async fn read(&self, uri: &str) -> Result<String>;
    
    // 搜索
    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>>;
    pub async fn find(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    
    // 文件系统
    pub async fn ls(&self, uri: &str) -> Result<Vec<FileEntry>>;
    pub async fn explore(&self, uri: &str, depth: usize) -> Result<DirectoryTree>;
    
    // 存储
    pub async fn store(&self, uri: &str, content: &str, metadata: Metadata) -> Result<()>;
}
```

### 使用示例

```rust
use cortex_mem_tools::MemoryOperations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化
    let ops = MemoryOperations::new("./cortex-data").await?;
    
    // 存储记忆
    ops.store(
        "cortex://user/my-memory.md",
        "这是记忆内容",
        Metadata::default()
    ).await?;
    
    // 读取 L0 摘要
    let abstract_text = ops.load_abstract("cortex://user/my-memory.md").await?;
    println!("摘要: {}", abstract_text);
    
    // 搜索
    let results = ops.search("关键词", SearchOptions::default()).await?;
    
    Ok(())
}
```

---

## 6. cortex-mem-rig

**类型**: Library  
**路径**: `cortex-mem-rig/`  
**描述**: Rig 框架集成，适配 Rig 0.23

### 职责

- 提供 Rig 风格的工具
- 简化 Agent 集成
- 支持租户隔离

### 核心类型

```rust
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
}

impl MemoryTools {
    pub fn new(operations: Arc<MemoryOperations>) -> Self;
    
    // 分层访问工具
    pub fn abstract_tool(&self) -> AbstractTool;
    pub fn overview_tool(&self) -> OverviewTool;
    pub fn read_tool(&self) -> ReadTool;
    
    // 搜索工具
    pub fn search_tool(&self) -> SearchTool;
    pub fn find_tool(&self) -> FindTool;
    
    // 文件系统工具
    pub fn ls_tool(&self) -> LsTool;
    pub fn explore_tool(&self) -> ExploreTool;
    
    // 存储工具
    pub fn store_tool(&self) -> StoreTool;
}

// 便捷创建函数
pub fn create_memory_tools(operations: Arc<MemoryOperations>) -> MemoryTools;

pub async fn create_memory_tools_with_tenant(
    data_dir: impl AsRef<std::path::Path>,
    tenant_id: impl Into<String>,
) -> Result<MemoryTools, Box<dyn std::error::Error>>;

pub async fn create_memory_tools_with_tenant_and_llm(
    data_dir: impl AsRef<std::path::Path>,
    tenant_id: impl Into<String>,
    llm_client: Arc<dyn LLMClient>,
) -> Result<MemoryTools, Box<dyn std::error::Error>>;
```

### 使用示例

```rust
use cortex_mem_rig::{create_memory_tools_with_tenant, MemoryTools};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建带租户隔离的工具
    let tools = create_memory_tools_with_tenant(
        "./cortex-data",
        "my-tenant"
    ).await?;
    
    // 使用工具
    let abstract_tool = tools.abstract_tool();
    let search_tool = tools.search_tool();
    let store_tool = tools.store_tool();
    
    Ok(())
}
```

---

## 7. cortex-mem-config

**类型**: Library  
**路径**: `cortex-mem-config/`  
**描述**: 配置管理

### 职责

- 配置文件解析
- 环境变量读取
- 配置验证

### 配置结构

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub llm: LLMConfig,
    pub vector_search: Option<VectorSearchConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LLMConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model_efficient: String,
    pub temperature: f32,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VectorSearchConfig {
    pub enabled: bool,
    pub qdrant_url: String,
    pub collection_name: String,
    pub embedding_dim: usize,
}
```

### 使用示例

```rust
use cortex_mem_config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从文件加载
    let config = Config::from_file("config.toml").await?;
    
    // 从环境变量加载
    let config = Config::from_env()?;
    
    Ok(())
}
```

---

## 8. cortex-mem-insights

**类型**: Web Application  
**路径**: `cortex-mem-insights/`  
**描述**: Web 管理界面（开发中）

### 技术栈

- **Framework**: SvelteKit
- **Language**: TypeScript
- **Styling**: TailwindCSS
- **Build**: Vite

### 功能规划

- [ ] 会话管理界面
- [ ] 记忆浏览和搜索
- [ ] 统计仪表盘
- [ ] 配置管理
- [ ] 实时监控

### 开发状态

🚧 开发中，预计 V2.1 发布

---

## 依赖关系图

```
cortex-mem-core
    │
    ├──► cortex-mem-tools
    │       │
    │       ├──► cortex-mem-rig
    │       │
    │       └──► cortex-mem-mcp (通过 ToolDefinition)
    │
    ├──► cortex-mem-cli
    │
    ├──► cortex-mem-service
    │
    └──► cortex-mem-config (被所有项目依赖)

cortex-mem-insights (独立，通过 HTTP API 通信)
```

---

## 版本对应关系

| Crate | 版本 | 说明 |
|-------|------|------|
| cortex-mem-core | 2.0.0 | 核心库 |
| cortex-mem-cli | 2.0.0 | CLI 工具 |
| cortex-mem-mcp | 2.0.0 | MCP 服务器 |
| cortex-mem-service | 2.0.0 | HTTP 服务 |
| cortex-mem-tools | 2.0.0 | 工具库 |
| cortex-mem-rig | 2.0.0 | Rig 集成 |
| cortex-mem-config | 2.0.0 | 配置管理 |
| cortex-mem-insights | 0.1.0 | Web 界面（开发中） |

---

## 构建命令

```bash
# 构建整个 workspace
cargo build --release --workspace

# 构建带向量搜索
cargo build --release --workspace --features vector-search

# 单独构建
cargo build --release -p cortex-mem-core
cargo build --release -p cortex-mem-cli
cargo build --release -p cortex-mem-mcp
cargo build --release -p cortex-mem-service
cargo build --release -p cortex-mem-tools
cargo build --release -p cortex-mem-rig
```
