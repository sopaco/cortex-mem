# Cortex Memory 架构设计文档

## 概述

Cortex Memory 是一个高性能、模块化的 AI Agent 记忆管理系统，采用 `cortex://` 虚拟 URI 协议，实现 L0/L1/L2 三层抽象架构，为 AI Agent 提供长期记忆存储和智能检索能力。

**版本**: V2.0.0  
**最后更新**: 2026-02-12

---

## 系统架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              应用层 (Application Layer)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │     CLI      │  │     MCP      │  │    HTTP      │  │   Web UI     │     │
│  │    工具      │  │   服务器     │  │   服务       │  │  (开发中)    │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
└─────────┼─────────────────┼─────────────────┼─────────────────┼─────────────┘
          │                 │                 │                 │
          └─────────────────┴─────────────────┴─────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │     cortex-mem-tools          │
                    │      (高级工具库)              │
                    └───────────────┬───────────────┘
                                    │
          ┌─────────────────────────┼─────────────────────────┐
          │                         │                         │
┌─────────▼──────────┐  ┌───────────▼────────────┐  ┌─────────▼──────────┐
│   cortex-mem-rig   │  │    cortex-mem-core     │  │  cortex-mem-config │
│  (Rig框架集成)      │  │      (核心库)           │  │    (配置管理)       │
└─────────┬──────────┘  └───────────┬────────────┘  └─────────┬──────────┘
          │                         │                         │
          └─────────────────────────┼─────────────────────────┘
                                    │
                    ┌───────────────▼───────────────┐
                    │      存储层 (Storage)         │
                    │  ┌─────────────────────────┐  │
                    │  │   Markdown 文件系统      │  │
                    │  │   (cortex-data/)        │  │
                    │  └─────────────────────────┘  │
                    │  ┌─────────────────────────┐  │
                    │  │   Qdrant 向量数据库      │  │
                    │  │   (可选，feature-gated)  │  │
                    │  └─────────────────────────┘  │
                    └───────────────────────────────┘
```

---

## 核心架构原则

### 1. 虚拟文件系统 (cortex://)

采用虚拟 URI 协议统一内存访问，所有数据以纯 Markdown 格式存储：

- **Protocol**: `cortex://`
- **Format**: Markdown
- **Structure**: 层次化目录组织
- **Access**: 通过 URI 路径访问

**URI 格式**:
```
cortex://{dimension}/{category}/{id}.md
cortex://session/{thread_id}/timeline/{timestamp}.md
cortex://user/{user_id}/memories/{memory_id}.md
cortex://agent/{agent_id}/memories/{memory_id}.md
```

### 2. 三层抽象架构 (L0/L1/L2)

为优化 LLM 上下文窗口使用，实现三层内容抽象：

| 层级 | 名称 | Token 数 | 用途 | 生成方式 |
|------|------|----------|------|----------|
| L0 | Abstract | ~100 | 快速浏览、筛选 | 规则/LLM |
| L1 | Overview | ~2000 | 详细理解核心信息 | 规则/LLM |
| L2 | Detail | 完整 | 深度分析和编辑 | 原始内容 |

**Token 效率提升**: 相比传统方式节省 80-92% 的 token 消耗

### 3. 多维度存储

支持三种存储维度，满足不同场景需求：

| 维度 | 路径 | 用途 |
|------|------|------|
| session | `cortex://session/{id}/` | 会话级存储，临时对话 |
| user | `cortex://user/{id}/` | 用户长期记忆 |
| agent | `cortex://agent/{id}/` | Agent 专属记忆 |

### 4. 租户隔离

支持多租户架构，数据完全隔离：

```
{data_dir}/
└── tenants/
    ├── tenant_a/
    │   ├── session/
    │   ├── user/
    │   └── agent/
    └── tenant_b/
        ├── session/
        ├── user/
        └── agent/
```

---

## 核心模块 (cortex-mem-core)

### 模块结构

```
cortex-mem-core/src/
├── lib.rs                 # 库入口，模块导出
├── config.rs              # 配置管理
├── error.rs               # 错误处理
├── types.rs               # 通用类型定义
├── logging.rs             # 日志系统
│
├── filesystem/            # 虚拟文件系统
│   ├── uri.rs            # URI 解析
│   └── operations.rs     # 文件操作
│
├── session/               # 会话管理
│   ├── manager.rs        # 会话生命周期
│   ├── message.rs        # 消息存储
│   ├── timeline.rs       # 时间轴组织
│   └── participant.rs    # 参与者管理
│
├── layers/                # 三层抽象
│   ├── generator.rs      # L0/L1 生成器
│   └── manager.rs        # 层级管理
│
├── retrieval/             # 检索引擎
│   ├── intent.rs         # 意图分析
│   ├── relevance.rs      # 相关性计算
│   └── engine.rs         # 检索引擎
│
├── extraction/            # 记忆提取
│   ├── extractor.rs      # 提取器
│   ├── types.rs          # 提取类型
│   └── user_profile.rs   # 用户画像
│
├── llm/                   # LLM 集成
│   └── client.rs         # rig-core 0.23 封装
│
├── automation/            # 自动化
│   ├── indexer.rs        # 自动索引
│   ├── extractor.rs      # 自动提取
│   └── sync.rs           # 同步管理
│
├── index/                 # 全文索引
│   └── tantivy.rs        # Tantivy 集成
│
├── init/                  # 初始化工具
│   └── init.rs           # 目录结构初始化
│
├── vector_store/          # 向量存储 (可选)
│   └── qdrant.rs         # Qdrant 集成
│
├── embedding/             # Embedding (可选)
│   └── client.rs         # Embedding 客户端
│
└── search/                # 向量搜索 (可选)
    └── engine.rs         # 搜索引擎
```

### 关键模块详解

#### 1. Filesystem 模块

负责虚拟文件系统的实现：

- **URI 解析**: 将 `cortex://` URI 转换为文件系统路径
- **文件操作**: 读写、列表、删除、元数据
- **租户隔离**: 支持多租户数据隔离

```rust
pub trait FilesystemOperations: Send + Sync {
    async fn list(&self, uri: &str) -> Result<Vec<FileEntry>>;
    async fn read(&self, uri: &str) -> Result<String>;
    async fn write(&self, uri: &str, content: &str) -> Result<()>;
    async fn delete(&self, uri: &str) -> Result<()>;
    async fn exists(&self, uri: &str) -> Result<bool>;
    async fn metadata(&self, uri: &str) -> Result<FileMetadata>;
}
```

#### 2. Session 模块

管理对话会话的生命周期：

- **会话管理**: 创建、更新、关闭、归档
- **消息存储**: 支持多角色（user/assistant/system）
- **Timeline**: 按时间组织消息
- **参与者**: 管理会话参与者

```rust
pub struct SessionManager {
    filesystem: Arc<CortexFilesystem>,
    config: SessionConfig,
}

pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}
```

#### 3. Layers 模块

实现 L0/L1/L2 三层抽象：

```rust
pub enum ContextLayer {
    L0Abstract,  // ~100 tokens
    L1Overview,  // ~2000 tokens
    L2Detail,    // Full content
}

pub struct LayerManager {
    filesystem: Arc<CortexFilesystem>,
    abstract_gen: AbstractGenerator,
    overview_gen: OverviewGenerator,
    llm_client: Option<Arc<dyn LLMClient>>,
}
```

#### 4. Retrieval 模块

智能检索引擎：

- **意图分析**: 理解用户查询意图
- **递归检索**: 分层检索相关内容
- **相关性计算**: 评估内容相关性

#### 5. Extraction 模块

LLM 驱动的记忆提取：

- **事实提取**: 提取关键事实
- **决策记录**: 记录重要决策
- **实体识别**: 识别人、物、概念
- **用户画像**: 构建用户档案

---

## 工具链架构

### 1. cortex-mem-cli

命令行工具，提供完整的 CLI 接口：

```
cortex-mem session create <id>
cortex-mem add --thread <id> <content>
cortex-mem search <query>
cortex-mem session extract <id>
```

### 2. cortex-mem-mcp

MCP 服务器，与 Claude Desktop 集成：

- **Tools**: store_memory, list_memories, get_memory, delete_memory, search_memories, query_memory
- **Transport**: stdio
- **Protocol**: MCP (Model Context Protocol)

### 3. cortex-mem-service

HTTP REST API 服务：

- **Framework**: Axum
- **Endpoints**: /health, /api/v2/sessions, /api/v2/search
- **Features**: CORS, 日志追踪

### 4. cortex-mem-tools

高级工具库，提供 8 个 OpenViking 风格工具：

| 工具 | 功能 | 层级 |
|------|------|------|
| abstract | 读取 L0 摘要 | L0 |
| overview | 读取 L1 概览 | L1 |
| read | 读取完整内容 | L2 |
| search | 全文搜索 | - |
| find | 语义搜索 | - |
| ls | 列出目录 | - |
| explore | 探索目录结构 | - |
| store | 存储记忆 | - |

### 5. cortex-mem-rig

Rig 框架集成：

```rust
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
}

// 提供 8 个工具方法
pub fn abstract_tool(&self) -> AbstractTool;
pub fn overview_tool(&self) -> OverviewTool;
pub fn read_tool(&self) -> ReadTool;
pub fn search_tool(&self) -> SearchTool;
pub fn find_tool(&self) -> FindTool;
pub fn ls_tool(&self) -> LsTool;
pub fn explore_tool(&self) -> ExploreTool;
pub fn store_tool(&self) -> StoreTool;
```

---

## 搜索架构

支持三种搜索模式：

### 1. Filesystem 搜索 (默认)

基于文件系统的全文搜索：

- **实现**: 遍历 + 正则匹配
- **优点**: 零外部依赖，快速启动
- **适用**: 小规模数据，简单查询

### 2. Vector 搜索 (可选)

基于向量相似度的语义搜索：

- **实现**: Qdrant 向量数据库
- **优点**: 语义理解，模糊匹配
- **适用**: 大规模数据，语义查询
- **要求**: 启用 `vector-search` feature，运行 Qdrant

### 3. Hybrid 搜索

结合文本和语义搜索：

- **实现**: 同时执行两种搜索，合并结果
- **优点**: 兼顾精确性和语义理解
- **适用**: 复杂查询场景

---

## 数据流

### 1. 消息存储流程

```
User Input
    │
    ▼
┌─────────────────┐
│ SessionManager  │
│  create_message │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ LayerManager    │
│ generate_all_   │
│ layers (L0/L1)  │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
L2 (Detail)  L0/L1 (Abstract)
    │            │
    └────┬───────┘
         ▼
┌─────────────────┐
│ CortexFilesystem│
│   write()       │
└────────┬────────┘
         ▼
   Markdown Files
```

### 2. 记忆提取流程

```
Session Messages
    │
    ▼
┌─────────────────┐
│ MemoryExtractor │
│  extract()      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  LLM Client     │
│ (rig-core 0.23) │
└────────┬────────┘
         │
         ▼
ExtractedMemories
├── facts: Vec<ExtractedFact>
├── decisions: Vec<ExtractedDecision>
├── entities: Vec<ExtractedEntity>
└── user_profile: UserProfile
         │
         ▼
┌─────────────────┐
│ CortexFilesystem│
│   write()       │
└────────┬────────┘
         ▼
cortex://user/{id}/memories/
```

### 3. 检索流程

```
Query
    │
    ▼
┌─────────────────┐
│ IntentAnalyzer  │
│  analyze()      │
└────────┬────────┘
         │
         ▼
    Intent
├── keywords: Vec<String>
├── entities: Vec<String>
├── filters: Filters
└── layer: ContextLayer
         │
         ▼
┌─────────────────┐
│ RetrievalEngine │
│  retrieve()     │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
Filesystem  Vector (optional)
Search      Search
    │         │
    └────┬────┘
         ▼
┌─────────────────┐
│ RelevanceCalc   │
│  calculate()    │
└────────┬────────┘
         ▼
Ranked Results
```

---

## Feature Flags

| Feature | 描述 | 依赖 |
|---------|------|------|
| `vector-search` | 启用向量搜索功能 | Qdrant |
| `default` | 仅文件系统搜索 | 无 |

---

## 配置架构

### 配置文件 (config.toml)

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key"
model_efficient = "gpt-4"
temperature = 0.1
max_tokens = 4096

[vector_search]
enabled = true
qdrant_url = "http://localhost:6333"
collection_name = "cortex-mem"
embedding_dim = 1536
```

### 环境变量

```bash
CORTEX_DATA_DIR="./cortex-data"
LLM_API_BASE_URL="https://api.openai.com/v1"
LLM_API_KEY="your-api-key"
QDRANT_URL="http://localhost:6333"
```

---

## 安全设计

### 1. 数据隔离

- 租户级别数据隔离
- 用户/Agent 维度分离
- 会话级访问控制

### 2. 存储安全

- 本地 Markdown 文件存储
- 支持 Git 版本控制
- 易于备份和迁移

### 3. 传输安全

- HTTP 服务支持 HTTPS（需反向代理）
- MCP 通过 stdio 本地通信
- CLI 本地执行

---

## 性能优化

### 1. Token 效率

| 场景 | 传统方式 | Cortex Memory | 节省 |
|------|----------|---------------|------|
| 搜索 20 个记忆 | 100,000 tokens | 8,000 tokens | 92% |
| 加载 10 个会话 | 50,000 tokens | 5,000 tokens | 90% |

### 2. 延迟目标

| 操作 | 目标延迟 | 状态 |
|------|----------|------|
| 文件读取 | < 10ms | ✅ 达标 |
| 消息添加 | < 50ms | ✅ 达标 |
| 全文搜索 | < 100ms | ✅ 达标 |
| 记忆提取 | 2-5s | ✅ 达标 |

### 3. 可扩展性

- 水平扩展：通过租户隔离支持多实例
- 垂直扩展：可选向量搜索处理大规模数据
- 存储扩展：文件系统天然支持大容量

---

## 部署架构

### 1. 单机部署

```
┌─────────────────────────────────────┐
│           单机部署                   │
│  ┌─────────────────────────────┐   │
│  │      cortex-mem-service     │   │
│  │         (Axum)              │   │
│  └─────────────┬───────────────┘   │
│                │                    │
│  ┌─────────────▼───────────────┐   │
│  │    cortex-mem-core          │   │
│  │    (文件系统 + 可选Qdrant)   │   │
│  └─────────────┬───────────────┘   │
│                │                    │
│  ┌─────────────▼───────────────┐   │
│  │      ./cortex-data/         │   │
│  │      (Markdown 文件)         │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

### 2. 多租户部署

```
┌─────────────────────────────────────┐
│         负载均衡器                  │
└─────────────┬───────────────────────┘
              │
    ┌─────────┼─────────┐
    ▼         ▼         ▼
┌───────┐ ┌───────┐ ┌───────┐
│Inst-1 │ │Inst-2 │ │Inst-3 │
│TenantA│ │TenantB│ │TenantC│
└───┬───┘ └───┬───┘ └───┬───┘
    │         │         │
    └─────────┼─────────┘
              ▼
    ┌─────────────────┐
    │  Shared Qdrant  │
    │  (可选)         │
    └─────────────────┘
```

---

## 总结

Cortex Memory V2 采用分层架构设计，核心特点：

1. **虚拟文件系统**: `cortex://` 协议统一访问
2. **三层抽象**: L0/L1/L2 优化 Token 使用
3. **多维度存储**: session/user/agent 满足不同场景
4. **模块化设计**: 核心库 + 多种工具链
5. **可选向量搜索**: Feature-gated，灵活部署
6. **租户隔离**: 支持多租户架构

架构优势：
- ✅ 零外部依赖（基础功能）
- ✅ 高性能（Token 效率提升 80-92%）
- ✅ 易部署（纯 Markdown 存储）
- ✅ 可扩展（模块化 + Feature flags）
- ✅ 多访问方式（CLI/MCP/HTTP/Tools/Rig）
