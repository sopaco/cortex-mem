# 检索引擎模块 (Search)

**模块路径**: `cortex-mem-core/src/search/`  
**职责**: 智能检索、全文搜索、向量语义搜索

---

## 核心组件

### VectorSearchEngine

```rust
pub struct VectorSearchEngine {
    filesystem: Arc<CortexFilesystem>,
    // Qdrant集成（可选feature）
    #[cfg(feature = "vector-search")]
    qdrant: Arc<QdrantVectorStore>,
}
```

**主要方法**:
- `search()` - 执行搜索
- `search_filesystem()` - 文件系统搜索
- `search_vector()` - 向量语义搜索（可选）

### SearchOptions

```rust
pub struct SearchOptions {
    pub limit: usize,
    pub threshold: f32,
    pub root_uri: Option<String>,
    pub recursive: bool,
}
```

## 搜索策略

### 1. 文件系统全文搜索

**优势**:
- ✅ 快速、准确
- ✅ 零外部依赖
- ✅ 支持正则表达式

**实现**:
```rust
async fn search_filesystem(&self, query: &str) -> Result<Vec<SearchResult>> {
    // 1. 列出所有.md文件
    // 2. 并发读取文件内容
    // 3. 文本匹配
    // 4. 计算相关性分数
    // 5. 排序和截断
}
```

**相关性计算**:
- 关键词出现次数
- 位置权重（标题 > 正文）
- 最近访问时间（未来）

### 2. 向量语义搜索（可选）

**优势**:
- ✅ 语义理解
- ✅ 支持模糊查询
- ✅ 跨语言搜索

**依赖**: Qdrant向量数据库

**实现**:
```rust
#[cfg(feature = "vector-search")]
async fn search_vector(&self, query: &str) -> Result<Vec<ScoredMemory>> {
    // 1. 查询向量化（embedding）
    // 2. Qdrant相似度搜索
    // 3. 结果映射回文件URI
    // 4. 加载文件内容
}
```

### 3. 混合搜索

**策略**: 结合文件系统和向量搜索

```rust
async fn hybrid_search(&self, query: &str) -> Result<Vec<SearchResult>> {
    // 1. 并行执行两种搜索
    let (fs_results, vec_results) = tokio::join!(
        self.search_filesystem(query),
        self.search_vector(query)
    );
    
    // 2. 结果合并去重
    // 3. 重新评分排序
    // 4. 返回top-k
}
```

## 搜索流程

```
用户查询 "如何实现OAuth"
    ↓
查询预处理
  - 分词: ["如何", "实现", "OAuth"]
  - 去除停用词: ["实现", "OAuth"]
  - 扩展同义词: ["实现", "OAuth", "授权"]
    ↓
┌─────────────────┬─────────────────┐
│ 文件系统搜索     │   向量搜索       │
│ (关键词匹配)     │  (语义相似度)    │
└────────┬────────┴────────┬────────┘
         │                 │
         └────────┬────────┘
                  ↓
           结果合并
             - 去重
             - 评分
             - 排序
                  ↓
           L0筛选
             - 快速浏览摘要
             - 过滤不相关
                  ↓
           L1加载
             - 详细上下文
             - 相关性评估
                  ↓
           L2返回
             - 完整内容
             - 返回给用户
```

## 索引优化

### 全文索引（Tantivy）

**未来计划**: 使用Tantivy构建倒排索引

```rust
struct FulltextIndex {
    index: tantivy::Index,
    schema: Schema,
}
```

**优势**:
- 更快的搜索速度
- 支持复杂查询语法
- 增量索引更新

### 向量索引（Qdrant）

**配置**:
```rust
let qdrant_config = QdrantConfig {
    url: "http://localhost:6333",
    collection_name: "cortex_memories",
    embedding_dim: Some(1536),  // OpenAI ada-002
    timeout_secs: 10,
};
```

## SearchResult结构

```rust
pub struct SearchResult {
    pub uri: String,           // cortex:// URI
    pub score: f32,            // 相关性分数 0.0-1.0
    pub snippet: String,       // 摘要片段
    pub content: Option<String>, // 完整内容（可选）
    pub metadata: SearchMetadata,
}

pub struct SearchMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,        // "filesystem" | "vector"
}
```

## 性能优化

### 1. 并发搜索

```rust
async fn search_parallel(&self, query: &str) -> Result<Vec<SearchResult>> {
    let files = self.list_files().await?;
    
    // 并发读取和搜索
    let results = stream::iter(files)
        .map(|file| async move {
            self.search_single_file(&file, query).await
        })
        .buffer_unordered(10)  // 并发度10
        .collect::<Vec<_>>()
        .await;
    
    // 合并结果
}
```

### 2. 缓存机制（未来）

```rust
struct SearchCache {
    cache: Arc<RwLock<LruCache<String, Vec<SearchResult>>>>,
    ttl: Duration,
}
```

### 3. 增量索引（未来）

```rust
struct IncrementalIndexer {
    last_indexed: DateTime<Utc>,
    index_queue: Arc<RwLock<VecDeque<String>>>,
}
```

## 查询语法（未来）

```
# 基础查询
OAuth

# 短语查询
"OAuth 2.0"

# 布尔查询
OAuth AND authorization
OAuth OR OIDC
OAuth NOT deprecated

# 字段查询
title:OAuth
content:implementation
role:user

# 范围查询
created_at:[2026-01-01 TO 2026-02-01]

# 通配符
Oa*th
auth?

# 模糊查询
OAuth~2
```

## 配置示例

```rust
let search_options = SearchOptions {
    limit: 10,              // 返回top 10结果
    threshold: 0.5,         // 最低相关性0.5
    root_uri: Some("cortex://threads/my-chat/".to_string()),
    recursive: true,        // 递归搜索子目录
};

let results = engine.search("OAuth implementation", &search_options).await?;
```

---

详见源码: [cortex-mem-core/src/search/](../../cortex-mem-core/src/search/)
