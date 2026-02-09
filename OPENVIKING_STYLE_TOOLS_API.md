# 🔄 OpenViking 风格工具 API 设计规范

## 📋 新工具体系设计

完全遵循 OpenViking 的设计理念，实现 Tiered Context Loading 和文件系统式 API。

---

## 🎯 核心工具清单

### 1. Tiered Access Tools（分层访问工具）

| 工具 | 功能 | 返回层级 | Token 消耗 | 用途 |
|------|------|---------|----------|------|
| `abstract` | 获取 L0 摘要 | L0 | ~100 tokens | 快速过滤、相关性判断 |
| `overview` | 获取 L1 概览 | L1 | ~2000 tokens | 规划决策、理解上下文 |
| `read` | 获取 L2 完整内容 | L2 | 完整内容 | 深度阅读、详细信息 |

---

### 2. Search Tools（搜索工具）

| 工具 | 功能 | 支持选项 |
|------|------|---------|
| `search` | 智能搜索 | engine (keyword/vector/hybrid), recursive, return_layers |
| `find` | 简单查找 | 关键词匹配 |

---

### 3. Filesystem Tools（文件系统工具）

| 工具 | 功能 | 参数 |
|------|------|------|
| `ls` | 列出目录内容 | uri, recursive |
| `explore` | 探索记忆空间 | query, start_uri |

---

### 4. Storage Tools（存储工具）

| 工具 | 功能 | 说明 |
|------|------|------|
| `store` | 存储内容 | 自动生成 L0/L1 摘要 |

---

## 📐 API 详细设计

### 1. abstract - 获取 L0 摘要

**用途**: 快速判断内容相关性，用于过滤和初筛。

**输入**:
```json
{
  "uri": "cortex://threads/{thread_id}/timeline/{year}-{month}/{day}/{timestamp}_{id}.md"
}
```

**输出**:
```json
{
  "uri": "cortex://threads/...",
  "abstract": "用户询问了关于 Python 装饰器的使用方法。",
  "layer": "L0",
  "token_count": 15
}
```

**实现**:
```rust
pub async fn abstract(&self, uri: &str) -> Result<AbstractResponse> {
    let layer_manager = LayerManager::new(self.filesystem.clone());
    let abstract_text = layer_manager.get_or_generate_abstract(uri).await?;
    
    Ok(AbstractResponse {
        uri: uri.to_string(),
        abstract_text,
        layer: "L0".to_string(),
        token_count: abstract_text.split_whitespace().count(),
    })
}
```

---

### 2. overview - 获取 L1 概览

**用途**: 理解内容核心信息和上下文，用于规划和决策。

**输入**:
```json
{
  "uri": "cortex://threads/{thread_id}/timeline/{year}-{month}/{day}/{timestamp}_{id}.md"
}
```

**输出**:
```json
{
  "uri": "cortex://threads/...",
  "overview": "## 核心内容\n用户询问 Python 装饰器...\n\n## 关键点\n1. 装饰器语法\n2. 常见用例\n\n## 使用场景\n适合学习 Python 高级特性的开发者",
  "layer": "L1",
  "token_count": 180
}
```

**实现**:
```rust
pub async fn overview(&self, uri: &str) -> Result<OverviewResponse> {
    let layer_manager = LayerManager::new(self.filesystem.clone());
    let overview_text = layer_manager.get_or_generate_overview(uri).await?;
    
    Ok(OverviewResponse {
        uri: uri.to_string(),
        overview_text,
        layer: "L1".to_string(),
        token_count: overview_text.split_whitespace().count(),
    })
}
```

---

### 3. read - 获取 L2 完整内容

**用途**: 深度阅读完整信息，仅在必要时使用。

**输入**:
```json
{
  "uri": "cortex://threads/{thread_id}/timeline/{year}-{month}/{day}/{timestamp}_{id}.md"
}
```

**输出**:
```json
{
  "uri": "cortex://threads/...",
  "content": "# 用户询问\n\n用户：什么是 Python 装饰器？\n\n# Assistant 回答\n\nPython 装饰器是一种设计模式...",
  "layer": "L2",
  "token_count": 1523,
  "metadata": {
    "created_at": "2026-02-06T08:00:00Z",
    "updated_at": "2026-02-06T08:00:00Z"
  }
}
```

**实现**:
```rust
pub async fn read(&self, uri: &str) -> Result<ReadResponse> {
    let content = self.filesystem.read(uri).await?;
    let metadata = self.filesystem.metadata(uri).await?;
    
    Ok(ReadResponse {
        uri: uri.to_string(),
        content,
        layer: "L2".to_string(),
        token_count: content.split_whitespace().count(),
        metadata: Some(FileMetadata {
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
        }),
    })
}
```

---

### 4. search - 智能搜索

**用途**: 强大的搜索功能，支持多种检索引擎和递归搜索。

**输入**:
```json
{
  "query": "Python 装饰器使用方法",
  "engine": "hybrid",           // "keyword" | "vector" | "hybrid"
  "recursive": true,            // 是否递归搜索子目录
  "return_layers": ["L0", "L1"], // 返回哪些层级
  "scope": "cortex://threads/{thread_id}",  // 搜索范围
  "limit": 10
}
```

**输出**:
```json
{
  "query": "Python 装饰器使用方法",
  "results": [
    {
      "uri": "cortex://threads/.../message_001.md",
      "score": 0.92,
      "abstract": "用户询问了关于 Python 装饰器的使用方法。",
      "overview": "## 核心内容\n用户询问 Python 装饰器...",
      "content": null  // L2 未请求
    },
    {
      "uri": "cortex://threads/.../message_002.md",
      "score": 0.85,
      "abstract": "讨论了装饰器的常见应用场景。",
      "overview": null,  // 如果只请求 L0
      "content": null
    }
  ],
  "total": 2,
  "engine_used": "hybrid"
}
```

**实现**:
```rust
pub async fn search(&self, args: SearchArgs) -> Result<SearchResponse> {
    // 1. 根据 engine 选择检索引擎
    let raw_results = match args.engine.as_deref() {
        Some("vector") => self.vector_search(&args).await?,
        Some("hybrid") => self.hybrid_search(&args).await?,
        _ => self.keyword_search(&args).await?,
    };
    
    // 2. 根据 return_layers 加载对应层级
    let results = self.enrich_results(raw_results, &args.return_layers).await?;
    
    Ok(SearchResponse {
        query: args.query,
        results,
        total: results.len(),
        engine_used: args.engine.unwrap_or("keyword".to_string()),
    })
}

async fn enrich_results(
    &self,
    raw_results: Vec<RawSearchResult>,
    return_layers: &[String],
) -> Result<Vec<SearchResult>> {
    let layer_manager = LayerManager::new(self.filesystem.clone());
    let mut enriched = Vec::new();
    
    for raw in raw_results {
        let mut result = SearchResult {
            uri: raw.uri.clone(),
            score: raw.score,
            abstract: None,
            overview: None,
            content: None,
        };
        
        // 按需加载层级
        if return_layers.contains(&"L0".to_string()) {
            result.abstract = Some(
                layer_manager.get_or_generate_abstract(&raw.uri).await?
            );
        }
        if return_layers.contains(&"L1".to_string()) {
            result.overview = Some(
                layer_manager.get_or_generate_overview(&raw.uri).await?
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
```

---

### 5. find - 简单查找

**用途**: 快速查找，返回 L0 摘要。

**输入**:
```json
{
  "query": "装饰器",
  "scope": "cortex://threads/{thread_id}",
  "limit": 5
}
```

**输出**:
```json
{
  "query": "装饰器",
  "results": [
    {
      "uri": "cortex://threads/.../message_001.md",
      "abstract": "用户询问了关于 Python 装饰器的使用方法。"
    }
  ],
  "total": 1
}
```

---

### 6. ls - 列出目录

**用途**: 浏览文件系统结构，探索记忆空间。

**输入**:
```json
{
  "uri": "cortex://threads/{thread_id}/timeline",
  "recursive": false,
  "include_abstracts": true  // 是否包含 L0 摘要
}
```

**输出**:
```json
{
  "uri": "cortex://threads/{thread_id}/timeline",
  "entries": [
    {
      "name": "2026-02",
      "uri": "cortex://threads/{thread_id}/timeline/2026-02",
      "is_directory": true,
      "child_count": 6
    },
    {
      "name": "2026-01",
      "uri": "cortex://threads/{thread_id}/timeline/2026-01",
      "is_directory": true,
      "child_count": 15,
      "abstract": "1月的对话主要集中在 Python 项目开发"  // 如果请求
    }
  ],
  "total": 2
}
```

**实现**:
```rust
pub async fn ls(&self, args: LsArgs) -> Result<LsResponse> {
    let entries = self.filesystem.list(&args.uri).await?;
    let layer_manager = LayerManager::new(self.filesystem.clone());
    
    let mut result_entries = Vec::new();
    for entry in entries {
        let mut result_entry = LsEntry {
            name: entry.name,
            uri: entry.uri.clone(),
            is_directory: entry.is_directory,
            child_count: if entry.is_directory {
                Some(self.filesystem.list(&entry.uri).await?.len())
            } else {
                None
            },
            abstract: None,
        };
        
        // 如果请求摘要且是文件
        if args.include_abstracts && !entry.is_directory {
            result_entry.abstract = Some(
                layer_manager.get_or_generate_abstract(&entry.uri).await?
            );
        }
        
        result_entries.push(result_entry);
    }
    
    Ok(LsResponse {
        uri: args.uri,
        entries: result_entries,
        total: result_entries.len(),
    })
}
```

---

### 7. explore - 探索记忆空间

**用途**: 智能探索，结合搜索和浏览。

**输入**:
```json
{
  "query": "Python 相关的对话",
  "start_uri": "cortex://threads",
  "max_depth": 3,
  "return_layers": ["L0"]
}
```

**输出**:
```json
{
  "query": "Python 相关的对话",
  "exploration_path": [
    {
      "uri": "cortex://threads/{thread_id}",
      "relevance_score": 0.95,
      "abstract": "与用户讨论 Python 开发的线程"
    },
    {
      "uri": "cortex://threads/{thread_id}/timeline/2026-02",
      "relevance_score": 0.88,
      "abstract": "2月份的 Python 相关讨论"
    }
  ],
  "matches": [ /* 匹配的具体文件 */ ],
  "total_explored": 45,
  "total_matches": 12
}
```

---

### 8. store - 存储内容

**用途**: 存储新内容，自动生成 L0/L1 摘要。

**输入**:
```json
{
  "content": "# 用户询问\n\n用户：什么是 Python 装饰器？\n\n# Assistant 回答\n\n...",
  "thread_id": "{thread_id}",
  "metadata": {
    "tags": ["python", "装饰器"],
    "importance": 0.8
  },
  "auto_generate_layers": true  // 自动生成 L0/L1
}
```

**输出**:
```json
{
  "uri": "cortex://threads/{thread_id}/timeline/2026-02/06/08_15_30_abc123.md",
  "layers_generated": {
    "L0": "cortex://threads/{thread_id}/timeline/2026-02/06/.layers/08_15_30_abc123/abstract.txt",
    "L1": "cortex://threads/{thread_id}/timeline/2026-02/06/.layers/08_15_30_abc123/overview.md"
  },
  "success": true
}
```

**实现**:
```rust
pub async fn store(&self, args: StoreArgs) -> Result<StoreResponse> {
    let session_manager = self.session_manager.read().await;
    
    // 1. 存储消息
    let message = Message::new(MessageRole::User, &args.content);
    let message_uri = session_manager
        .message_storage()
        .save_message(&args.thread_id, &message)
        .await?;
    
    // 2. 自动生成分层摘要
    let mut layers_generated = HashMap::new();
    if args.auto_generate_layers.unwrap_or(true) {
        let layer_manager = LayerManager::new(self.filesystem.clone());
        
        // 生成 L0
        let abstract_uri = layer_manager
            .generate_and_save_abstract(&message_uri, &args.content)
            .await?;
        layers_generated.insert("L0".to_string(), abstract_uri);
        
        // 生成 L1
        let overview_uri = layer_manager
            .generate_and_save_overview(&message_uri, &args.content)
            .await?;
        layers_generated.insert("L1".to_string(), overview_uri);
    }
    
    Ok(StoreResponse {
        uri: message_uri,
        layers_generated,
        success: true,
    })
}
```

---

## 📊 工具使用场景

### 场景 1: Agent 快速过滤记忆

```
1. Agent 收到用户问题："我之前问过什么关于 Python 的问题？"
2. Agent 调用 search(query="Python", return_layers=["L0"], limit=20)
3. 获取 20 个 L0 摘要（~100 tokens each）
4. Agent 快速判断相关性，筛选出 3 个相关记忆
5. Agent 调用 overview() 获取这 3 个的 L1 概览
6. Agent 总结回答用户
```

**Token 消耗**: 
- 20 x 100 (L0) + 3 x 2000 (L1) = 8,000 tokens
- 如果全用 L2: 20 x 5000 = 100,000 tokens (节省 92%)

---

### 场景 2: Agent 探索记忆空间

```
1. Agent 需要了解用户的对话历史
2. Agent 调用 ls("cortex://threads/{thread_id}/timeline", recursive=false, include_abstracts=true)
3. 看到多个月份目录及其 L0 摘要
4. Agent 选择相关月份，调用 ls() 深入
5. Agent 调用 overview() 获取关键对话的 L1
6. Agent 综合信息，理解用户背景
```

---

### 场景 3: Agent 深度阅读

```
1. Agent 需要详细了解某次对话
2. 已通过 search 找到 URI
3. Agent 调用 read(uri) 获取完整内容
4. Agent 分析详细信息
5. Agent 提供精确回答
```

---

## 🎯 与 OpenViking 的对应关系

| OpenViking API | Cortex-Mem API | 功能 |
|---------------|----------------|------|
| `client.abstract(uri)` | `abstract(uri)` | ✅ 完全对应 |
| `client.overview(uri)` | `overview(uri)` | ✅ 完全对应 |
| `client.read(uri)` | `read(uri)` | ✅ 完全对应 |
| `client.search(query, session)` | `search(query, options)` | ✅ 增强版本 |
| `client.find(query)` | `find(query, scope)` | ✅ 对应 |
| `client.ls(uri)` | `ls(uri, options)` | ✅ 增强版本 |
| - | `explore(query, start_uri)` | ✅ 额外功能 |
| - | `store(content, metadata)` | ✅ 额外功能 |

---

## 📐 工具定义（MCP 格式）

### abstract

```json
{
  "name": "abstract",
  "description": "获取内容的 L0 抽象摘要（~100 tokens），用于快速判断相关性",
  "inputSchema": {
    "type": "object",
    "properties": {
      "uri": {
        "type": "string",
        "description": "内容的 URI（如 cortex://threads/{thread_id}/...）"
      }
    },
    "required": ["uri"]
  }
}
```

### overview

```json
{
  "name": "overview",
  "description": "获取内容的 L1 概览（~2000 tokens），包含核心信息和使用场景",
  "inputSchema": {
    "type": "object",
    "properties": {
      "uri": {
        "type": "string",
        "description": "内容的 URI"
      }
    },
    "required": ["uri"]
  }
}
```

### read

```json
{
  "name": "read",
  "description": "获取 L2 完整内容，仅在需要详细信息时使用",
  "inputSchema": {
    "type": "object",
    "properties": {
      "uri": {
        "type": "string",
        "description": "内容的 URI"
      }
    },
    "required": ["uri"]
  }
}
```

### search

```json
{
  "name": "search",
  "description": "智能搜索记忆，支持关键词/向量/混合检索和递归搜索",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "搜索查询"
      },
      "engine": {
        "type": "string",
        "enum": ["keyword", "vector", "hybrid"],
        "description": "检索引擎类型",
        "default": "keyword"
      },
      "recursive": {
        "type": "boolean",
        "description": "是否递归搜索子目录",
        "default": true
      },
      "return_layers": {
        "type": "array",
        "items": {
          "type": "string",
          "enum": ["L0", "L1", "L2"]
        },
        "description": "返回哪些层级的内容",
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
  }
}
```

### find

```json
{
  "name": "find",
  "description": "快速查找内容，返回 L0 摘要",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "查找关键词"
      },
      "scope": {
        "type": "string",
        "description": "查找范围 URI"
      },
      "limit": {
        "type": "integer",
        "description": "最大结果数",
        "default": 5
      }
    },
    "required": ["query"]
  }
}
```

### ls

```json
{
  "name": "ls",
  "description": "列出目录内容，浏览文件系统结构",
  "inputSchema": {
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
  }
}
```

### explore

```json
{
  "name": "explore",
  "description": "智能探索记忆空间，结合搜索和浏览",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "探索查询"
      },
      "start_uri": {
        "type": "string",
        "description": "起始 URI",
        "default": "cortex://threads"
      },
      "max_depth": {
        "type": "integer",
        "description": "最大探索深度",
        "default": 3
      },
      "return_layers": {
        "type": "array",
        "items": {
          "type": "string",
          "enum": ["L0", "L1", "L2"]
        },
        "description": "返回哪些层级",
        "default": ["L0"]
      }
    },
    "required": ["query"]
  }
}
```

### store

```json
{
  "name": "store",
  "description": "存储新内容，自动生成分层摘要",
  "inputSchema": {
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
      "metadata": {
        "type": "object",
        "description": "元数据（标签、重要性等）"
      },
      "auto_generate_layers": {
        "type": "boolean",
        "description": "是否自动生成 L0/L1 摘要",
        "default": true
      }
    },
    "required": ["content", "thread_id"]
  }
}
```

---

## 📝 总结

### 核心设计原则

1. ✅ **Progressive Disclosure**: L0 → L1 → L2 渐进式加载
2. ✅ **Filesystem Paradigm**: 文件系统式 URI 和 API
3. ✅ **Intelligent Search**: 支持多种检索引擎
4. ✅ **Minimal Token Consumption**: 只加载必要的层级

### 与 OpenViking 的一致性

- ✅ L0/L1/L2 分层完全对应
- ✅ abstract/overview/read API 完全对应
- ✅ search/find/ls 文件系统式 API
- ✅ Progressive Disclosure 模式

### 优势

- Token 消耗减少 80-90%
- 检索效率大幅提升
- Agent 可以智能探索记忆空间
- 支持向量检索、关键词检索、混合检索

---

**设计时间**: 2026-02-06 16:20  
**状态**: API 规范完成，待实现  
**下一步**: 实现 MCP 工具和 Rig 工具
