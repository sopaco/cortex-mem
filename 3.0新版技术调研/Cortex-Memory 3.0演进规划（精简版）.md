# Cortex-Memory 3.0 演进规划（精简版）

> 轻量化、高性能、智能化的 AI 上下文数据库

---

## 一、核心定位与原则

### 1.1 设计原则

✅ **必须坚持:**
- **轻量至上**: 零额外依赖，单机部署，开箱即用
- **性能卓越**: Rust 原生性能，保持 93%+ Recall@1
- **Token 高效**: L0 < 2K，智能分层加载
- **简洁易用**: 配置简单，文档完善

❌ **明确不做:**
- 分布式存储（保持单机简洁性）
- 操作历史回溯（避免复杂性）
- 企业级审计日志（聚焦核心功能）

### 1.2 核心目标

1. **修复当前问题** (优先级最高)
2. **引入先进架构** (借鉴 OpenViking)
3. **保持竞争优势** (轻量、性能、生态)

---

## 二、当前问题修复（阶段 0，必须优先完成）

### 问题 1: 三层文件缺失

**现状**: 不是每个目录都有 `.abstract` 和 `.overview`

**解决方案**: 渐进式主动生成

```rust
impl AutoIndexer {
    /// 后台扫描并生成缺失的 L0/L1
    pub async fn ensure_all_layers(&self) -> Result<GenerationStats> {
        let directories = self.scan_all_directories().await?;
        let missing = self.filter_missing_layers(&directories).await?;
        
        // 分批生成，避免过载
        for batch in missing.chunks(10) {
            for dir in batch {
                self.generate_layers_for_directory(dir).await?;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Ok(stats)
    }
}
```

**配置**:
```toml
[layers.generation]
enable_progressive_generation = true
batch_size = 10
delay_ms = 2000
auto_generate_on_startup = true
```

**验收标准**:
- [ ] 100% 目录拥有 L0/L1 文件
- [ ] CLI 命令: `cortex-mem-cli layers ensure-all`
- [ ] 启动时自动检查并补全

---

### 问题 2: .abstract 过大

**现状**: 有时接近 5K，应控制在 500-2K

**解决方案**: 强化 Prompt + 后处理截断

```rust
impl LayerGenerator {
    async fn generate_abstract_v2(&self, content: &str) -> Result<String> {
        let prompt = format!(
            r#"为以下内容生成简洁摘要。
            
【严格要求】
- 最多 400 tokens（约 2000 字符）
- 1-3 个完整句子
- 提炼核心要点，删除细节

【内容】
{content}

仅返回摘要文本。"#
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        
        // 强制截断到 2K
        let result = self.enforce_limit(response, 2000)?;
        Ok(result)
    }
    
    fn enforce_limit(&self, text: String, max_chars: usize) -> Result<String> {
        if text.len() <= max_chars {
            return Ok(text);
        }
        
        // 截断到最后一个句号
        if let Some(pos) = text[..max_chars].rfind(|c| c == '。' || c == '.') {
            return Ok(text[..=pos].to_string());
        }
        
        Ok(format!("{}...", &text[..max_chars-3]))
    }
}
```

**配置**:
```toml
[layers.abstract]
max_tokens = 400
max_chars = 2000
target_sentences = 2
```

**验收标准**:
- [ ] 100% 的 `.abstract` 文件 < 2K 字符
- [ ] Prompt 模板更新
- [ ] 现有文件重新生成

---

### 问题 3: 性能优化

**现状**: 查询时间较长

**解决方案**: 并发 + 缓存

#### 优化 1: 并发 L0/L1/L2 读取

```rust
impl LayerReader {
    pub async fn read_all_layers(&self, uris: &[String]) -> Result<HashMap<String, Layers>> {
        let tasks = uris.iter().map(|uri| async move {
            let (l0, l1, l2) = tokio::join!(
                self.read_abstract(uri),
                self.read_overview(uri),
                self.read_content(uri),
            );
            (uri.clone(), Layers { l0, l1, l2 })
        });
        
        let results = futures::future::join_all(tasks).await;
        Ok(results.into_iter().collect())
    }
}

// 性能: 100ms -> 50ms
```

#### 优化 2: Embedding 缓存

```rust
pub struct CachedEmbeddingClient {
    inner: Arc<dyn EmbeddingClient>,
    cache: Arc<Mutex<LruCache<String, Vec<f32>>>>,
}

impl CachedEmbeddingClient {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // 检查缓存
        if let Some(vector) = self.cache.lock().await.get(text) {
            return Ok(vector.clone());
        }
        
        // 生成并缓存
        let vector = self.inner.embed(text).await?;
        self.cache.lock().await.put(text.to_string(), vector.clone());
        Ok(vector)
    }
}

// 性能: 重复查询从 50ms -> 0.1ms
```

#### 优化 3: 批量 Embedding

```rust
impl EmbeddingClient {
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // 利用 OpenAI API 批量接口
        let response = self.client.post("/embeddings")
            .json(&json!({
                "model": self.model,
                "input": texts,
            }))
            .send().await?;
        
        Ok(response.json::<EmbeddingResponse>()?.vectors)
    }
}

// 性能: 10 个查询从 500ms -> 80ms
```

**配置**:
```toml
[performance]
enable_concurrent_reading = true
enable_embedding_cache = true
embedding_cache_size = 1000
enable_batch_embedding = true
batch_size = 32
```

**验收标准**:

| 指标 | 当前 | 目标 |
|------|------|------|
| 单次查询 | ~200ms | ~80ms |
| 重复查询 | ~200ms | ~10ms |
| 批量查询(10个) | ~2000ms | ~300ms |

---

## 三、核心功能演进（阶段 1-3）

### 阶段 1: 检索引擎升级（1-2 个月）

#### 功能 1.1: 目录递归检索

**目标**: 从平铺式升级为层级化检索

**核心算法**:
```rust
pub struct HierarchicalRetriever {
    vector_store: Arc<dyn VectorStore>,
    config: HierarchicalConfig,
}

impl HierarchicalRetriever {
    pub async fn retrieve(&self, query: &TypedQuery) -> Result<Vec<SearchResult>> {
        // 1. 全局搜索定位高分目录
        let top_dirs = self.global_search(query, 3).await?;
        
        // 2. 递归搜索子目录（最多 3 层）
        let mut candidates = vec![];
        for dir in top_dirs {
            let sub_results = self.recursive_search(&dir, query, 3).await?;
            candidates.extend(sub_results);
        }
        
        // 3. 分数传播
        let scored = self.apply_score_propagation(candidates);
        
        // 4. 排序返回
        Ok(self.sort_and_limit(scored, query.limit))
    }
    
    fn apply_score_propagation(&self, candidates: Vec<Candidate>) -> Vec<Candidate> {
        candidates.into_iter().map(|mut c| {
            c.final_score = 0.5 * c.current_score + 0.5 * c.parent_score;
            c
        }).collect()
    }
}
```

**配置**:
```toml
[search.hierarchical]
enabled = true
max_depth = 3
score_propagation_alpha = 0.5
global_search_topk = 3
```

**验收标准**:
- [ ] Recall@1 提升到 95%+
- [ ] 单元测试覆盖率 > 80%
- [ ] 性能基准: 检索延迟 < 100ms

---

#### 功能 1.2: 意图分析（简化版）

**目标**: 自动分析查询意图，生成 2-3 个类型化查询

**核心实现**:
```rust
pub struct LightweightIntentAnalyzer {
    llm_client: Arc<dyn LLMClient>,
}

impl LightweightIntentAnalyzer {
    pub async fn analyze(&self, query: &str) -> Result<Vec<TypedQuery>> {
        let prompt = format!(
            r#"分析查询，生成 1-3 个类型化查询。

【查询】{}

【返回 JSON】
[
  {{"query": "优化后的查询", "context_type": "memory|resource|agent"}},
  ...
]"#,
            query
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        let queries: Vec<TypedQuery> = serde_json::from_str(&response)?;
        Ok(queries.into_iter().take(3).collect())
    }
}
```

**验收标准**:
- [ ] 查询精准度提升 15%+
- [ ] 单次 LLM 调用 < 1s
- [ ] 可配置开关

---

### 阶段 2: 记忆管理增强（1 个月）

#### 功能 2.1: 记忆分类扩展

**目标**: 新增 Profile 和 Pattern 分类

**当前**: Preference, Entity, Event, Case

**目标**: 
```rust
pub enum MemoryCategory {
    // 用户记忆
    Profile,      // 🆕 用户画像
    Preference,   // 用户偏好
    Entity,       // 实体记忆
    Event,        // 事件记录
    
    // Agent 记忆
    Case,         // 案例库
    Pattern,      // 🆕 模式库
}
```

**Profile 示例**:
```markdown
# 用户画像

## 基本信息
- 职业: 软件工程师
- 技术栈: Rust, Python
- 兴趣: AI, 开源

## 工作习惯
- 偏好简洁高效的工具
- 重视代码质量和性能
```

**Pattern 示例**:
```markdown
# 模式: 调试性能问题的流程

## 适用场景
应用响应慢、CPU/内存占用高

## 步骤
1. 使用 perf 分析 CPU 热点
2. 检查 allocator 性能
3. 添加 tracing 日志
4. 对比优化前后基准测试
```

**验收标准**:
- [ ] Profile 自动提取和合并
- [ ] Pattern 独立存储
- [ ] 单元测试覆盖

---

#### 功能 2.2: 记忆去重优化

**目标**: 智能检测和合并重复记忆

**核心实现**:
```rust
pub struct MemoryDeduplicator {
    vector_store: Arc<dyn VectorStore>,
    llm_client: Arc<dyn LLMClient>,
}

impl MemoryDeduplicator {
    pub async fn check_duplicate(&self, candidate: &CandidateMemory) -> Result<DeduplicationResult> {
        // 1. 向量相似度检索
        let vector = self.embed(&candidate.abstract_text).await?;
        let similar = self.vector_store.search(vector, 5).await?
            .into_iter()
            .filter(|r| r.score > 0.85)
            .collect::<Vec<_>>();
        
        if similar.is_empty() {
            return Ok(DeduplicationResult::NoDuplicate);
        }
        
        // 2. LLM 精确判断
        for existing in similar {
            if self.is_duplicate_by_llm(candidate, &existing).await? {
                return Ok(DeduplicationResult::Duplicate(existing.uri));
            }
        }
        
        Ok(DeduplicationResult::NoDuplicate)
    }
    
    pub async fn merge_memory(&self, existing: &str, new: &str) -> Result<String> {
        let prompt = format!(
            "合并两个记忆，保留完整信息：\n现有: {}\n新增: {}",
            existing, new
        );
        
        let merged = self.llm_client.generate(&prompt).await?;
        Ok(merged)
    }
}
```

**验收标准**:
- [ ] 重复检测准确率 > 90%
- [ ] Profile/Preference 自动合并
- [ ] Entity/Event/Case/Pattern 独立保存

---

### 阶段 3: 可观测性增强（可选，按需实施）

#### 功能 3.1: 检索轨迹记录（轻量版）

**目标**: 记录关键检索步骤，用于调试

**核心实现**:
```rust
pub struct SearchTrace {
    pub query: String,
    pub steps: Vec<String>,  // 简化为文本描述
    pub final_count: usize,
    pub duration_ms: u64,
}

impl HierarchicalRetriever {
    pub async fn retrieve_with_trace(&self, query: &TypedQuery) -> Result<(Vec<SearchResult>, SearchTrace)> {
        let mut trace = SearchTrace::new(&query.query);
        
        trace.add_step("全局搜索: 找到 3 个高分目录");
        trace.add_step("递归搜索: 探索 12 个子目录");
        trace.add_step("分数传播: 调整 45 个候选");
        
        // ... 执行检索 ...
        
        Ok((results, trace))
    }
}
```

**存储**:
```rust
// 可选：保存到文件
let trace_path = format!("cortex://session/{}/traces/search_{}.json", session_id, uuid);
filesystem.write(&trace_path, &serde_json::to_string(&trace)?).await?;
```

**验收标准**:
- [ ] 可选开关控制
- [ ] 最小化性能影响 (< 5ms)
- [ ] JSON 格式导出

---

## 四、实施路线图

### 时间规划

| 阶段 | 内容 | 时间 | 验收标准 |
|------|------|------|----------|
| **阶段 0** | 修复当前问题 | 2 周 | 三层文件 100%覆盖<br/>.abstract < 2K<br/>查询延迟 < 80ms |
| **阶段 1** | 检索引擎升级 | 6 周 | Recall@1 > 95%<br/>递归检索生效<br/>意图分析集成 |
| **阶段 2** | 记忆管理增强 | 4 周 | 六分类支持<br/>去重准确率 > 90% |
| **阶段 3** | 可观测性（可选） | 2 周 | 轨迹记录功能<br/>性能影响 < 5% |

### 里程碑

**M0**: 问题修复完成（第 2 周）
- 三层文件补全
- .abstract 大小控制
- 性能优化生效

**M1**: 递归检索上线（第 8 周）
- HierarchicalRetriever 实现
- 意图分析集成
- 性能基准达标

**M2**: 记忆增强上线（第 12 周）
- 六分类记忆支持
- 去重合并功能
- 完整测试覆盖

**M3**: 3.0 正式发布（第 14 周）
- 所有功能完成
- 文档更新
- 性能报告

---

## 五、技术规范

### 5.1 代码规范

```rust
// 所有公开 API 必须有文档注释
/// 检索记忆，支持层级化递归检索
///
/// # Arguments
/// * `query` - 查询文本
/// * `options` - 检索选项
///
/// # Returns
/// 排序后的搜索结果列表
pub async fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>>;

// 配置必须有默认值
impl Default for HierarchicalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_depth: 3,
            score_propagation_alpha: 0.5,
        }
    }
}

// 错误处理必须明确
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Vector store error: {0}")]
    VectorStore(#[from] VectorStoreError),
    
    #[error("LLM error: {0}")]
    LLM(#[from] LLMError),
}
```

### 5.2 性能要求

| 操作 | 目标延迟 | 并发 |
|------|----------|------|
| 单次查询 | < 80ms | 支持 |
| 批量查询 (10个) | < 300ms | 必需 |
| Embedding 缓存命中 | < 1ms | - |
| L0/L1/L2 读取 | < 50ms | 并发 |

### 5.3 测试要求

- 单元测试覆盖率 > 80%
- 集成测试覆盖核心流程
- 性能基准测试自动化
- 每个 PR 必须通过 CI

---

## 六、风险与应对

### 风险 1: 递归检索增加延迟

**应对**:
- 限制最大深度为 3
- 早停机制
- 可配置开关

### 风险 2: 性能优化可能引入 Bug

**应对**:
- 充分测试
- 灰度发布
- 性能监控

### 风险 3: LLM 去重判断不准确

**应对**:
- 向量相似度初筛
- 调整阈值
- 提供手动干预

---

## 七、成功标准

### 7.1 性能指标

| 指标 | 2.x | 3.0 目标 |
|------|-----|---------|
| Recall@1 | 93.33% | 95%+ |
| 查询延迟 | ~200ms | ~80ms |
| Token 消耗 | 可变 | < 2K/abstract |

### 7.2 功能完整性

- ✅ 三层文件 100% 覆盖
- ✅ 目录递归检索
- ✅ 意图分析
- ✅ 六分类记忆
- ✅ 智能去重
- ✅ 性能优化

### 7.3 生态完整性

- ✅ REST API 2.0
- ✅ MCP Server
- ✅ Web 仪表板
- ✅ CLI 工具
- ✅ 完整文档

---

## 八、总结

### 核心亮点

1. **修复遗留问题**: 三层文件、大小控制、性能优化
2. **引入先进架构**: 递归检索、智能去重
3. **保持轻量化**: 零额外依赖，简单部署
4. **保持高性能**: Rust 原生，< 80ms 查询延迟

### 竞争优势

- 🚀 **最轻量**: 单机部署，零复杂度
- ⚡ **最快速**: Rust 性能，缓存优化
- 🧠 **最智能**: 递归检索，意图分析
- 📊 **最完整**: REST + MCP + Web + CLI

**Cortex-Memory 3.0 = 轻量 + 性能 + 智能！🎯**
