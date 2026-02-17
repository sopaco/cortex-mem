# Cortex-Mem V2.1 技术改进方案

## 状态：已实施 ✅

**实施日期**：2026-02-16

---

## 一、改进概述

本次重构对 Cortex-Mem V2 进行了重大改进，主要参考 OpenViking 的设计理念，实现了：

1. ✅ 移除关键词搜索，强制依赖 vector-search
2. ✅ 简化 URI 结构（移除 user_id/agent_id）
3. ✅ 重构 user/agent 记忆分类（OpenViking 风格）
4. ✅ 强制 LLM 依赖（确保 L0/L1 层生成质量）
5. ✅ 实现目录递归检索（OpenViking 风格）
6. ✅ 向量存储 ID 规范化
7. ✅ 实现 Session 记忆提取机制

---

## 二、改进详情

### 2.1 移除关键词搜索，强制 vector-search

**修改文件**：
- `cortex-mem-core/Cargo.toml` - 移除 `vector-search` feature，qdrant-client 变为直接依赖
- `cortex-mem-core/src/lib.rs` - 移除所有 `#[cfg(feature = "vector-search")]` 条件编译
- `cortex-mem-core/src/retrieval/` - 删除整个目录（关键词搜索实现）
- `cortex-mem-tools/src/tools/search.rs` - 简化为纯向量搜索

**核心变更**：
```rust
// 之前：支持多种搜索引擎
let raw_results = match normalized_args.engine.as_deref() {
    Some("vector") => self.vector_search(&normalized_args).await?,
    Some("hybrid") => self.hybrid_search(&normalized_args).await?,
    _ => self.keyword_search(&normalized_args).await?,
};

// 之后：仅使用向量搜索
let results = self.vector_engine.recursive_search(
    &args.query,
    args.scope.as_deref().unwrap_or("cortex://session"),
    &search_options
).await?;
```

### 2.2 简化 URI 结构

**修改文件**：
- `cortex-mem-core/src/filesystem/uri.rs` - 重构 URI 解析逻辑

**新 URI 结构**：
```
cortex://
├── resources/{resource_name}/
├── user/preferences/{name}.md
├── user/entities/{name}.md
├── user/events/{name}.md
├── agent/cases/{name}.md
├── agent/skills/{name}.md
└── session/{session_id}/timeline/{time}.md
```

**辅助方法**：
```rust
// CortexUri 提供便捷方法
CortexUri::user_preferences("language");  // cortex://user/preferences/language.md
CortexUri::agent_cases("case_001");        // cortex://agent/cases/case_001.md
CortexUri::session("abc123");              // cortex://session/abc123
```

### 2.3 重构 user/agent 记忆分类

**修改文件**：
- `cortex-mem-core/src/types.rs` - 添加新的分类类型

**新增类型**：
```rust
/// User memory category (OpenViking-aligned)
pub enum UserMemoryCategory {
    Profile,      // 用户画像
    Preferences,  // 偏好
    Entities,     // 实体（人物、项目）
    Events,       // 事件/决策
}

/// Agent memory category (OpenViking-aligned)
pub enum AgentMemoryCategory {
    Cases,        // 问题+解决方案案例
    Skills,       // 技能
    Instructions, // 指令
}
```

### 2.4 强制 LLM 依赖

**修改文件**：
- `cortex-mem-core/src/layers/manager.rs` - 移除无 LLM 的 fallback
- `cortex-mem-core/src/layers/generator.rs` - 移除简单截断实现

**核心变更**：
```rust
// 之前：可选 LLM
pub fn new(filesystem: Arc<CortexFilesystem>) -> Self { ... }

// 之后：强制 LLM
pub fn new(filesystem: Arc<CortexFilesystem>, llm_client: Arc<dyn LLMClient>) -> Self { ... }
```

### 2.5 实现目录递归检索

**修改文件**：
- `cortex-mem-core/src/search/vector_engine.rs`

**检索策略**：
1. Intent Analysis - 分析查询意图
2. Initial Positioning - 使用 L0 层快速定位高分目录
3. Refined Exploration - 在高分目录内精细搜索
4. Recursive Drill-down - 递归子目录
5. Result Aggregation - 排序和去重

### 2.6 向量存储 ID 规范化

**修改文件**：
- `cortex-mem-core/src/vector_store/mod.rs`

**新增函数**：
```rust
/// Generate normalized vector ID: {uri}#/L{layer}
pub fn uri_to_vector_id(uri: &str, layer: ContextLayer) -> String {
    match layer {
        ContextLayer::L0Abstract => format!("{}#/L0", uri),
        ContextLayer::L1Overview => format!("{}#/L1", uri),
        ContextLayer::L2Detail => uri.to_string(),
    }
}

/// Parse vector ID back to (uri, layer)
pub fn parse_vector_id(vector_id: &str) -> (String, ContextLayer);
```

### 2.7 实现 Session 记忆提取机制

**新增文件**：
- `cortex-mem-core/src/session/extraction.rs`

**核心结构**：
```rust
/// Extracted memory from session
pub struct ExtractedMemories {
    pub preferences: Vec<PreferenceMemory>,
    pub entities: Vec<EntityMemory>,
    pub events: Vec<EventMemory>,
    pub cases: Vec<CaseMemory>,
}

/// Memory extractor for session commit
pub struct MemoryExtractor {
    llm_client: Arc<dyn LLMClient>,
    filesystem: Arc<CortexFilesystem>,
}

impl MemoryExtractor {
    pub async fn extract(&self, messages: &[String]) -> Result<ExtractedMemories>;
    pub async fn save_memories(&self, memories: &ExtractedMemories) -> Result<()>;
}
```

---

## 三、存储结构

### 3.1 租户隔离

每个租户（Bot）有独立的存储目录：

```
{app_data}/cortex/tenants/{bot_id}/
├── user/
│   ├── preferences/
│   ├── entities/
│   └── events/
├── agent/
│   ├── cases/
│   └── skills/
└── session/
    └── {session_id}/
        └── timeline/
```

### 3.2 物理路径映射

| URI | 物理路径 |
|-----|---------|
| `cortex://user/preferences/language.md` | `{data}/tenants/{bot_id}/user/preferences/language.md` |
| `cortex://agent/cases/case_001.md` | `{data}/tenants/{bot_id}/agent/cases/case_001.md` |
| `cortex://session/{id}/timeline/10_00.md` | `{data}/tenants/{bot_id}/session/{id}/timeline/10_00.md` |

---

## 四、API 变更

### 4.1 MemoryOperations 初始化

**之前**：
```rust
// 多种初始化方式
MemoryOperations::from_data_dir(&data_dir).await?;
MemoryOperations::with_tenant(&data_dir, tenant_id).await?;
MemoryOperations::with_tenant_and_llm(&data_dir, tenant_id, llm_client).await?;
```

**之后**（统一入口）：
```rust
// 唯一初始化方式，需要所有依赖
MemoryOperations::new(
    &data_dir,
    tenant_id,
    llm_client,
    qdrant_url,
    qdrant_collection,
    embedding_api_base_url,
    embedding_api_key,
).await?;
```

### 4.2 cortex-mem-rig API

**之前**：
```rust
create_memory_tools_with_tenant(data_dir, tenant_id).await?;
create_memory_tools_with_tenant_and_llm(data_dir, tenant_id, llm_client).await?;
create_memory_tools_with_tenant_and_vector(...).await?;
```

**之后**（唯一入口）：
```rust
create_memory_tools_with_tenant_and_vector(
    data_dir,
    tenant_id,
    llm_client,
    qdrant_url,
    qdrant_collection,
    embedding_api_base_url,
    embedding_api_key,
).await?;
```

---

## 五、依赖要求

### 5.1 必需依赖

- **Qdrant** - 向量数据库（http://localhost:6334）
- **LLM 服务** - 用于 L0/L1 层生成（OpenAI 兼容 API）
- **Embedding 服务** - 用于向量化（text-embedding-3-small）

### 5.2 配置示例

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model_efficient = "gpt-4o-mini"

[qdrant]
url = "http://localhost:6334"
collection_name = "cortex_mem"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model = "text-embedding-3-small"
```

---

## 六、与 OpenViking 对比

| 方面 | OpenViking | Cortex-Mem V2.1 |
|------|------------|-----------------|
| **URI 协议** | `viking://` | `cortex://` |
| **user 维度** | `user/memories/preferences/` | `user/preferences/` |
| **agent 维度** | `agent/memories/cases/` | `agent/cases/` |
| **session 维度** | `session/{id}/` | `session/{id}/` |
| **L0/L1/L2** | ✅ | ✅ |
| **目录递归检索** | ✅ | ✅ |
| **记忆提取** | ✅ | ✅ |
| **多租户** | 多目录 | 单目录多租户 |
| **语言** | Python | Rust |

---

## 七、后续工作

### 7.1 建议优化

1. **记忆提取触发机制** - 在 SessionManager 中集成自动提取
2. **profile.json 迁移** - 将旧的用户 profile 迁移到新的分类结构
3. **索引优化** - 在 SyncManager 中使用新的向量 ID 格式

### 7.2 测试覆盖

- [ ] 单元测试：URI 解析和生成
- [ ] 单元测试：向量 ID 规范化
- [ ] 集成测试：记忆提取流程
- [ ] 集成测试：目录递归检索

---

## 八、编译验证

```bash
cargo check --lib
# ✅ 编译成功，仅有一些未使用变量的警告
```

---

**文档版本**：v2.1
**最后更新**：2026-02-16
