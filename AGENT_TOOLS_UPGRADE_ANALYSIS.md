# 🔄 Agent 记忆工具架构升级分析

## 📋 问题分析

用户要求分析当前 TARS 使用的工具问题，并借鉴 OpenViking 的 agent 集成方式来升级新架构的记忆能力。

---

## 🎯 核心发现

### 1. OpenViking 的 Agent 集成方式

根据 DeepWiki 分析，OpenViking 提供了创新的 **Tiered Context Loading** 机制：

#### L0/L1/L2 三层渐进式加载

```
┌─────────────────────────────────────────────────┐
│  L0: Abstract (~100 tokens)                     │
│  - 一句话摘要                                   │
│  - 用于快速相关性检查和过滤                      │
│  - API: client.abstract(uri)                    │
└─────────────────────────────────────────────────┘
           ↓ (如果相关，继续加载)
┌─────────────────────────────────────────────────┐
│  L1: Overview (~2000 tokens)                    │
│  - 核心信息和使用场景                           │
│  - 用于 agent 规划和决策                        │
│  - API: client.overview(uri)                    │
└─────────────────────────────────────────────────┘
           ↓ (需要细节时，才加载)
┌─────────────────────────────────────────────────┐
│  L2: Details (完整内容)                         │
│  - 完整原始数据                                 │
│  - 仅在需要深度阅读时加载                        │
│  - API: client.read(uri)                        │
└─────────────────────────────────────────────────┘
```

**优势**:
- ✅ **大幅减少 Token 消耗**（Progressive Disclosure）
- ✅ **提高检索效率**（先用 L0 过滤，再用 L1 规划）
- ✅ **按需加载**（只在必要时加载 L2 完整内容）

---

#### OpenViking 的 Agent API

```python
# 文件系统式 API
client.ls("viking://resources/")           # 列出目录
client.find("query")                        # 简单语义搜索
client.search("complex query", session)     # 复杂搜索（带意图分析）

# Tiered Loading API
client.abstract(uri)   # L0: 摘要
client.overview(uri)   # L1: 概览
client.read(uri)       # L2: 完整内容
```

---

#### OpenViking 的递归检索策略

```
Directory Recursive Retrieval:
1. Intent Analysis        ← 分析查询意图
2. Initial Positioning    ← 向量检索定位高分目录
3. Refined Exploration    ← 在目录内二次检索
4. Recursive Drill-down   ← 递归处理子目录
5. Result Aggregation     ← 聚合结果
```

---

### 2. 当前 TARS 的工具架构（老架构）

#### 现有工具清单

| 工具 | 功能 | 参数 | 问题 |
|------|------|------|------|
| `store_memory` | 存储记忆 | content, user_id, agent_id, memory_type, topics | ❌ 无分层支持 |
| `query_memory` | 语义搜索 | query, k, memory_type, topics, ... | ❌ 无渐进式加载 |
| `list_memories` | 列出记忆 | limit, memory_type, user_id, ... | ❌ 返回完整内容 |
| `get_memory` | 获取单条 | memory_id | ❌ 只能获取完整内容 |

#### 现有架构的局限性

1. **❌ 无分层加载**
   - 所有工具都返回完整内容（L2 level）
   - Agent 必须处理大量 token
   - 无法快速过滤不相关记忆

2. **❌ 无递归检索**
   - `query_memory` 只做简单向量搜索
   - 没有 OpenViking 的 Directory Recursive Retrieval
   - 检索效果不如新架构的 RetrievalEngine

3. **❌ 无文件系统式 API**
   - 没有 `ls()` 列出目录结构
   - 没有 `abstract()` / `overview()` / `read()` 分层访问
   - Agent 难以探索记忆空间

4. **❌ 基于向量搜索**
   - 依赖 Qdrant（外部依赖）
   - 需要 Embedding API（成本高）
   - 而新架构的 RetrievalEngine 已支持关键词检索

---

### 3. 新架构的能力（Cortex-Mem V2）

#### 已实现的功能

| 功能 | 模块 | 状态 |
|------|------|------|
| **分层内容管理** | `LayerManager` | ✅ 已实现 |
| **L0 Abstract** | `LayerGenerator::generate_abstract()` | ✅ 已实现 |
| **L1 Overview** | `LayerGenerator::generate_overview()` | ✅ 已实现 |
| **L2 Details** | `CortexFilesystem::read()` | ✅ 已实现 |
| **递归检索** | `RetrievalEngine::search()` | ✅ 已实现 |
| **向量检索** | `VectorSearchEngine::recursive_search()` | ✅ 已实现（未启用）|
| **文件系统** | `CortexFilesystem` | ✅ 已实现 |
| **会话管理** | `SessionManager` | ✅ 已实现 |

---

## 🎯 问题诊断

### 当前 TARS 的工具架构问题

```rust
// 当前工具（老架构）
store_memory(content, user_id, agent_id, ...)    // ❌ 直接存储，无分层
query_memory(query, k, ...)                      // ❌ 返回完整内容
list_memories(limit, ...)                        // ❌ 返回完整内容
get_memory(memory_id)                            // ❌ 返回完整内容
```

**问题**:
1. ✅ 新架构已经支持 L0/L1/L2 分层（LayerManager）
2. ❌ 但工具没有暴露分层访问能力
3. ❌ Agent 被迫处理所有 L2 完整内容
4. ❌ 无法利用 RetrievalEngine 的递归检索
5. ❌ 无法利用 VectorSearchEngine 的向量检索

---

## 🔧 解决方案设计

### 方案 1: OpenViking 风格的分层工具

#### 新工具设计

```rust
// 文件系统式工具
ls(uri)                    // 列出目录内容
find(query, scope)         // 简单搜索（关键词或向量）

// Tiered Loading 工具
abstract(uri)              // L0: 获取摘要 (~100 tokens)
overview(uri)              // L1: 获取概览 (~2000 tokens)
read(uri)                  // L2: 获取完整内容

// 高级检索工具
search(query, options)     // 复杂搜索（支持递归、向量、混合）
  - engine: "keyword" | "vector" | "hybrid"
  - recursive: true/false
  - scope: uri
  - layers: ["L0", "L1", "L2"]  // 返回哪些层级

// 存储工具
store(content, metadata)   // 存储内容（自动生成 L0/L1）
```

---

### 方案 2: 兼容老工具 + 新增分层工具

#### 保留老工具（向后兼容）

```rust
// 保留（MCP 兼容）
store_memory(content, ...)
query_memory(query, ...)
list_memories(limit, ...)
get_memory(memory_id)
```

#### 新增分层工具

```rust
// 新增 Tiered Tools
get_abstract(uri) -> L0Abstract
get_overview(uri) -> L1Overview
get_details(uri) -> L2Details

// 新增搜索工具
search_tiered(query, options) -> TieredResults {
  matches: [{
    uri: String,
    abstract: L0Abstract,    // 默认返回 L0
    overview: Option<L1Overview>,  // 可选返回 L1
    score: f32,
  }]
}

// 新增文件系统工具
list_directory(uri) -> DirectoryListing
explore(query, start_uri) -> ExplorationResult
```

---

## 📊 对比表

| 方面 | 老架构工具 | OpenViking API | 新架构能力 | 建议工具 |
|------|-----------|---------------|-----------|---------|
| **分层加载** | ❌ 无 | ✅ L0/L1/L2 | ✅ LayerManager | ✅ abstract/overview/read |
| **递归检索** | ❌ 无 | ✅ Directory Recursive | ✅ RetrievalEngine | ✅ search(recursive=true) |
| **向量检索** | ✅ query_memory | ✅ search() | ✅ VectorSearchEngine | ✅ search(engine="vector") |
| **关键词检索** | ❌ 无 | ⚠️ 未知 | ✅ RetrievalEngine | ✅ search(engine="keyword") |
| **文件系统** | ❌ 无 | ✅ ls/find | ✅ CortexFilesystem | ✅ ls/find |
| **Token 优化** | ❌ 无 | ✅ Progressive Disclosure | ✅ 支持 | ✅ 分层工具 |

---

## 🎯 推荐方案

### 方案 2+: 混合方案（推荐）

#### Phase 1: 新增分层工具（优先）

```rust
// 1. Tiered Access Tools
pub struct AbstractTool;        // 获取 L0 摘要
pub struct OverviewTool;        // 获取 L1 概览
pub struct DetailsTool;         // 获取 L2 完整内容

// 2. Enhanced Search Tool
pub struct SearchTool {
    engine: SearchEngine,       // keyword | vector | hybrid
    recursive: bool,            // 递归搜索
    return_layers: Vec<Layer>,  // 返回哪些层级
}

// 3. Filesystem Tools
pub struct ListDirectoryTool;   // 列出目录
pub struct ExploreTool;         // 探索记忆空间
```

#### Phase 2: 保留兼容工具

```rust
// 保留老工具（MCP 兼容）
pub struct StoreMemoryTool;
pub struct QueryMemoryTool;
pub struct ListMemoriesTool;
pub struct GetMemoryTool;
```

---

## 🔍 实现细节

### 1. AbstractTool 实现

```rust
pub struct AbstractTool {
    base: Arc<MemoryToolsBase>,
}

#[derive(Serialize, Deserialize)]
pub struct AbstractArgs {
    pub uri: String,
}

#[derive(Serialize, Deserialize)]
pub struct AbstractOutput {
    pub uri: String,
    pub abstract_text: String,  // L0 摘要
    pub word_count: usize,       // ~100 tokens
}

impl Tool for AbstractTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_abstract".to_string(),
            description: "获取记忆的 L0 摘要（~100 tokens），用于快速判断相关性".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "记忆的 URI（如 cortex://threads/xxx/timeline/...）"
                    }
                },
                "required": ["uri"]
            }),
        }
    }

    async fn call(&self, args: &str) -> Result<String, ToolError> {
        let args: AbstractArgs = serde_json::from_str(args)?;
        
        // 使用 LayerManager 获取 L0
        let layer_manager = LayerManager::new(self.base.filesystem.clone());
        let abstract_text = layer_manager.get_or_generate_abstract(&args.uri).await?;
        
        let output = AbstractOutput {
            uri: args.uri,
            abstract_text,
            word_count: abstract_text.split_whitespace().count(),
        };
        
        Ok(serde_json::to_string(&output)?)
    }
}
```

---

### 2. SearchTool 实现

```rust
pub struct SearchTool {
    base: Arc<MemoryToolsBase>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    pub engine: Option<String>,      // "keyword" | "vector" | "hybrid"
    pub recursive: Option<bool>,     // 递归搜索
    pub return_layers: Option<Vec<String>>,  // ["L0", "L1", "L2"]
    pub scope: Option<String>,       // 搜索范围 URI
    pub limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub uri: String,
    pub score: f32,
    pub abstract: Option<String>,   // L0 (如果请求)
    pub overview: Option<String>,   // L1 (如果请求)
    pub content: Option<String>,    // L2 (如果请求)
}

impl Tool for SearchTool {
    async fn call(&self, args: &str) -> Result<String, ToolError> {
        let args: SearchArgs = serde_json::from_str(args)?;
        
        // 根据 engine 选择检索引擎
        let results = match args.engine.as_deref() {
            Some("vector") => {
                // 使用向量搜索引擎
                self.vector_search(&args).await?
            }
            Some("hybrid") => {
                // 混合搜索
                self.hybrid_search(&args).await?
            }
            _ => {
                // 默认使用关键词搜索
                self.keyword_search(&args).await?
            }
        };
        
        // 根据 return_layers 加载对应层级
        let enriched_results = self.enrich_results(
            results,
            args.return_layers.unwrap_or(vec!["L0".to_string()])
        ).await?;
        
        Ok(serde_json::to_string(&enriched_results)?)
    }
    
    async fn keyword_search(&self, args: &SearchArgs) -> Result<Vec<SearchResult>> {
        let engine = RetrievalEngine::new(
            self.base.filesystem.clone(),
            self.base.layer_manager.clone()
        );
        
        let options = RetrievalOptions {
            top_k: args.limit.unwrap_or(10),
            recursive: args.recursive.unwrap_or(true),
            ..Default::default()
        };
        
        let scope = args.scope.as_deref().unwrap_or("cortex://threads");
        let result = engine.search(&args.query, scope, options).await?;
        
        Ok(result.results.into_iter().map(|r| SearchResult {
            uri: r.uri,
            score: r.score,
            abstract: None,  // 按需加载
            overview: None,
            content: None,
        }).collect())
    }
    
    async fn vector_search(&self, args: &SearchArgs) -> Result<Vec<SearchResult>> {
        #[cfg(feature = "vector-search")]
        {
            let engine = self.base.vector_engine.as_ref()
                .ok_or(ToolError::Custom("Vector search not enabled".to_string()))?;
            
            let search_options = cortex_mem_core::search::SearchOptions {
                limit: args.limit.unwrap_or(10),
                threshold: 0.5,
                root_uri: args.scope.clone(),
                recursive: args.recursive.unwrap_or(true),
            };
            
            let results = if args.recursive.unwrap_or(true) {
                engine.recursive_search(
                    &args.query,
                    args.scope.as_deref().unwrap_or("cortex://threads"),
                    &search_options
                ).await?
            } else {
                engine.semantic_search(&args.query, &search_options).await?
            };
            
            Ok(results.into_iter().map(|r| SearchResult {
                uri: r.uri,
                score: r.score,
                abstract: None,
                overview: None,
                content: r.content,
            }).collect())
        }
        
        #[cfg(not(feature = "vector-search"))]
        {
            Err(ToolError::Custom("Vector search feature not enabled".to_string()))
        }
    }
}
```

---

## 📝 实装计划

### Step 1: 在 cortex-mem-rig 中实现新工具

```rust
// cortex-mem-rig/src/tiered_tools.rs (新文件)

mod tiered_tools;

pub struct TieredMemoryTools {
    base: Arc<MemoryToolsBase>,
}

impl TieredMemoryTools {
    pub fn abstract_tool(&self) -> AbstractTool { ... }
    pub fn overview_tool(&self) -> OverviewTool { ... }
    pub fn details_tool(&self) -> DetailsTool { ... }
    pub fn search_tool(&self) -> SearchTool { ... }
    pub fn list_directory_tool(&self) -> ListDirectoryTool { ... }
}
```

---

### Step 2: 扩展 MemoryToolsBase

```rust
pub struct MemoryToolsBase {
    operations: Arc<MemoryOperations>,
    filesystem: Arc<CortexFilesystem>,
    layer_manager: Arc<LayerManager>,
    
    #[cfg(feature = "vector-search")]
    vector_engine: Option<Arc<VectorSearchEngine>>,
    
    config: MemoryToolConfig,
}
```

---

### Step 3: 更新 TARS agent.rs

```rust
// 创建分层记忆工具
let tiered_tools = create_tiered_memory_tools(
    operations.clone(),
    config
);

let agent = llm_client
    .completion_model(model)
    .into_agent_builder()
    // 新的分层工具
    .tool(tiered_tools.search_tool())        // 智能搜索
    .tool(tiered_tools.abstract_tool())      // L0 摘要
    .tool(tiered_tools.overview_tool())      // L1 概览
    .tool(tiered_tools.details_tool())       // L2 完整内容
    .tool(tiered_tools.list_directory_tool()) // 浏览目录
    // 保留兼容工具（可选）
    .tool(memory_tools.store_memory())
    .tool(memory_tools.get_memory())
    .preamble(&system_prompt)
    .build();
```

---

## 🎯 总结

### 关键发现

1. ✅ **新架构已支持 L0/L1/L2**（LayerManager）
2. ✅ **新架构已支持递归检索**（RetrievalEngine）
3. ✅ **新架构已支持向量检索**（VectorSearchEngine）
4. ❌ **但工具没有暴露这些能力**

### 核心问题

- 老工具设计基于向量搜索架构
- 没有利用新架构的分层能力
- Agent 被迫处理完整内容（L2）
- Token 消耗大，效率低

### 解决方案

**实现 OpenViking 风格的分层工具**:
1. `abstract(uri)` - L0 摘要（快速过滤）
2. `overview(uri)` - L1 概览（规划决策）
3. `read(uri)` - L2 完整内容（深度阅读）
4. `search(query, options)` - 智能搜索（支持递归、向量、混合）
5. `ls(uri)` - 文件系统浏览

### 优势

- ✅ 大幅减少 Token 消耗
- ✅ 提高检索效率
- ✅ 按需加载内容
- ✅ 利用新架构全部能力
- ✅ 兼容 OpenViking 设计理念

---

**分析时间**: 2026-02-06 16:05  
**状态**: 待实现  
**优先级**: 高（显著提升 agent 能力）
