# Cortex Memory 功能流程文档

本文档详细介绍 Cortex Memory 的核心功能流程和使用场景。

**版本**: V2.0.0  
**最后更新**: 2026-02-12

---

## 目录

1. [会话管理流程](#1-会话管理流程)
2. [消息存储流程](#2-消息存储流程)
3. [三层抽象生成流程](#3-三层抽象生成流程)
4. [记忆提取流程](#4-记忆提取流程)
5. [检索流程](#5-检索流程)
6. [搜索流程](#6-搜索流程)
7. [租户隔离流程](#7-租户隔离流程)

---

## 1. 会话管理流程

### 1.1 创建会话

```
用户请求
    │
    ▼
┌─────────────────┐
│  SessionManager │
│  create_session │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 生成会话元数据   │
│ - id            │
│ - title         │
│ - status: Active│
│ - created_at    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ CortexFilesystem│
│ write()         │
└────────┬────────┘
         │
         ▼
cortex://session/{id}/session.md
```

**代码示例**:

```rust
use cortex_mem_core::{SessionManager, SessionConfig};

let config = SessionConfig::default();
let manager = SessionManager::new(filesystem, config);

// 创建会话
let session = manager.create_session(
    "my-session",
    Some("技术讨论".to_string()),
    None, // tags
    None, // participants
).await?;
```

### 1.2 会话状态流转

```
┌─────────┐    close     ┌─────────┐    archive    ┌──────────┐
│ Active  │ ───────────► │ Closed  │ ───────────►  │ Archived │
└────┬────┘              └────┬────┘               └──────────┘
     │                        │
     │ delete                 │ delete
     ▼                        ▼
  ┌──────┐                 ┌──────┐
  │ 删除  │                 │ 删除  │
  └──────┘                 └──────┘
```

**状态说明**:

| 状态 | 说明 | 可执行操作 |
|------|------|------------|
| Active | 活跃状态，可添加消息 | add, close, delete |
| Closed | 已关闭，只读 | archive, delete |
| Archived | 已归档，长期存储 | delete |

### 1.3 参与者管理

```rust
use cortex_mem_core::{Participant, ParticipantRole};

// 添加参与者
let participant = Participant::new(
    "user-123",
    "张三",
    ParticipantRole::User,
);
manager.add_participant("session-id", participant).await?;
```

---

## 2. 消息存储流程

### 2.1 添加消息

```
用户输入
    │
    ▼
┌─────────────────┐
│ 创建 Message    │
│ - id            │
│ - role          │
│ - content       │
│ - timestamp     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ MessageStorage  │
│   store()       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ TimelineGenerator│
│  生成时间轴条目  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ CortexFilesystem│
│   write()       │
└────────┬────────┘
         │
         ▼
cortex://session/{id}/timeline/{timestamp}.md
```

**代码示例**:

```rust
use cortex_mem_core::{Message, MessageRole};

// 添加用户消息
let message = Message::new_user("Hello, how are you?");
manager.add_message("session-id", message).await?;

// 添加助手消息
let message = Message::new_assistant("I'm doing well, thank you!");
manager.add_message("session-id", message).await?;
```

### 2.2 Timeline 组织

消息按时间组织在 Timeline 目录中：

```
cortex://session/{session_id}/
├── session.md           # 会话元数据
└── timeline/
    ├── 2026-02-12/
    │   ├── 10-00-00_user.md
    │   ├── 10-00-05_assistant.md
    │   └── 10-05-00_user.md
    └── 2026-02-13/
        └── 09-00-00_user.md
```

---

## 3. 三层抽象生成流程

### 3.1 L0/L1/L2 生成

```
原始内容 (L2)
    │
    ├─────────────────────────────────────┐
    │                                     │
    ▼                                     ▼
┌─────────────────┐              ┌─────────────────┐
│ AbstractGenerator│            │ OverviewGenerator│
│  (L0 生成)       │            │  (L1 生成)       │
│                 │            │                 │
│ 有 LLM?         │            │ 有 LLM?         │
│ ├─ 是: LLM生成  │            │ ├─ 是: LLM生成  │
│ └─ 否: 规则生成 │            │ └─ 否: 规则生成 │
└────────┬────────┘            └────────┬────────┘
         │                             │
         ▼                             ▼
    L0 Abstract (~100 tokens)    L1 Overview (~2000 tokens)
         │                             │
         └─────────────┬───────────────┘
                       ▼
              ┌─────────────────┐
              │ CortexFilesystem│
              │   write()       │
              └─────────────────┘
```

### 3.2 生成策略

**有 LLM 时**:

```rust
// 使用 LLM 生成高质量摘要
let abstract = llm_client.generate(
    format!("Summarize in 100 tokens: {}", content)
).await?;

let overview = llm_client.generate(
    format!("Summarize in 2000 tokens: {}", content)
).await?;
```

**无 LLM 时**:

```rust
// 使用规则生成
let abstract = rule_based_abstract(&content);  // 取前100 tokens
let overview = rule_based_overview(&content);  // 取前2000 tokens
```

### 3.3 文件存储结构

```
cortex://user/{user_id}/memories/
├── my-memory.md              # L2: 完整内容
├── my-memory.L0.md           # L0: 摘要 (~100 tokens)
└── my-memory.L1.md           # L1: 概览 (~2000 tokens)
```

### 3.4 分层读取

```rust
use cortex_mem_core::{LayerManager, ContextLayer};

let layer_manager = LayerManager::new(filesystem);

// 读取不同层级
let l0 = layer_manager.load(uri, ContextLayer::L0Abstract).await?;  // ~100 tokens
let l1 = layer_manager.load(uri, ContextLayer::L1Overview).await?;  // ~2000 tokens
let l2 = layer_manager.load(uri, ContextLayer::L2Detail).await?;    // 完整内容
```

---

## 4. 记忆提取流程

### 4.1 自动提取流程

```
Session Messages
    │
    ▼
┌─────────────────┐
│ MemoryExtractor │
│   extract()     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 构建 Prompt     │
│ 包含消息历史    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   LLM Client    │
│  (rig-core)     │
│   generate()    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 解析 LLM 响应   │
│ 提取结构化数据  │
└────────┬────────┘
         │
    ┌────┴────┬──────────┬──────────┐
    ▼         ▼          ▼          ▼
 Facts    Decisions   Entities   UserProfile
    │         │          │          │
    └─────────┴────┬─────┴──────────┘
                   ▼
          ┌─────────────────┐
          │ CortexFilesystem│
          │   write()       │
          └─────────────────┘
```

### 4.2 提取内容类型

```rust
pub struct ExtractedMemories {
    pub facts: Vec<ExtractedFact>,
    pub decisions: Vec<ExtractedDecision>,
    pub entities: Vec<ExtractedEntity>,
    pub user_profile: UserProfile,
}

pub struct ExtractedFact {
    pub content: String,
    pub importance: MemoryImportance,
    pub confidence: f32,
}

pub struct UserProfile {
    pub preferences: Vec<String>,
    pub interests: Vec<String>,
    pub goals: Vec<String>,
}
```

### 4.3 存储位置

```
cortex://user/{user_id}/
├── memories/
│   ├── facts/
│   │   └── {fact_id}.md
│   ├── decisions/
│   │   └── {decision_id}.md
│   └── entities/
│       └── {entity_id}.md
└── profile.md              # 用户画像
```

### 4.4 触发方式

**手动触发**:

```bash
# CLI
cortex-mem session extract <session-id>
```

```rust
// 代码
extractor.extract_session("session-id").await?;
```

**自动触发**:

```rust
use cortex_mem_core::{AutoExtractor, AutoExtractConfig};

let config = AutoExtractConfig {
    trigger_on_message_count: 10,  // 每10条消息触发
    extract_interval: Duration::from_secs(3600),  // 或每小时
};

let auto_extractor = AutoExtractor::new(filesystem, llm_client, config);
auto_extractor.start().await?;
```

---

## 5. 检索流程

### 5.1 意图分析

```
Query: "Find my OAuth implementation notes from last week"
    │
    ▼
┌─────────────────┐
│ IntentAnalyzer  │
└────────┬────────┘
         │
         ▼
Intent {
    keywords: ["OAuth", "implementation", "notes"],
    entities: ["OAuth"],
    time_range: Some("last week"),
    filters: Filters {
        dimension: Some("user"),
        layer: Some(L1Overview),
    },
}
```

### 5.2 递归检索

```
Intent
    │
    ▼
┌─────────────────┐
│ RetrievalEngine │
│   retrieve()    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│  1. 搜索索引    │────►│ 获取候选文件    │
│  (tantivy)      │     │ (按相关性排序)  │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│ 2. 加载 L0 层   │────►│ 快速筛选        │
│ (100 tokens)    │     │ (排除不相关)    │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│ 3. 加载 L1 层   │────►│ 详细评估        │
│ (2000 tokens)   │     │ (计算相关性)    │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│ 4. 按需加载 L2  │────►│ 最终排序        │
│ (完整内容)      │     │ 返回结果        │
└─────────────────┘     └─────────────────┘
```

### 5.3 相关性计算

```rust
pub struct RelevanceCalculator;

impl RelevanceCalculator {
    pub fn calculate(
        &self,
        intent: &Intent,
        content: &str,
        metadata: &FileMetadata,
    ) -> f32 {
        let keyword_score = self.keyword_match(intent.keywords(), content);
        let semantic_score = self.semantic_similarity(intent.embedding(), content);
        let time_score = self.time_relevance(intent.time_range(), metadata.created_at());
        let importance_score = metadata.importance() as f32 / 10.0;
        
        // 加权组合
        keyword_score * 0.3 +
        semantic_score * 0.4 +
        time_score * 0.2 +
        importance_score * 0.1
    }
}
```

---

## 6. 搜索流程

### 6.1 Filesystem 搜索

```
Query: "OAuth"
    │
    ▼
┌─────────────────┐
│ FilesystemSearch│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 遍历目录结构    │
│ walkdir         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 正则匹配内容    │
│ regex::Regex    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 返回匹配文件    │
│ 按文件名排序    │
└─────────────────┘
```

**特点**:
- 零外部依赖
- 精确匹配
- 快速启动

### 6.2 Vector 搜索

```
Query: "authentication methods"
    │
    ▼
┌─────────────────┐
│ EmbeddingClient │
│   embed()       │
└────────┬────────┘
         │
         ▼
Query Embedding: [0.1, -0.2, 0.3, ...] (1536 dims)
         │
         ▼
┌─────────────────┐
│ QdrantVectorStore│
│   search()      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Qdrant Server   │
│ 向量相似度搜索  │
└────────┬────────┘
         │
         ▼
Similar Memories (按相似度排序)
```

**特点**:
- 语义理解
- 模糊匹配
- 需要 Qdrant

### 6.3 Hybrid 搜索

```
Query: "OAuth implementation"
    │
    ├─────────────────┐
    │                 │
    ▼                 ▼
┌──────────┐   ┌──────────┐
│Filesystem│   │ Vector   │
│ Search   │   │ Search   │
└────┬─────┘   └────┬─────┘
     │              │
     ▼              ▼
Results A      Results B
     │              │
     └──────┬───────┘
            ▼
    ┌───────────────┐
    │ 结果合并      │
    │ - 去重        │
    │ - 重新排序    │
    └───────┬───────┘
            ▼
    Final Results
```

**合并策略**:

```rust
pub fn merge_results(
    fs_results: Vec<SearchResult>,
    vector_results: Vec<SearchResult>,
) -> Vec<SearchResult> {
    let mut combined: HashMap<String, SearchResult> = HashMap::new();
    
    // 添加文件系统结果
    for result in fs_results {
        combined.insert(result.uri.clone(), result);
    }
    
    // 合并向量搜索结果（提升已有结果的分数）
    for result in vector_results {
        combined.entry(result.uri.clone())
            .and_modify(|e| e.score = (e.score + result.score) / 2.0)
            .or_insert(result);
    }
    
    // 按分数排序
    let mut results: Vec<_> = combined.into_values().collect();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results
}
```

---

## 7. 租户隔离流程

### 7.1 带租户的初始化

```
Data Directory: ./cortex-data
Tenant ID: "tenant-a"
    │
    ▼
┌─────────────────┐
│ CortexFilesystem│
│ with_tenant()   │
└────────┬────────┘
         │
         ▼
创建目录结构:
./cortex-data/
└── tenants/
    └── tenant-a/
        ├── resources/
        ├── user/
        ├── agent/
        └── session/
```

### 7.2 URI 解析

```rust
// 无租户
let fs = CortexFilesystem::new("./cortex-data");
fs.write("cortex://user/test.md", content).await?;
// 实际路径: ./cortex-data/user/test.md

// 有租户
let fs = CortexFilesystem::with_tenant("./cortex-data", "tenant-a");
fs.write("cortex://user/test.md", content).await?;
// 实际路径: ./cortex-data/tenants/tenant-a/user/test.md
```

### 7.3 多租户部署

```
Request with Tenant Header
    │
    ▼
┌─────────────────┐
│  HTTP Service   │
│  Extract Tenant │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Create Isolated │
│   Filesystem    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Process Request │
│ (Tenant Scoped) │
└─────────────────┘
```

---

## 8. 工具调用流程

### 8.1 OpenViking 风格工具

```
Agent Request
    │
    ▼
┌─────────────────┐
│  Tool Selection │
└────────┬────────┘
         │
    ┌────┴────┬──────────┬──────────┐
    ▼         ▼          ▼          ▼
 Abstract  Overview    Read      Search
   Tool      Tool      Tool       Tool
    │         │          │          │
    └─────────┴────┬─────┴──────────┘
                   ▼
          ┌─────────────────┐
          │ MemoryOperations│
          └─────────────────┘
                   │
                   ▼
          ┌─────────────────┐
          │  cortex-mem-core│
          └─────────────────┘
```

### 8.2 工具调用示例

```rust
use cortex_mem_tools::{MemoryOperations, SearchOptions};

let ops = MemoryOperations::new("./cortex-data").await?;

// 1. abstract - 快速了解
let summary = ops.load_abstract("cortex://user/project.md").await?;

// 2. overview - 详细理解
let overview = ops.load_overview("cortex://user/project.md").await?;

// 3. read - 完整内容
let full = ops.read("cortex://user/project.md").await?;

// 4. search - 关键词搜索
let results = ops.search("OAuth", SearchOptions::default()).await?;

// 5. find - 语义搜索
let results = ops.find("authentication", 10).await?;

// 6. ls - 列出目录
let files = ops.ls("cortex://user/").await?;

// 7. explore - 探索结构
let tree = ops.explore("cortex://user/", 3).await?;

// 8. store - 存储记忆
ops.store("cortex://user/new.md", "内容", Metadata::default()).await?;
```

---

## 9. 自动存储流程

### 9.1 Agent 对话自动存储

```
User Input
    │
    ▼
┌─────────────────┐
│ Agent Processing│
└────────┬────────┘
         │
         ▼
Assistant Output
    │
    ▼
┌─────────────────┐
│ Auto Storage    │
│ (Background)    │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
User Msg  Assistant Msg
    │         │
    └────┬────┘
         ▼
┌─────────────────┐
│ Session Storage │
│ with L0/L1 Gen  │
└─────────────────┘
```

### 9.2 配置自动存储

```rust
use cortex_mem_core::{AutoStorageConfig, AgentChatHandler};

let config = AutoStorageConfig {
    enabled: true,
    session_id: "current-session".to_string(),
    generate_layers: true,
    extract_memories: true,
};

let handler = AgentChatHandler::new(filesystem, config);

// 每轮对话自动存储
handler.on_turn_complete(user_msg, assistant_msg).await?;
```

---

## 10. 完整使用场景

### 场景 1: 技术讨论会话

```rust
// 1. 创建会话
let session = manager.create_session(
    "tech-discussion",
    Some("OAuth 2.0 实现讨论".to_string()),
).await?;

// 2. 添加参与者
manager.add_participant(
    "tech-discussion",
    Participant::new("user-1", "张三", ParticipantRole::User),
).await?;

// 3. 进行对话
manager.add_message(
    "tech-discussion",
    Message::new_user("如何实现 OAuth 2.0 授权码流程？"),
).await?;

manager.add_message(
    "tech-discussion",
    Message::new_assistant("建议步骤：1. 注册应用..."),
).await?;

// 4. 提取记忆
let memories = extractor.extract_session("tech-discussion").await?;

// 5. 后续搜索
let results = retrieval_engine.retrieve(
    "OAuth implementation",
    RetrievalOptions::default(),
).await?;
```

### 场景 2: 用户长期记忆

```rust
// 存储用户偏好
ops.store(
    "cortex://user/user-123/preferences.md",
    "## 技术偏好\n- 语言: Rust, Python\n- 框架: Axum, Svelte",
    Metadata::default(),
).await?;

// 存储项目记忆
ops.store(
    "cortex://user/user-123/projects/oauth.md",
    "## OAuth 项目\n实现了完整的 OAuth 2.0 服务器...",
    Metadata::default(),
).await?;

// 快速检索
let prefs = ops.load_abstract("cortex://user/user-123/preferences.md").await?;
let projects = ops.search("OAuth", SearchOptions::default()).await?;
```

### 场景 3: Agent 专属记忆

```rust
// Agent 存储学习到的知识
ops.store(
    "cortex://agent/code-assistant/patterns.md",
    "## 常见代码模式\n1. 错误处理: 使用 anyhow...",
    Metadata::default(),
).await?;

// Agent 检索自身知识
let patterns = ops.load_overview("cortex://agent/code-assistant/patterns.md").await?;
```
