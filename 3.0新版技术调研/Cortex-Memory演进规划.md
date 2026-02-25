# Cortex-Memory 3.0 演进规划

> 基于 OpenViking 深度调研的技术演进路线图（轻量化、高性能版本）

---

## 一、演进愿景

### 1.1 目标定位

**Cortex-Memory 3.0** 将从"高性能内存框架"演进为 **"轻量级、高性能、智能化的AI上下文数据库"**，融合 Rust 原生性能优势与 OpenViking 的先进架构理念，同时保持简洁、易用、高效的核心优势。

### 1.2 核心价值主张

- **轻量至上**: 零额外依赖，简单部署，开箱即用
- **性能卓越**: 保持 Rust 原生性能优势（93.33% Recall@1），优化查询延迟
- **Token高效**: 智能分层加载，精准控制上下文大小（.abstract < 2K）
- **架构先进**: 借鉴 OpenViking 目录递归检索、智能去重
- **生态完整**: 强化 REST API + MCP + Web 仪表板

### 1.3 非目标（明确不做）

- ❌ 分布式存储（保持单机部署简洁性）
- ❌ 历史操作记录回溯（避免复杂性）
- ❌ 企业级审计日志（聚焦核心功能）
- ❌ 多副本高可用（保持轻量）

---

## 二、当前遗留问题修复（优先级：🔥🔥🔥）

> 在实施新功能前，必须先解决 2.0 版本的现存问题

### 2.1 三层递进文件缺失问题

**问题描述**: 当前实现中，并非每个目录都生成了 `.abstract` 和 `.overview` 文件，导致分层检索不完整。

**根本原因分析**:
```rust
// 当前实现：懒生成策略
// 仅在首次访问时生成，但很多目录从未被访问过
pub async fn get_abstract(&self, uri: &str) -> Result<String> {
    if let Some(cached) = self.cache.get(uri) {
        return Ok(cached);
    }
    // 问题：如果从未被调用，L0/L1 永远不会生成
    let abstract_text = self.generate_abstract(uri).await?;
    Ok(abstract_text)
}
```

**解决方案**:

#### 方案1: 渐进式主动生成（推荐）

```rust
pub struct LayerGenerationStrategy {
    // 新增：渐进式生成配置
    pub enable_progressive_generation: bool,
    pub batch_size: usize,  // 每批生成数量
    pub delay_ms: u64,      // 批次间延迟
}

impl AutoIndexer {
    /// 在后台渐进式生成所有缺失的 L0/L1
    pub async fn ensure_all_layers(&self) -> Result<GenerationStats> {
        // 1. 扫描所有目录
        let directories = self.scan_all_directories().await?;
        
        // 2. 过滤出缺失 L0/L1 的目录
        let missing = self.filter_missing_layers(&directories).await?;
        
        // 3. 分批生成，避免过载
        let mut generated = 0;
        for batch in missing.chunks(self.config.batch_size) {
            for dir in batch {
                if let Err(e) = self.generate_layers_for_directory(dir).await {
                    warn!("Failed to generate layers for {}: {}", dir, e);
                } else {
                    generated += 1;
                }
            }
            // 批次间延迟，避免 LLM API 限流
            tokio::time::sleep(Duration::from_millis(self.config.delay_ms)).await;
        }
        
        Ok(GenerationStats { 
            total: missing.len(),
            generated,
            failed: missing.len() - generated,
        })
    }
    
    /// 检测目录是否缺失 L0/L1
    async fn has_layers(&self, uri: &str) -> Result<bool> {
        let abstract_path = format!("{}/.abstract", uri);
        let overview_path = format!("{}/.overview", uri);
        
        Ok(
            self.filesystem.exists(&abstract_path).await? &&
            self.filesystem.exists(&overview_path).await?
        )
    }
}
```

**配置**:
```toml
[layers.generation]
# 启用渐进式生成
enable_progressive_generation = true
# 每批生成 10 个目录
batch_size = 10
# 批次间延迟 2 秒
delay_ms = 2000
# 启动时自动检查并生成
auto_generate_on_startup = true
```

**CLI 支持**:
```bash
# 手动触发全量生成
cortex-mem-cli layers ensure-all --tenant acme

# 查看生成进度
cortex-mem-cli layers status --tenant acme
```

**实现计划**:
- [ ] 扩展 `AutoIndexer` 支持层级生成
- [ ] 实现目录扫描和缺失检测
- [ ] 实现分批渐进式生成
- [ ] 添加 CLI 命令
- [ ] 添加启动时自动检查
- [ ] 编写单元测试

**预期收益**:
- 100% 目录覆盖 L0/L1
- 递归检索完整性保障
- 用户无感知后台生成

---

### 2.2 .abstract 文件过大问题

**问题描述**: 生成的 `.abstract` 文件有时接近 5K，远超 500-2K 的目标范围，导致 Token 消耗过大。

**根本原因分析**:
```rust
// 当前 Prompt 缺乏明确的长度约束
let prompt = format!(
    "请为以下内容生成一句话摘要：\n\n{}",
    content
);
// 问题：LLM 可能生成冗长的摘要
```

**解决方案**:

#### 方案1: 强化 Prompt 约束（推荐）

```rust
pub struct AbstractGenerationConfig {
    pub max_tokens: usize,     // 最大 Token 数（默认 400）
    pub max_chars: usize,      // 最大字符数（默认 2000）
    pub target_sentences: usize, // 目标句子数（默认 1-3）
}

impl LayerGenerator {
    async fn generate_abstract_v2(
        &self,
        content: &str,
        category: &str,
    ) -> Result<String> {
        let prompt = format!(
            r#"请为以下{category}内容生成简洁的一句话摘要。

【严格要求】
- 最多 {max_tokens} tokens（约 {max_chars} 字符）
- 1-3 个完整句子
- 提炼核心要点，删除细节描述
- 使用精炼语言，避免冗余

【内容】
{content}

【输出格式】
仅返回摘要文本，不要包含任何前缀、后缀或解释。"#,
            category = category,
            max_tokens = self.config.max_tokens,
            max_chars = self.config.max_chars,
            content = self.truncate_content(content, 4000), // 输入也截断
        );
        
        // 调用 LLM
        let response = self.llm_client.generate(&prompt).await?;
        
        // 后处理：强制截断
        let abstract_text = self.enforce_limits(response)?;
        
        Ok(abstract_text)
    }
    
    /// 强制执行长度限制
    fn enforce_limits(&self, text: String) -> Result<String> {
        let mut result = text.trim().to_string();
        
        // 1. 字符数限制
        if result.len() > self.config.max_chars {
            // 截断到最后一个句号/问号/叹号
            if let Some(pos) = result[..self.config.max_chars]
                .rfind(|c| c == '。' || c == '.' || c == '?' || c == '!') 
            {
                result.truncate(pos + 1);
            } else {
                result.truncate(self.config.max_chars);
                result.push_str("...");
            }
        }
        
        // 2. 验证 Token 数（使用 tiktoken 或估算）
        let token_count = self.estimate_tokens(&result);
        if token_count > self.config.max_tokens {
            // 再次压缩
            result = self.compress_to_tokens(result, self.config.max_tokens)?;
        }
        
        Ok(result)
    }
    
    /// 估算 Token 数（简化版）
    fn estimate_tokens(&self, text: &str) -> usize {
        // 中文：1字符 ≈ 1.5 tokens
        // 英文：1单词 ≈ 1.3 tokens
        // 简化估算：平均 1 字符 ≈ 1.2 tokens
        (text.len() as f32 * 1.2) as usize
    }
}
```

**配置**:
```toml
[layers.abstract]
# 最大 Token 数
max_tokens = 400
# 最大字符数（约 500 tokens）
max_chars = 2000
# 目标句子数
target_sentences = 2

[layers.overview]
# Overview 允许稍长
max_tokens = 1500
max_chars = 6000
```

**实现计划**:
- [ ] 更新 Prompt 模板，增加明确的长度约束
- [ ] 实现后处理截断逻辑
- [ ] 集成 Token 估算（或tiktoken库）
- [ ] 添加配置支持
- [ ] 编写验证测试（确保 100% 符合长度要求）
- [ ] 更新现有 `.abstract` 文件（重新生成）

**预期收益**:
- `.abstract` 严格控制在 500-2K 字符
- Token 消耗降低 50%+
- 检索速度提升

---

### 2.3 性能优化

**问题描述**: 当前记忆查询时间较长，需要通过并发、缓存等手段优化。

**性能瓶颈分析**:

```rust
// 当前实现的主要性能瓶颈：

// 1. 串行 L0/L1/L2 读取
let l0 = self.read_abstract(uri).await?;  // 20ms
let l1 = self.read_overview(uri).await?;  // 30ms
let l2 = self.read_content(uri).await?;   // 50ms
// 总计：100ms

// 2. 重复 Embedding 生成
for query in queries {
    let vector = self.embed(query).await?; // 每次 50ms
}

// 3. 同步等待向量搜索
let results = self.vector_store.search(vector).await?; // 30ms
```

**解决方案**:

#### 优化1: 并发 L0/L1/L2 读取

```rust
use futures::future::try_join_all;

impl LayerReader {
    /// 并发读取所有层级
    pub async fn read_all_layers_concurrent(
        &self,
        uris: &[String],
    ) -> Result<HashMap<String, LayerBundle>> {
        let tasks: Vec<_> = uris.iter().map(|uri| {
            let uri = uri.clone();
            let reader = self.clone();
            async move {
                // 并发读取 L0/L1/L2
                let (l0, l1, l2) = tokio::join!(
                    reader.read_abstract(&uri),
                    reader.read_overview(&uri),
                    reader.read_content(&uri),
                );
                
                Ok::<_, Error>((uri, LayerBundle {
                    abstract_text: l0.ok(),
                    overview: l1.ok(),
                    content: l2.ok(),
                }))
            }
        }).collect();
        
        let results = try_join_all(tasks).await?;
        Ok(results.into_iter().collect())
    }
}

// 性能提升：100ms -> 50ms（理论）
```

#### 优化2: Embedding 缓存

```rust
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CachedEmbeddingClient {
    inner: Arc<dyn EmbeddingClient>,
    cache: Arc<Mutex<LruCache<String, Vec<f32>>>>,
}

impl CachedEmbeddingClient {
    pub fn new(client: Arc<dyn EmbeddingClient>, capacity: usize) -> Self {
        Self {
            inner: client,
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }
    
    pub async fn embed_with_cache(&self, text: &str) -> Result<Vec<f32>> {
        // 1. 检查缓存
        {
            let mut cache = self.cache.lock().await;
            if let Some(vector) = cache.get(text) {
                return Ok(vector.clone());
            }
        }
        
        // 2. 生成 Embedding
        let vector = self.inner.embed(text).await?;
        
        // 3. 写入缓存
        {
            let mut cache = self.cache.lock().await;
            cache.put(text.to_string(), vector.clone());
        }
        
        Ok(vector)
    }
}

// 性能提升：重复查询从 50ms -> 0.1ms
```

#### 优化3: 批量 Embedding 生成

```rust
impl EmbeddingClient {
    /// 批量生成 Embedding（利用 API 批量接口）
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        
        // OpenAI API 支持批量（最多 2048 个）
        let response = self.client.post("/embeddings")
            .json(&serde_json::json!({
                "model": self.config.model_name,
                "input": texts,
            }))
            .send()
            .await?;
        
        // 解析批量响应
        let data: EmbeddingResponse = response.json().await?;
        Ok(data.data.into_iter().map(|d| d.embedding).collect())
    }
}

// 性能提升：10个查询从 500ms -> 80ms
```

#### 优化4: 向量搜索结果缓存

```rust
pub struct SearchCache {
    cache: Arc<Mutex<LruCache<SearchCacheKey, Vec<SearchResult>>>>,
    ttl: Duration,
}

#[derive(Hash, Eq, PartialEq)]
struct SearchCacheKey {
    query_hash: u64,
    limit: usize,
    filters: String, // JSON 序列化的过滤条件
}

impl VectorSearchEngine {
    pub async fn search_with_cache(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let cache_key = SearchCacheKey {
            query_hash: self.hash_query(query),
            limit: options.limit,
            filters: serde_json::to_string(&options.filters)?,
        };
        
        // 检查缓存
        if let Some(cached) = self.cache.get(&cache_key).await {
            if !cached.is_expired() {
                return Ok(cached.results.clone());
            }
        }
        
        // 执行搜索
        let results = self.inner_search(query, options).await?;
        
        // 写入缓存
        self.cache.put(cache_key, CachedResult {
            results: results.clone(),
            timestamp: Utc::now(),
        }).await;
        
        Ok(results)
    }
}
```

**配置**:
```toml
[performance]
# 并发读取
enable_concurrent_layer_reading = true
max_concurrent_reads = 10

# Embedding 缓存
enable_embedding_cache = true
embedding_cache_size = 1000

# 批量 Embedding
enable_batch_embedding = true
batch_size = 32

# 搜索结果缓存
enable_search_cache = true
search_cache_size = 500
search_cache_ttl_secs = 300
```

**实现计划**:
- [ ] 实现并发 L0/L1/L2 读取
- [ ] 实现 Embedding 缓存层
- [ ] 实现批量 Embedding 接口
- [ ] 实现搜索结果缓存
- [ ] 添加性能监控指标
- [ ] 编写性能基准测试
- [ ] 文档更新

**预期收益**:

| 优化项 | 当前 | 优化后 | 提升 |
|--------|------|--------|------|
| 单次查询延迟 | ~200ms | ~80ms | 60% |
| 重复查询 | ~200ms | ~10ms | 95% |
| 批量查询 (10个) | ~2000ms | ~300ms | 85% |
| 内存占用 | 50MB | 100MB | -50MB (可接受) |

---

## 三、核心功能演进

> 在修复当前问题后，实施以下核心功能

### 3.1 检索引擎升级（优先级：🔥🔥🔥）

#### 3.1.1 目录递归检索 (Hierarchical Retrieval)

**目标**: 从平铺式向量检索升级为层级化目录递归检索（借鉴 OpenViking，但保持轻量）

**当前实现:**
```rust
// cortex-mem-core/src/search/mod.rs
// 平铺式检索：直接向量搜索 + L0/L1/L2 加权
pub async fn search(&self, query: &str) -> Vec<SearchResult> {
    let vector = self.embedding_client.embed(query).await?;
    let results = self.vector_store.search(vector, limit).await?;
    // 加权评分
    self.apply_weighted_scoring(results)
}
```

**目标实现:**
```rust
// 新增 hierarchical_retriever.rs
pub struct HierarchicalRetriever {
    vector_store: Arc<dyn VectorStore>,
    embedder: Arc<dyn EmbeddingClient>,
    config: HierarchicalConfig,
}

impl HierarchicalRetriever {
    /// 目录递归检索
    pub async fn retrieve(&self, query: &TypedQuery) -> QueryResult {
        // 1. 全局搜索定位高分目录
        let global_results = self.global_search(query).await?;
        
        // 2. 递归搜索子目录
        let candidates = self.recursive_search(
            query,
            global_results,
            self.config.max_depth,
        ).await?;
        
        // 3. 分数传播与收敛
        let scored = self.apply_score_propagation(candidates);
        
        // 4. 可选 Rerank
        if let Some(reranker) = &self.reranker {
            reranker.rerank(query, scored).await
        } else {
            Ok(scored)
        }
    }
    
    /// 分数传播机制
    fn apply_score_propagation(&self, candidates: Vec<Candidate>) -> Vec<Candidate> {
        let alpha = 0.5; // 可配置
        candidates.into_iter().map(|mut c| {
            c.final_score = alpha * c.current_score 
                          + (1.0 - alpha) * c.parent_score;
            c
        }).collect()
    }
}
```

**配置参数:**
```toml
[search.hierarchical]
enabled = true
max_depth = 3
score_propagation_alpha = 0.5
convergence_rounds = 3
global_search_topk = 3
```

**实现计划:**
- [ ] 定义 `TypedQuery` 结构体（支持 context_type、target_directories）
- [ ] 实现 `HierarchicalRetriever` 核心逻辑
- [ ] 实现分数传播算法
- [ ] 实现收敛检测机制
- [ ] 编写单元测试和基准测试
- [ ] 集成到现有 `VectorSearchEngine`

**预期收益:**
- 检索精度提升 10-15%
- 更好的全局理解能力
- 减少误召回
- **保持轻量**: 无需额外依赖，核心算法 < 500 行代码

---

#### 3.1.2 意图分析增强 (Intent Analysis)

**目标**: 自动分析用户查询意图，生成更精准的类型化查询（简化版，避免过度复杂）

**实现:**
```rust
pub struct IntentAnalyzer {
    llm_client: Arc<dyn LLMClient>,
}

pub struct QueryPlan {
    queries: Vec<TypedQuery>,
}

pub struct TypedQuery {
    query: String,
    context_type: ContextType,  // Memory/Resource/Skill
    intent: String,
    priority: u8,
    target_directories: Vec<String>,
}

impl IntentAnalyzer {
    pub async fn analyze(
        &self,
        query: &str,
        session_context: Option<&SessionContext>,
    ) -> Result<QueryPlan> {
        let prompt = format!(
            "分析用户查询意图，生成多个类型化查询：\n\
             用户查询: {}\n\
             会话上下文: {:?}\n\
             返回 JSON 格式的 QueryPlan",
            query, session_context
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        let plan: QueryPlan = serde_json::from_str(&response)?;
        Ok(plan)
    }
}
```

**使用场景:**
```rust
// 用户查询: "我之前提到的那个项目现在进展如何?"
let plan = intent_analyzer.analyze(query, Some(&session)).await?;
// 生成:
// - TypedQuery { context_type: Memory, query: "用户提到的项目", ... }
// - TypedQuery { context_type: Session, query: "项目进展", ... }
```

**实现计划:**
- [ ] 定义 `IntentAnalyzer` 和 `QueryPlan`
- [ ] 编写 Prompt 模板
- [ ] 集成到搜索流程
- [ ] 支持会话上下文注入

---

### 2.2 会话管理增强（优先级：🔥🔥）

#### 2.2.1 会话压缩与归档

**目标**: 借鉴 OpenViking 的自动压缩归档机制，控制上下文窗口

**当前实现:**
```rust
// 保留完整会话历史
session_manager.add_message(thread_id, message).await?;
// 会话关闭时提取记忆
session_manager.close(thread_id).await?;
```

**目标实现:**
```rust
pub struct SessionCompressionConfig {
    pub auto_threshold_tokens: usize,  // 默认 8000
    pub auto_threshold_messages: usize, // 默认 100
    pub archive_enabled: bool,
    pub max_archives: usize,  // 最多保留归档数
}

pub struct SessionCompression {
    pub summary: String,
    pub original_count: usize,
    pub compressed_count: usize,
    pub compression_index: usize,
}

impl SessionManager {
    /// 检查是否需要压缩
    async fn check_compression_needed(&self, thread_id: &str) -> bool {
        let stats = self.get_session_stats(thread_id).await?;
        stats.total_tokens > self.config.auto_threshold_tokens
            || stats.message_count > self.config.auto_threshold_messages
    }
    
    /// 自动压缩归档
    pub async fn auto_compress(&self, thread_id: &str) -> Result<CompressionResult> {
        // 1. 读取当前消息
        let messages = self.get_messages(thread_id).await?;
        
        // 2. 生成结构化摘要 (LLM)
        let summary = self.generate_summary(&messages).await?;
        let abstract_text = self.extract_abstract(&summary);
        
        // 3. 创建归档
        let compression_idx = self.get_next_compression_index(thread_id).await?;
        let archive_uri = format!(
            "cortex://session/{}/history/archive_{:03}",
            thread_id, compression_idx
        );
        
        // 写入归档
        self.filesystem.write(
            &format!("{}/messages.jsonl", archive_uri),
            &serde_json::to_string(&messages)?,
        ).await?;
        
        self.filesystem.write(
            &format!("{}/.abstract.md", archive_uri),
            &abstract_text,
        ).await?;
        
        self.filesystem.write(
            &format!("{}/.overview.md", archive_uri),
            &summary,
        ).await?;
        
        // 4. 提取长期记忆
        let memories = self.memory_extractor.extract(&messages, thread_id).await?;
        
        // 5. 清空当前消息
        self.clear_current_messages(thread_id).await?;
        
        Ok(CompressionResult {
            compression_index: compression_idx,
            archive_uri,
            memories_extracted: memories.len(),
        })
    }
    
    /// 获取会话上下文用于检索
    pub async fn get_context_for_search(
        &self,
        thread_id: &str,
        query: &str,
        max_archives: usize,
    ) -> Result<SessionContext> {
        // 1. 当前消息
        let recent_messages = self.get_recent_messages(thread_id, 20).await?;
        
        // 2. 相关归档摘要（基于 query 匹配）
        let summaries = self.find_relevant_archives(
            thread_id,
            query,
            max_archives,
        ).await?;
        
        Ok(SessionContext {
            recent_messages,
            summaries,
        })
    }
}
```

**配置:**
```toml
[session.compression]
enabled = true
auto_threshold_tokens = 8000
auto_threshold_messages = 100
archive_enabled = true
max_archives = 10  # 自动删除旧归档
```

**实现计划:**
- [ ] 定义 `SessionCompressionConfig` 和相关结构体
- [ ] 实现自动压缩触发逻辑
- [ ] 实现归档写入和管理
- [ ] 实现归档检索和上下文注入
- [ ] 编写压缩统计和监控

**预期收益:**
- 上下文窗口可控
- 支持超长对话
- 降低 LLM 成本

---

#### 2.2.2 记忆分类扩展

**目标**: 扩展记忆分类，支持 Profile 和 Pattern

**当前分类:**
```rust
pub enum MemoryCategory {
    Preference,  // 用户偏好
    Entity,      // 实体记忆
    Event,       // 事件记录
    Case,        // Agent案例
}
```

**目标分类:**
```rust
pub enum MemoryCategory {
    // 用户记忆
    Profile,     // 🆕 用户画像
    Preference,  // 用户偏好
    Entity,      // 实体记忆
    Event,       // 事件记录
    
    // Agent记忆
    Case,        // 案例库
    Pattern,     // 🆕 模式库
}
```

**Profile 实现:**
```rust
impl MemoryExtractor {
    async fn extract_profile(
        &self,
        messages: &[Message],
    ) -> Result<Option<ProfileMemory>> {
        // 分析用户基本信息、职业、兴趣等
        let prompt = "从对话中提取用户画像...";
        let response = self.llm_client.generate(prompt).await?;
        
        // 合并到现有 Profile
        let existing = self.filesystem.read(
            "cortex://user/profile.md"
        ).await.ok();
        
        if let Some(existing) = existing {
            // LLM 合并
            self.merge_profile(existing, response).await
        } else {
            Ok(Some(ProfileMemory { content: response }))
        }
    }
}
```

**Pattern 实现:**
```rust
pub struct PatternMemory {
    pub abstract_text: String,
    pub overview: String,
    pub content: String,      // Markdown格式的模式描述
    pub applicability: String, // 适用场景
    pub examples: Vec<String>, // 示例
}

impl MemoryExtractor {
    async fn extract_patterns(
        &self,
        messages: &[Message],
    ) -> Result<Vec<PatternMemory>> {
        // 从多次交互中提炼可复用模式
        let prompt = "提炼可复用的流程、方法和最佳实践...";
        let response = self.llm_client.generate(prompt).await?;
        // 解析为 PatternMemory 列表
        self.parse_patterns(response).await
    }
}
```

**实现计划:**
- [ ] 扩展 `MemoryCategory` 枚举
- [ ] 实现 Profile 提取和合并逻辑
- [ ] 实现 Pattern 提取和存储
- [ ] 更新提取 Prompt 模板
- [ ] 更新存储路径映射

---

#### 2.2.3 记忆去重与合并

**目标**: 借鉴 OpenViking 的智能去重机制

**实现:**
```rust
pub struct MemoryDeduplicator {
    vector_store: Arc<dyn VectorStore>,
    llm_client: Arc<dyn LLMClient>,
    similarity_threshold: f32, // 0.85
}

impl MemoryDeduplicator {
    pub async fn check_duplicate(
        &self,
        candidate: &CandidateMemory,
        category: MemoryCategory,
    ) -> Result<DeduplicationResult> {
        // 1. 向量相似度检索
        let vector = self.embedding_client.embed(&candidate.abstract).await?;
        let similar = self.vector_store.search(
            vector,
            Filter::category(category),
            limit: 5,
        ).await?;
        
        // 2. 过滤高相似度候选
        let high_similar: Vec<_> = similar.into_iter()
            .filter(|r| r.score > self.similarity_threshold)
            .collect();
        
        if high_similar.is_empty() {
            return Ok(DeduplicationResult::NoDuplicate);
        }
        
        // 3. LLM 精确判断
        for existing in high_similar {
            let prompt = format!(
                "判断以下两个记忆是否重复：\n\
                 现有记忆: {}\n\
                 新记忆: {}\n\
                 返回 JSON: {{\"is_duplicate\": bool, \"reason\": string}}",
                existing.content,
                candidate.content
            );
            
            let response = self.llm_client.generate(&prompt).await?;
            let result: DuplicateCheckResult = serde_json::from_str(&response)?;
            
            if result.is_duplicate {
                return Ok(DeduplicationResult::Duplicate {
                    existing_uri: existing.uri,
                    should_merge: self.should_merge(category),
                });
            }
        }
        
        Ok(DeduplicationResult::NoDuplicate)
    }
    
    /// 合并记忆
    pub async fn merge_memory(
        &self,
        existing_uri: &str,
        new_content: &str,
        category: MemoryCategory,
    ) -> Result<MergedMemory> {
        let existing_content = self.filesystem.read(existing_uri).await?;
        
        let prompt = format!(
            "合并以下两个记忆，保留完整信息：\n\
             现有: {}\n\
             新增: {}\n\
             返回 JSON: {{\"abstract\": string, \"overview\": string, \"content\": string}}",
            existing_content, new_content
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        let merged: MergedMemory = serde_json::from_str(&response)?;
        
        // 更新文件
        self.filesystem.write(existing_uri, &merged.content).await?;
        
        Ok(merged)
    }
}
```

**实现计划:**
- [ ] 定义 `MemoryDeduplicator` 结构体
- [ ] 实现向量相似度检索
- [ ] 实现 LLM 精确判断
- [ ] 实现合并逻辑
- [ ] 集成到提取流程

---

### 2.3 分层内存优化（优先级：🔥）

#### 2.3.1 主动生成 vs 懒生成策略

**目标**: 提供可配置的 L0/L1 生成策略

**当前实现:** 懒生成
```rust
// 仅在首次访问时生成
pub async fn get_abstract(&self, uri: &str) -> Result<String> {
    if let Some(cached) = self.cache.get(uri) {
        return Ok(cached);
    }
    // 生成并缓存
    let abstract_text = self.generate_abstract(uri).await?;
    self.cache.insert(uri, abstract_text.clone());
    Ok(abstract_text)
}
```

**目标实现:** 支持主动生成
```rust
pub enum LayerGenerationStrategy {
    Lazy,     // 懒生成（按需）
    Eager,    // 主动生成（写入时）
    Hybrid,   // 混合（高频访问主动，低频懒加载）
}

impl LayerManager {
    pub async fn write_with_layers(
        &self,
        uri: &str,
        content: &str,
        strategy: LayerGenerationStrategy,
    ) -> Result<()> {
        // 1. 写入原始内容
        self.filesystem.write(uri, content).await?;
        
        match strategy {
            LayerGenerationStrategy::Eager => {
                // 立即生成 L0/L1
                let (abstract_text, overview) = self.generate_layers(content).await?;
                
                // 写入独立文件
                let parent = self.get_parent_uri(uri);
                self.filesystem.write(
                    &format!("{}/.abstract.md", parent),
                    &abstract_text,
                ).await?;
                
                self.filesystem.write(
                    &format!("{}/.overview.md", parent),
                    &overview,
                ).await?;
            }
            
            LayerGenerationStrategy::Lazy => {
                // 什么都不做，等待首次访问
            }
            
            LayerGenerationStrategy::Hybrid => {
                // 异步队列生成
                self.enqueue_layer_generation(uri).await?;
            }
        }
        
        Ok(())
    }
}
```

**配置:**
```toml
[layers]
generation_strategy = "hybrid"  # lazy | eager | hybrid
cache_enabled = true
cache_ttl_secs = 3600
```

**实现计划:**
- [ ] 定义生成策略枚举
- [ ] 实现主动生成逻辑
- [ ] 实现混合策略（异步队列）
- [ ] 扩展配置支持
- [ ] 性能测试对比

---

#### 2.3.2 批量抽象获取优化

**目标**: 借鉴 OpenViking 的并发批量抽象获取

**实现:**
```rust
impl LayerManager {
    /// 批量并发获取抽象
    pub async fn batch_get_abstracts(
        &self,
        uris: &[String],
        concurrency: usize,
    ) -> Result<HashMap<String, String>> {
        use futures::stream::{self, StreamExt};
        
        let results: Vec<_> = stream::iter(uris)
            .map(|uri| async move {
                let abstract_text = self.get_abstract(uri).await?;
                Ok::<_, Error>((uri.clone(), abstract_text))
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;
        
        let mut map = HashMap::new();
        for result in results {
            let (uri, abstract_text) = result?;
            map.insert(uri, abstract_text);
        }
        
        Ok(map)
    }
}
```

**使用场景:**
```rust
// 目录列表展示抽象
let uris = filesystem.list("cortex://user/memories/").await?;
let abstracts = layer_manager.batch_get_abstracts(&uris, 6).await?;

for uri in uris {
    println!("{}: {}", uri, abstracts.get(&uri).unwrap_or(&"".to_string()));
}
```

**实现计划:**
- [ ] 实现批量并发获取
- [ ] 添加信号量限流
- [ ] 集成到 CLI `list` 命令
- [ ] 集成到 REST API

---

### 2.4 可观测性增强（优先级：🔥）

#### 2.4.1 检索轨迹记录

**目标**: 记录完整的检索过程，支持可视化分析

**实现:**
```rust
pub struct SearchTrace {
    pub query: String,
    pub timestamp: DateTime<Utc>,
    pub steps: Vec<SearchStep>,
    pub final_results: Vec<SearchResult>,
    pub total_duration_ms: u64,
}

pub struct SearchStep {
    pub step_type: SearchStepType,
    pub description: String,
    pub directory: Option<String>,
    pub candidates_count: usize,
    pub top_scores: Vec<f32>,
    pub duration_ms: u64,
}

pub enum SearchStepType {
    GlobalSearch,
    DirectorySearch,
    ScorePropagation,
    Rerank,
}

impl HierarchicalRetriever {
    pub async fn retrieve_with_trace(
        &self,
        query: &TypedQuery,
    ) -> Result<(QueryResult, SearchTrace)> {
        let mut trace = SearchTrace::new(query.query.clone());
        let start = Instant::now();
        
        // 1. 全局搜索
        let global_start = Instant::now();
        let global_results = self.global_search(query).await?;
        trace.add_step(SearchStep {
            step_type: SearchStepType::GlobalSearch,
            description: "全局向量搜索定位高分目录".to_string(),
            directory: None,
            candidates_count: global_results.len(),
            top_scores: global_results.iter().take(3).map(|r| r.score).collect(),
            duration_ms: global_start.elapsed().as_millis() as u64,
        });
        
        // 2. 递归搜索
        let recursive_start = Instant::now();
        let candidates = self.recursive_search_with_trace(
            query,
            global_results,
            &mut trace,
        ).await?;
        
        // 3. 分数传播
        let prop_start = Instant::now();
        let scored = self.apply_score_propagation(candidates);
        trace.add_step(SearchStep {
            step_type: SearchStepType::ScorePropagation,
            description: "应用分数传播算法".to_string(),
            directory: None,
            candidates_count: scored.len(),
            top_scores: scored.iter().take(5).map(|c| c.final_score).collect(),
            duration_ms: prop_start.elapsed().as_millis() as u64,
        });
        
        trace.total_duration_ms = start.elapsed().as_millis() as u64;
        trace.final_results = scored.clone();
        
        Ok((QueryResult { results: scored }, trace))
    }
}
```

**存储:**
```rust
// 保存检索轨迹到文件
let trace_uri = format!(
    "cortex://session/{}/traces/search_{}.json",
    thread_id,
    Uuid::new_v4()
);
filesystem.write(&trace_uri, &serde_json::to_string(&trace)?).await?;
```

**可视化集成:**
```typescript
// cortex-mem-insights/src/components/SearchTraceViewer.svelte
export interface SearchTrace {
  query: string;
  timestamp: string;
  steps: SearchStep[];
  finalResults: SearchResult[];
  totalDurationMs: number;
}

// 展示检索流程图、分数分布等
```

**实现计划:**
- [ ] 定义 `SearchTrace` 结构体
- [ ] 集成到检索流程
- [ ] 实现轨迹持久化
- [ ] Web 仪表板可视化
- [ ] REST API 暴露轨迹查询

---

#### 2.4.2 IO 录制与回放

**目标**: 记录文件系统操作，用于调试和评估

**实现:**
```rust
pub struct IORecorder {
    enabled: bool,
    operations: Arc<Mutex<Vec<IOOperation>>>,
}

pub struct IOOperation {
    pub op_type: IOOpType,
    pub uri: String,
    pub timestamp: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub metadata: HashMap<String, String>,
}

pub enum IOOpType {
    Read,
    Write,
    Delete,
    List,
}

impl CortexFilesystem {
    pub async fn read_with_record(&self, uri: &str) -> Result<String> {
        let content = self.inner_read(uri).await?;
        
        if self.recorder.enabled {
            self.recorder.record(IOOperation {
                op_type: IOOpType::Read,
                uri: uri.to_string(),
                timestamp: Utc::now(),
                content_hash: Some(self.hash(&content)),
                metadata: HashMap::new(),
            });
        }
        
        Ok(content)
    }
}
```

**使用场景:**
```rust
// 测试和评估
recorder.start_recording();
let result = search_engine.search(query).await?;
let operations = recorder.stop_and_get_operations();

// 分析 IO 模式
println!("Total reads: {}", operations.iter().filter(|op| op.op_type == IOOpType::Read).count());
println!("Total writes: {}", operations.iter().filter(|op| op.op_type == IOOpType::Write).count());
```

**实现计划:**
- [ ] 定义 `IORecorder` 和 `IOOperation`
- [ ] 集成到 `CortexFilesystem`
- [ ] 实现录制开关
- [ ] 导出为 JSON/CSV
- [ ] 用于性能分析和优化

---

### 2.5 资源解析增强（优先级：⭐）

#### 2.5.1 丰富解析器生态

**目标**: 参考 OpenViking 扩展解析器类型

**当前解析器:**
- Markdown
- Text
- (基础)

**目标解析器:**
- PDF
- HTML
- Code Repository (支持多语言)
- Office 文档 (Word, Excel, PPT)
- 图片 (OCR + VLM)

**实现框架:**
```rust
pub trait ResourceParser: Send + Sync {
    fn supported_extensions(&self) -> Vec<&str>;
    async fn parse(&self, path: &Path) -> Result<ParseResult>;
}

pub struct ParseResult {
    pub root: ResourceNode,
    pub metadata: HashMap<String, String>,
}

pub struct ResourceNode {
    pub uri: String,
    pub node_type: NodeType,
    pub content: String,
    pub children: Vec<ResourceNode>,
}

// 插件化注册
pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn ResourceParser>>,
}

impl ParserRegistry {
    pub fn register(&mut self, parser: Box<dyn ResourceParser>) {
        for ext in parser.supported_extensions() {
            self.parsers.insert(ext.to_string(), parser.clone());
        }
    }
}
```

**实现计划:**
- [ ] 定义 `ResourceParser` trait
- [ ] 实现 `PDFParser` (使用 pdf-extract)
- [ ] 实现 `HTMLParser` (使用 scraper)
- [ ] 实现 `CodeRepositoryParser` (使用 tree-sitter)
- [ ] 实现插件注册机制

---

## 三、技术债务清理

### 3.1 代码质量提升

- [ ] 增加单元测试覆盖率（目标 80%+）
- [ ] 增加集成测试
- [ ] 性能基准测试自动化
- [ ] 代码静态分析 (clippy --all-features)
- [ ] 依赖安全扫描

### 3.2 文档完善

- [ ] 英文文档补充
- [ ] 架构设计文档
- [ ] API 参考文档自动生成
- [ ] 最佳实践指南
- [ ] 故障排查指南

### 3.3 CI/CD 优化

- [ ] 自动发布 Crate
- [ ] Docker 镜像自动构建
- [ ] 性能回归检测
- [ ] 兼容性测试矩阵

---

## 四、实施路线图

### 阶段一：核心检索升级（1-2个月）

**目标**: 实现目录递归检索和混合向量检索

- ✅ 定义核心数据结构
- ✅ 实现 HierarchicalRetriever
- ✅ 实现分数传播算法
- ✅ 集成混合向量检索
- ✅ 编写测试和基准
- ✅ 文档更新

### 阶段二：会话管理增强（1个月）

**目标**: 实现会话压缩归档和记忆去重

- ✅ 实现自动压缩触发
- ✅ 实现归档写入和管理
- ✅ 实现记忆去重
- ✅ 扩展记忆分类（Profile/Pattern）
- ✅ 编写测试
- ✅ 文档更新

### 阶段三：可观测性和分层优化（1个月）

**目标**: 增强可观测性和分层内存策略

- ✅ 实现检索轨迹记录
- ✅ 实现 IO 录制
- ✅ 实现主动生成策略
- ✅ 实现批量抽象获取
- ✅ Web 仪表板集成
- ✅ 文档更新

### 阶段四：资源解析和生态（1-2个月）

**目标**: 丰富解析器和集成生态

- ✅ 实现多种解析器
- ✅ 插件化架构
- ✅ MCP 集成增强
- ✅ Rig 集成增强
- ✅ 示例和教程

### 阶段五：性能优化和发布（持续）

**目标**: 性能调优和稳定性提升

- ✅ 性能基准对比
- ✅ 内存优化
- ✅ 并发优化
- ✅ 代码质量提升
- ✅ 文档完善
- ✅ 正式发布 3.0

---

## 五、预期成果

### 5.1 性能指标

| 指标 | 当前 (2.x) | 目标 (3.0) | 提升 |
|------|-----------|-----------|------|
| Recall@1 | 93.33% | 95%+ | +1.67pp |
| MRR | 93.72% | 95%+ | +1.28pp |
| NDCG@5 | 80.73% | 85%+ | +4.27pp |
| 检索延迟 | ~50ms | ~60ms | -10ms (递归检索成本) |
| 索引吞吐 | ~1000/s | ~1200/s | +20% |

### 5.2 功能完整性

- ✅ 目录递归检索
- ✅ 混合向量检索
- ✅ 意图分析
- ✅ 会话压缩归档
- ✅ 记忆去重合并
- ✅ 六分类记忆
- ✅ 检索轨迹可视化
- ✅ IO 录制与回放
- ✅ 丰富解析器生态

### 5.3 生态完整性

- ✅ REST API 2.0
- ✅ MCP Server
- ✅ Rig Framework 集成
- ✅ Web 仪表板
- ✅ CLI 工具
- ✅ Docker 镜像
- ✅ 完整文档

---

## 六、风险与应对

### 6.1 技术风险

**风险**: 递归检索增加复杂度和延迟

**应对**:
- 收敛检测早停
- 可配置最大深度
- 缓存优化
- 提供简化模式开关

**风险**: 会话压缩可能丢失信息

**应对**:
- 归档完整保留原始消息
- LLM 摘要质量监控
- 可配置压缩策略
- 用户可手动关闭

### 6.2 兼容性风险

**风险**: 3.0 可能不兼容 2.x

**应对**:
- 提供数据迁移脚本
- 保持配置向后兼容
- 文档迁移指南
- 长期支持 2.x LTS

### 6.3 性能风险

**风险**: 新功能可能影响性能

**应对**:
- 持续性能基准测试
- 性能回归检测
- 可选功能开关
- 性能调优

---

## 七、总结

### 7.1 核心价值

Cortex-Memory 3.0 将融合:

1. **Rust 高性能优势**: 保持性能领先
2. **OpenViking 先进架构**: 引入递归检索、分层管理
3. **完整生态**: REST + MCP + Web + CLI
4. **易用性**: 简化部署，降低门槛
5. **企业就绪**: 多租户、可观测、可运维

### 7.2 竞争力提升

- ✅ **性能**: 继续保持 Rust 性能优势
- ✅ **精度**: 引入递归检索提升检索质量
- ✅ **智能**: 意图分析、去重合并
- ✅ **效率**: 会话压缩控制成本
- ✅ **可观测**: 轨迹记录、IO 回放
- ✅ **生态**: 最完整的集成生态

### 7.3 长期愿景

**Cortex-Memory** 将成为:
- AI 应用的 **首选记忆基础设施**
- 开源社区的 **性能标杆**
- 企业级应用的 **可靠选择**

---

**Let's build the future of AI memory together! 🚀**
