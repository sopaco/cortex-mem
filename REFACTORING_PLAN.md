# 🚀 OpenViking 风格工具体系重构实施计划

## 📋 重构范围

完全删除老的工具体系，实现全新的 OpenViking 风格工具。

---

## 🎯 重构清单

### 需要修改的 Crates

| Crate | 当前状态 | 重构内容 | 优先级 |
|-------|---------|---------|--------|
| `cortex-mem-tools` | 老工具 MCP 定义 | 重写为新 API | 🔴 P0 |
| `cortex-mem-rig` | 老工具 Rig 实现 | 重写为新工具 | 🔴 P0 |
| `cortex-mem-tars` | 使用老工具 | 更新为新工具 | 🟡 P1 |
| `cortex-mem-service` | MCP Server | 检查并更新 | 🟡 P1 |

---

## 📐 详细实施步骤

### Phase 1: cortex-mem-tools 重构（核心）

#### 1.1 删除老代码

```bash
# 删除文件
rm cortex-mem-tools/src/mcp_tools.rs

# 需要重写的文件
- operations.rs  # 保留部分，重写工具暴露
- types.rs       # 新增类型定义
- lib.rs         # 更新导出
```

#### 1.2 新增文件

```
cortex-mem-tools/src/
├── lib.rs              # 重写导出
├── errors.rs           # 保留
├── operations.rs       # 重写
├── types.rs            # 新增类型
├── tools/              # 新目录
│   ├── mod.rs
│   ├── tiered.rs       # abstract/overview/read
│   ├── search.rs       # search/find
│   ├── filesystem.rs   # ls/explore
│   └── storage.rs      # store
└── mcp/                # MCP 定义
    ├── mod.rs
    └── definitions.rs  # MCP 工具定义
```

#### 1.3 核心实现

**operations.rs** - 核心操作类:
```rust
use cortex_mem_core::{
    CortexFilesystem, SessionManager, LayerManager,
    RetrievalEngine, VectorSearchEngine,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MemoryOperations {
    pub filesystem: Arc<CortexFilesystem>,
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub layer_manager: Arc<LayerManager>,
    
    #[cfg(feature = "vector-search")]
    pub vector_engine: Option<Arc<VectorSearchEngine>>,
}

impl MemoryOperations {
    // Tiered Access
    pub async fn abstract(&self, uri: &str) -> Result<AbstractResponse>;
    pub async fn overview(&self, uri: &str) -> Result<OverviewResponse>;
    pub async fn read(&self, uri: &str) -> Result<ReadResponse>;
    
    // Search
    pub async fn search(&self, args: SearchArgs) -> Result<SearchResponse>;
    pub async fn find(&self, args: FindArgs) -> Result<FindResponse>;
    
    // Filesystem
    pub async fn ls(&self, args: LsArgs) -> Result<LsResponse>;
    pub async fn explore(&self, args: ExploreArgs) -> Result<ExploreResponse>;
    
    // Storage
    pub async fn store(&self, args: StoreArgs) -> Result<StoreResponse>;
}
```

**tools/tiered.rs** - 分层访问工具:
```rust
impl MemoryOperations {
    pub async fn abstract(&self, uri: &str) -> Result<AbstractResponse> {
        let abstract_text = self.layer_manager
            .get_or_generate_abstract(uri)
            .await?;
        
        Ok(AbstractResponse {
            uri: uri.to_string(),
            abstract_text,
            layer: "L0".to_string(),
            token_count: abstract_text.split_whitespace().count(),
        })
    }
    
    pub async fn overview(&self, uri: &str) -> Result<OverviewResponse> {
        let overview_text = self.layer_manager
            .get_or_generate_overview(uri)
            .await?;
        
        Ok(OverviewResponse {
            uri: uri.to_string(),
            overview_text,
            layer: "L1".to_string(),
            token_count: overview_text.split_whitespace().count(),
        })
    }
    
    pub async fn read(&self, uri: &str) -> Result<ReadResponse> {
        let content = self.filesystem.read(uri).await?;
        
        Ok(ReadResponse {
            uri: uri.to_string(),
            content,
            layer: "L2".to_string(),
            token_count: content.split_whitespace().count(),
            metadata: None,  // TODO: 添加元数据
        })
    }
}
```

**tools/search.rs** - 搜索工具:
```rust
impl MemoryOperations {
    pub async fn search(&self, args: SearchArgs) -> Result<SearchResponse> {
        // 1. 根据 engine 选择检索引擎
        let raw_results = match args.engine.as_deref() {
            Some("vector") => self.vector_search(&args).await?,
            Some("hybrid") => self.hybrid_search(&args).await?,
            _ => self.keyword_search(&args).await?,
        };
        
        // 2. 根据 return_layers 丰富结果
        let results = self.enrich_results(
            raw_results,
            &args.return_layers.unwrap_or(vec!["L0".to_string()])
        ).await?;
        
        Ok(SearchResponse {
            query: args.query,
            results,
            total: results.len(),
            engine_used: args.engine.unwrap_or("keyword".to_string()),
        })
    }
    
    async fn keyword_search(&self, args: &SearchArgs) -> Result<Vec<RawSearchResult>> {
        let engine = RetrievalEngine::new(
            self.filesystem.clone(),
            self.layer_manager.clone()
        );
        
        let options = RetrievalOptions {
            top_k: args.limit.unwrap_or(10),
            ..Default::default()
        };
        
        let scope = args.scope.as_deref().unwrap_or("cortex://threads");
        let result = engine.search(&args.query, scope, options).await?;
        
        Ok(result.results.into_iter().map(|r| RawSearchResult {
            uri: r.uri,
            score: r.score,
        }).collect())
    }
    
    #[cfg(feature = "vector-search")]
    async fn vector_search(&self, args: &SearchArgs) -> Result<Vec<RawSearchResult>> {
        let engine = self.vector_engine.as_ref()
            .ok_or(ToolsError::VectorSearchNotEnabled)?;
        
        let search_options = cortex_mem_core::search::SearchOptions {
            limit: args.limit.unwrap_or(10),
            threshold: 0.5,
            root_uri: args.scope.clone(),
            recursive: args.recursive.unwrap_or(true),
        };
        
        let results = if args.recursive.unwrap_or(true) {
            engine.recursive_search(&args.query, 
                args.scope.as_deref().unwrap_or("cortex://threads"),
                &search_options
            ).await?
        } else {
            engine.semantic_search(&args.query, &search_options).await?
        };
        
        Ok(results.into_iter().map(|r| RawSearchResult {
            uri: r.uri,
            score: r.score,
        }).collect())
    }
    
    async fn enrich_results(
        &self,
        raw_results: Vec<RawSearchResult>,
        return_layers: &[String],
    ) -> Result<Vec<SearchResult>> {
        let mut enriched = Vec::new();
        
        for raw in raw_results {
            let mut result = SearchResult {
                uri: raw.uri.clone(),
                score: raw.score,
                abstract_text: None,
                overview_text: None,
                content: None,
            };
            
            if return_layers.contains(&"L0".to_string()) {
                result.abstract_text = Some(
                    self.layer_manager.get_or_generate_abstract(&raw.uri).await?
                );
            }
            if return_layers.contains(&"L1".to_string()) {
                result.overview_text = Some(
                    self.layer_manager.get_or_generate_overview(&raw.uri).await?
                );
            }
            if return_layers.contains(&"L2".to_string()) {
                result.content = Some(
                    self.filesystem.read(&raw.uri).await?
                );
            }
            
            enriched.push(result);
        }
        
        Ok(enriched)
    }
}
```

**mcp/definitions.rs** - MCP 工具定义:
```rust
pub fn get_mcp_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Tiered Access Tools
        ToolDefinition {
            name: "abstract".to_string(),
            description: "获取内容的 L0 抽象摘要（~100 tokens），用于快速判断相关性".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "内容的 URI"
                    }
                },
                "required": ["uri"]
            }),
        },
        
        ToolDefinition {
            name: "overview".to_string(),
            description: "获取内容的 L1 概览（~2000 tokens），包含核心信息".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "内容的 URI"
                    }
                },
                "required": ["uri"]
            }),
        },
        
        ToolDefinition {
            name: "read".to_string(),
            description: "获取 L2 完整内容，仅在需要详细信息时使用".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "内容的 URI"
                    }
                },
                "required": ["uri"]
            }),
        },
        
        // Search Tools
        ToolDefinition {
            name: "search".to_string(),
            description: "智能搜索记忆，支持关键词/向量/混合检索".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "搜索查询" },
                    "engine": {
                        "type": "string",
                        "enum": ["keyword", "vector", "hybrid"],
                        "description": "检索引擎类型",
                        "default": "keyword"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "是否递归搜索",
                        "default": true
                    },
                    "return_layers": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["L0", "L1", "L2"] },
                        "description": "返回哪些层级",
                        "default": ["L0"]
                    },
                    "scope": {
                        "type": "string",
                        "description": "搜索范围 URI",
                        "default": "cortex://threads"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "最大结果数",
                        "default": 10
                    }
                },
                "required": ["query"]
            }),
        },
        
        // Filesystem Tools
        ToolDefinition {
            name: "ls".to_string(),
            description: "列出目录内容，浏览文件系统结构".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "目录 URI"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "是否递归列出",
                        "default": false
                    },
                    "include_abstracts": {
                        "type": "boolean",
                        "description": "是否包含 L0 摘要",
                        "default": false
                    }
                },
                "required": ["uri"]
            }),
        },
        
        // Storage Tools
        ToolDefinition {
            name: "store".to_string(),
            description: "存储新内容，自动生成分层摘要".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "要存储的内容"
                    },
                    "thread_id": {
                        "type": "string",
                        "description": "线程 ID"
                    },
                    "auto_generate_layers": {
                        "type": "boolean",
                        "description": "是否自动生成 L0/L1 摘要",
                        "default": true
                    }
                },
                "required": ["content", "thread_id"]
            }),
        },
    ]
}
```

---

### Phase 2: cortex-mem-rig 重构

#### 2.1 删除老代码

```bash
rm cortex-mem-rig/src/tool.rs
rm cortex-mem-rig/src/simplified.rs
rm cortex-mem-rig/src/processor.rs
```

#### 2.2 新文件结构

```
cortex-mem-rig/src/
├── lib.rs
├── tools/
│   ├── mod.rs
│   ├── tiered.rs       # AbstractTool, OverviewTool, ReadTool
│   ├── search.rs       # SearchTool, FindTool
│   ├── filesystem.rs   # LsTool, ExploreTool
│   └── storage.rs      # StoreTool
└── types.rs            # 参数类型定义
```

#### 2.3 核心实现

**tools/tiered.rs**:
```rust
pub struct AbstractTool {
    operations: Arc<MemoryOperations>,
}

#[derive(Serialize, Deserialize)]
pub struct AbstractArgs {
    pub uri: String,
}

impl Tool for AbstractTool {
    fn definition(&self) -> ToolDefinition {
        // 从 MCP 定义转换
        let mcp_def = get_mcp_tool_definition("abstract");
        mcp_to_rig_definition(mcp_def)
    }
    
    async fn call(&self, args: &str) -> Result<String, ToolError> {
        let args: AbstractArgs = serde_json::from_str(args)?;
        let response = self.operations.abstract(&args.uri).await?;
        Ok(serde_json::to_string(&response)?)
    }
}

// OverviewTool, ReadTool 类似实现
```

**tools/search.rs**:
```rust
pub struct SearchTool {
    operations: Arc<MemoryOperations>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    pub engine: Option<String>,
    pub recursive: Option<bool>,
    pub return_layers: Option<Vec<String>>,
    pub scope: Option<String>,
    pub limit: Option<usize>,
}

impl Tool for SearchTool {
    fn definition(&self) -> ToolDefinition {
        let mcp_def = get_mcp_tool_definition("search");
        mcp_to_rig_definition(mcp_def)
    }
    
    async fn call(&self, args: &str) -> Result<String, ToolError> {
        let args: SearchArgs = serde_json::from_str(args)?;
        let response = self.operations.search(args.into()).await?;
        Ok(serde_json::to_string(&response)?)
    }
}
```

**lib.rs** - 创建工具集:
```rust
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
}

impl MemoryTools {
    pub fn new(operations: Arc<MemoryOperations>) -> Self {
        Self { operations }
    }
    
    // Tiered Access Tools
    pub fn abstract_tool(&self) -> AbstractTool {
        AbstractTool { operations: self.operations.clone() }
    }
    
    pub fn overview_tool(&self) -> OverviewTool {
        OverviewTool { operations: self.operations.clone() }
    }
    
    pub fn read_tool(&self) -> ReadTool {
        ReadTool { operations: self.operations.clone() }
    }
    
    // Search Tools
    pub fn search_tool(&self) -> SearchTool {
        SearchTool { operations: self.operations.clone() }
    }
    
    pub fn find_tool(&self) -> FindTool {
        FindTool { operations: self.operations.clone() }
    }
    
    // Filesystem Tools
    pub fn ls_tool(&self) -> LsTool {
        LsTool { operations: self.operations.clone() }
    }
    
    pub fn explore_tool(&self) -> ExploreTool {
        ExploreTool { operations: self.operations.clone() }
    }
    
    // Storage Tools
    pub fn store_tool(&self) -> StoreTool {
        StoreTool { operations: self.operations.clone() }
    }
}

pub fn create_memory_tools(operations: Arc<MemoryOperations>) -> MemoryTools {
    MemoryTools::new(operations)
}
```

---

### Phase 3: cortex-mem-tars 更新

#### 3.1 更新 agent.rs

```rust
pub async fn create_memory_agent(
    operations: Arc<MemoryOperations>,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    thread_id: &str,
) -> Result<RigAgent<CompletionModel>> {
    // 创建新的记忆工具
    let memory_tools = create_memory_tools(operations);
    
    let llm_client = Client::builder(api_key)
        .base_url(api_base_url)
        .build();
    
    let system_prompt = format!(r#"你是一个拥有分层记忆功能的智能 AI 助手。

记忆工具说明：
- abstract(uri): 获取 L0 摘要（~100 tokens），快速判断相关性
- overview(uri): 获取 L1 概览（~2000 tokens），理解核心信息
- read(uri): 获取 L2 完整内容，仅在需要详细信息时使用
- search(query, options): 智能搜索记忆，支持关键词/向量/混合检索
- ls(uri): 浏览目录结构
- store(content, thread_id): 存储新内容

使用策略：
1. 优先使用 search 查找相关记忆，默认返回 L0 摘要
2. 根据 L0 摘要判断相关性，需要更多信息时调用 overview
3. 仅在必须了解详细信息时调用 read
4. 使用 ls 探索记忆空间结构
5. 重要信息自动使用 store 存储

当前线程 ID: {thread_id}
"#, thread_id = thread_id);
    
    let agent = llm_client
        .completion_model(model)
        .into_agent_builder()
        // 注册新的分层工具
        .tool(memory_tools.search_tool())        // 主要搜索工具
        .tool(memory_tools.abstract_tool())      // L0 访问
        .tool(memory_tools.overview_tool())      // L1 访问
        .tool(memory_tools.read_tool())          // L2 访问
        .tool(memory_tools.ls_tool())            // 浏览目录
        .tool(memory_tools.store_tool())         // 存储
        .preamble(&system_prompt)
        .build();
    
    Ok(agent)
}
```

---

## 📊 重构影响分析

### 删除的代码

| 文件 | 大小 | 状态 |
|------|------|------|
| `cortex-mem-tools/src/mcp_tools.rs` | 8.18 KB | ❌ 删除 |
| `cortex-mem-rig/src/tool.rs` | 11.89 KB | ❌ 删除 |
| `cortex-mem-rig/src/simplified.rs` | 3.77 KB | ❌ 删除 |
| `cortex-mem-rig/src/processor.rs` | 1.82 KB | ❌ 删除 |
| **总计** | ~26 KB | ❌ 删除 |

### 新增的代码

| 文件 | 预估大小 | 状态 |
|------|---------|------|
| `cortex-mem-tools/src/tools/*` | ~15 KB | ✅ 新增 |
| `cortex-mem-tools/src/mcp/*` | ~5 KB | ✅ 新增 |
| `cortex-mem-rig/src/tools/*` | ~12 KB | ✅ 新增 |
| **总计** | ~32 KB | ✅ 新增 |

---

## ⚠️ 破坏性变更

### API 变更

| 老 API | 新 API | 说明 |
|--------|--------|------|
| `store_memory` | `store` | 简化命名 |
| `query_memory` | `search` | 增强功能 |
| `list_memories` | `ls` + `search` | 分离职责 |
| `get_memory` | `read` | 语义化命名 |

### 工具数量变化

- 老工具: 4 个（store_memory, query_memory, list_memories, get_memory）
- 新工具: 8 个（abstract, overview, read, search, find, ls, explore, store）

---

## ✅ 验证清单

- [ ] cortex-mem-tools 编译通过
- [ ] cortex-mem-rig 编译通过
- [ ] cortex-mem-tars 编译通过
- [ ] MCP Server 正常工作
- [ ] Agent 能正确调用新工具
- [ ] 分层加载功能正常
- [ ] 向量搜索功能正常（如果启用）
- [ ] 关键词搜索功能正常

---

## 🎯 下一步

**需要用户确认**:
1. 是否现在开始执行这个重构？
2. 是否需要先实现某一个 Phase？
3. 是否有特殊要求或调整？

---

**计划时间**: 2026-02-06 16:30  
**状态**: 等待执行  
**预计工作量**: 2-3 小时
