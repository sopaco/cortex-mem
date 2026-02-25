# Cortex-Memory 3.0 详细开发计划

> 按阶段拆解的详细开发任务、交付物和验收标准

---

## 阶段 0: 当前问题修复（2周，必须优先完成）

### Sprint 0.1: 三层文件补全（1周）

#### Task 0.1.1: 目录扫描与检测

**负责模块**: `cortex-mem-core/src/automation/auto_indexer.rs`

**任务描述**:
1. 实现 `scan_all_directories()` 方法，递归扫描所有维度的目录
2. 实现 `has_layers()` 方法，检测目录是否拥有 `.abstract` 和 `.overview`
3. 实现 `filter_missing_layers()` 方法，过滤出缺失的目录

**代码骨架**:
```rust
// cortex-mem-core/src/automation/layer_generator.rs (新文件)
pub struct LayerGenerator {
    filesystem: Arc<CortexFilesystem>,
    llm_client: Arc<dyn LLMClient>,
    config: LayerGenerationConfig,
}

pub struct LayerGenerationConfig {
    pub batch_size: usize,
    pub delay_ms: u64,
    pub auto_generate_on_startup: bool,
}

impl LayerGenerator {
    /// 扫描所有目录
    pub async fn scan_all_directories(&self) -> Result<Vec<String>> {
        let mut directories = vec![];
        
        // 扫描四个维度
        for scope in &["session", "user", "agent", "resources"] {
            let scope_dirs = self.scan_scope(scope).await?;
            directories.extend(scope_dirs);
        }
        
        Ok(directories)
    }
    
    async fn scan_scope(&self, scope: &str) -> Result<Vec<String>> {
        // TODO: 递归扫描 cortex://{scope}/ 下的所有目录
        unimplemented!()
    }
    
    /// 检测目录是否有 L0/L1 文件
    pub async fn has_layers(&self, uri: &str) -> Result<bool> {
        let abstract_path = format!("{}/.abstract", uri);
        let overview_path = format!("{}/.overview", uri);
        
        Ok(
            self.filesystem.exists(&abstract_path).await? &&
            self.filesystem.exists(&overview_path).await?
        )
    }
    
    /// 过滤出缺失 L0/L1 的目录
    pub async fn filter_missing_layers(&self, dirs: &[String]) -> Result<Vec<String>> {
        let mut missing = vec![];
        for dir in dirs {
            if !self.has_layers(dir).await? {
                missing.push(dir.clone());
            }
        }
        Ok(missing)
    }
}
```

**Deliverables**:
- [ ] `layer_generator.rs` 文件创建
- [ ] 单元测试: `test_scan_all_directories()`
- [ ] 单元测试: `test_has_layers()`
- [ ] 单元测试: `test_filter_missing_layers()`

**验收标准**:
- 能正确扫描所有维度的目录
- 准确检测 L0/L1 文件是否存在
- 测试覆盖率 > 85%

---

#### Task 0.1.2: 渐进式生成实现

**任务描述**:
1. 实现 `ensure_all_layers()` 方法，分批生成缺失的 L0/L1
2. 添加批次间延迟，避免 LLM API 限流
3. 实现进度跟踪和统计

**代码骨架**:
```rust
impl LayerGenerator {
    /// 确保所有目录拥有 L0/L1
    pub async fn ensure_all_layers(&self) -> Result<GenerationStats> {
        info!("开始扫描目录...");
        let directories = self.scan_all_directories().await?;
        
        info!("检测缺失的 L0/L1...");
        let missing = self.filter_missing_layers(&directories).await?;
        
        info!("发现 {} 个目录缺失 L0/L1，开始生成...", missing.len());
        
        let mut stats = GenerationStats {
            total: missing.len(),
            generated: 0,
            failed: 0,
        };
        
        // 分批生成
        for (batch_idx, batch) in missing.chunks(self.config.batch_size).enumerate() {
            info!("处理批次 {}/{}", batch_idx + 1, (missing.len() + self.config.batch_size - 1) / self.config.batch_size);
            
            for dir in batch {
                match self.generate_layers_for_directory(dir).await {
                    Ok(_) => {
                        stats.generated += 1;
                        info!("✓ 生成成功: {}", dir);
                    }
                    Err(e) => {
                        stats.failed += 1;
                        warn!("✗ 生成失败: {} - {}", dir, e);
                    }
                }
            }
            
            // 批次间延迟
            if batch_idx < (missing.len() + self.config.batch_size - 1) / self.config.batch_size - 1 {
                tokio::time::sleep(Duration::from_millis(self.config.delay_ms)).await;
            }
        }
        
        info!("生成完成: 成功 {}, 失败 {}", stats.generated, stats.failed);
        Ok(stats)
    }
    
    /// 为单个目录生成 L0/L1
    async fn generate_layers_for_directory(&self, uri: &str) -> Result<()> {
        // 1. 读取目录内容
        let entries = self.filesystem.list(uri).await?;
        
        // 2. 聚合内容（读取子文件）
        let content = self.aggregate_directory_content(uri, &entries).await?;
        
        // 3. 生成 L0 抽象
        let abstract_text = self.generate_abstract(&content).await?;
        
        // 4. 生成 L1 概览
        let overview = self.generate_overview(&content).await?;
        
        // 5. 写入文件
        self.filesystem.write(&format!("{}/.abstract", uri), &abstract_text).await?;
        self.filesystem.write(&format!("{}/.overview", uri), &overview).await?;
        
        Ok(())
    }
    
    /// 聚合目录内容
    async fn aggregate_directory_content(&self, uri: &str, entries: &[String]) -> Result<String> {
        // TODO: 读取子文件内容，拼接成完整文本
        // 注意：需要合理截断，避免超出 LLM 上下文限制
        unimplemented!()
    }
}
```

**Deliverables**:
- [ ] `ensure_all_layers()` 实现
- [ ] `generate_layers_for_directory()` 实现
- [ ] 单元测试: `test_ensure_all_layers()`
- [ ] 集成测试: 模拟缺失目录生成

**验收标准**:
- 能分批生成所有缺失的 L0/L1
- 批次间延迟生效
- 统计信息准确
- 失败后继续处理其他目录

---

#### Task 0.1.3: CLI 集成

**任务描述**:
1. 添加 `layers ensure-all` 命令
2. 添加 `layers status` 命令查看进度
3. 支持 `--tenant` 参数

**代码骨架**:
```rust
// cortex-mem-cli/src/commands/layers.rs (新文件)
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct LayersCommand {
    #[command(subcommand)]
    pub action: LayersAction,
}

#[derive(Subcommand)]
pub enum LayersAction {
    /// 确保所有目录拥有 L0/L1 文件
    EnsureAll {
        #[arg(long)]
        tenant: Option<String>,
    },
    
    /// 查看层级生成状态
    Status {
        #[arg(long)]
        tenant: Option<String>,
    },
}

pub async fn handle_layers_command(cmd: LayersCommand, config: &Config) -> Result<()> {
    match cmd.action {
        LayersAction::EnsureAll { tenant } => {
            println!("开始检查并生成缺失的 L0/L1 文件...");
            
            let layer_generator = LayerGenerator::new(/* ... */);
            let stats = layer_generator.ensure_all_layers().await?;
            
            println!("\n生成完成:");
            println!("  总计: {}", stats.total);
            println!("  成功: {}", stats.generated);
            println!("  失败: {}", stats.failed);
            
            Ok(())
        }
        
        LayersAction::Status { tenant } => {
            // TODO: 显示当前状态（多少目录有/没有 L0/L1）
            unimplemented!()
        }
    }
}
```

**Deliverables**:
- [ ] `layers.rs` 命令文件
- [ ] 集成到主 CLI
- [ ] 用户文档更新

**验收标准**:
- `cortex-mem-cli layers ensure-all` 能正常运行
- 输出清晰的进度和统计信息
- 支持多租户隔离

---

#### Task 0.1.4: 启动时自动检查

**任务描述**:
1. 在 `AutomationManager` 启动时触发检查
2. 支持配置开关

**代码骨架**:
```rust
// cortex-mem-core/src/automation/manager.rs
impl AutomationManager {
    pub async fn start(&self) -> Result<()> {
        // 启动现有自动化...
        
        // 检查并生成缺失的 L0/L1
        if self.config.layer_generation.auto_generate_on_startup {
            info!("启动时自动检查并生成缺失的 L0/L1...");
            tokio::spawn({
                let layer_generator = self.layer_generator.clone();
                async move {
                    if let Err(e) = layer_generator.ensure_all_layers().await {
                        error!("自动生成 L0/L1 失败: {}", e);
                    }
                }
            });
        }
        
        Ok(())
    }
}
```

**Deliverables**:
- [ ] `AutomationManager` 集成
- [ ] 配置项添加
- [ ] 日志输出

**验收标准**:
- 启动时自动检查（如果配置启用）
- 不阻塞主启动流程（后台异步）
- 失败不影响应用启动

---

### Sprint 0.2: .abstract 大小控制（0.5周）

#### Task 0.2.1: 更新 Prompt 模板

**任务描述**:
1. 强化 Prompt 约束，明确长度要求
2. 添加后处理截断逻辑

**代码骨架**:
```rust
// cortex-mem-core/src/layers/generator.rs
pub struct AbstractConfig {
    pub max_tokens: usize,   // 默认 400
    pub max_chars: usize,    // 默认 2000
    pub target_sentences: usize, // 默认 2
}

impl LayerGenerator {
    async fn generate_abstract_v2(&self, content: &str, category: &str) -> Result<String> {
        let prompt = format!(
            r#"请为以下{category}内容生成简洁的摘要。

【严格要求】
- 最多 {max_tokens} tokens（约 {max_chars} 字符）
- {target_sentences} 个完整句子
- 提炼核心要点，删除细节描述
- 使用精炼语言，避免冗余

【内容】
{content}

【输出格式】
仅返回摘要文本，不要包含任何前缀、后缀或解释。"#,
            category = category,
            max_tokens = self.config.abstract_config.max_tokens,
            max_chars = self.config.abstract_config.max_chars,
            target_sentences = self.config.abstract_config.target_sentences,
            content = self.truncate_content(content, 4000),
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        
        // 强制执行长度限制
        let abstract_text = self.enforce_limits(response)?;
        
        Ok(abstract_text)
    }
    
    fn enforce_limits(&self, text: String) -> Result<String> {
        let mut result = text.trim().to_string();
        let max_chars = self.config.abstract_config.max_chars;
        
        if result.len() <= max_chars {
            return Ok(result);
        }
        
        // 截断到最后一个句号/问号/叹号
        if let Some(pos) = result[..max_chars]
            .rfind(|c| c == '。' || c == '.' || c == '?' || c == '!' || c == '！' || c == '？')
        {
            result.truncate(pos + '。'.len_utf8());
        } else {
            result.truncate(max_chars - 3);
            result.push_str("...");
        }
        
        Ok(result)
    }
    
    fn truncate_content(&self, content: &str, max_chars: usize) -> String {
        if content.len() <= max_chars {
            content.to_string()
        } else {
            format!("{}...", &content[..max_chars])
        }
    }
}
```

**Deliverables**:
- [ ] Prompt 模板更新
- [ ] `enforce_limits()` 实现
- [ ] 单元测试: `test_enforce_limits()`
- [ ] 单元测试: `test_generate_abstract_v2()`

**验收标准**:
- 100% 的新生成 `.abstract` < 2K 字符
- Prompt 清晰约束长度
- 后处理截断正确

---

#### Task 0.2.2: 现有文件重新生成

**任务描述**:
1. 扫描所有现有 `.abstract` 文件
2. 检测超大文件（> 2K）
3. 重新生成

**代码骨架**:
```rust
impl LayerGenerator {
    /// 重新生成所有超大的 .abstract 文件
    pub async fn regenerate_oversized_abstracts(&self) -> Result<RegenerationStats> {
        let directories = self.scan_all_directories().await?;
        let mut stats = RegenerationStats::default();
        
        for dir in directories {
            let abstract_path = format!("{}/.abstract", dir);
            
            if let Ok(content) = self.filesystem.read(&abstract_path).await {
                if content.len() > self.config.abstract_config.max_chars {
                    info!("重新生成超大 .abstract: {} ({} 字符)", dir, content.len());
                    
                    match self.generate_layers_for_directory(&dir).await {
                        Ok(_) => stats.regenerated += 1,
                        Err(e) => {
                            stats.failed += 1;
                            warn!("重新生成失败: {} - {}", dir, e);
                        }
                    }
                }
            }
        }
        
        Ok(stats)
    }
}
```

**Deliverables**:
- [ ] `regenerate_oversized_abstracts()` 实现
- [ ] CLI 命令: `layers regenerate-oversized`
- [ ] 执行脚本文档

**验收标准**:
- 所有现有 `.abstract` 文件 < 2K
- 重新生成不破坏原有内容质量

---

### Sprint 0.3: 性能优化（0.5周）

#### Task 0.3.1: 并发 L0/L1/L2 读取

**任务描述**:
1. 实现并发读取接口
2. 集成到搜索流程

**代码骨架**:
```rust
// cortex-mem-core/src/layers/reader.rs
use futures::future::try_join_all;

pub struct LayerBundle {
    pub abstract_text: Option<String>,
    pub overview: Option<String>,
    pub content: Option<String>,
}

impl LayerReader {
    /// 并发读取所有层级
    pub async fn read_all_layers_concurrent(
        &self,
        uris: &[String],
    ) -> Result<HashMap<String, LayerBundle>> {
        let tasks: Vec<_> = uris.iter().map(|uri| {
            let uri = uri.clone();
            let filesystem = self.filesystem.clone();
            
            async move {
                let (l0, l1, l2) = tokio::join!(
                    filesystem.read(&format!("{}/.abstract", uri)),
                    filesystem.read(&format!("{}/.overview", uri)),
                    filesystem.read(&uri),
                );
                
                (uri, LayerBundle {
                    abstract_text: l0.ok(),
                    overview: l1.ok(),
                    content: l2.ok(),
                })
            }
        }).collect();
        
        let results = futures::future::join_all(tasks).await;
        Ok(results.into_iter().collect())
    }
}
```

**Deliverables**:
- [ ] `read_all_layers_concurrent()` 实现
- [ ] 性能基准测试
- [ ] 集成到 `VectorSearchEngine`

**验收标准**:
- 性能提升 30%+ (100ms -> 70ms)
- 并发安全
- 无 deadlock

---

#### Task 0.3.2: Embedding 缓存

**任务描述**:
1. 实现 LRU 缓存层
2. 包装现有 `EmbeddingClient`

**代码骨架**:
```rust
// cortex-mem-core/src/embedding/cached_client.rs
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
}

#[async_trait]
impl EmbeddingClient for CachedEmbeddingClient {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
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
```

**Deliverables**:
- [ ] `CachedEmbeddingClient` 实现
- [ ] 配置支持
- [ ] 单元测试
- [ ] 性能基准测试

**验收标准**:
- 重复查询从 50ms -> 0.1ms
- 缓存命中率监控
- 内存占用可控

---

#### Task 0.3.3: 批量 Embedding

**任务描述**:
1. 扩展 `EmbeddingClient` trait 支持批量接口
2. 实现 OpenAI API 批量调用

**代码骨架**:
```rust
// cortex-mem-core/src/embedding/client.rs
#[async_trait]
pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    
    /// 批量生成 Embedding
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // 默认实现：逐个调用
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}

// cortex-mem-core/src/embedding/openai_client.rs
impl EmbeddingClient for OpenAIEmbeddingClient {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        
        let response = self.client
            .post(&format!("{}/embeddings", self.config.api_base))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&serde_json::json!({
                "model": self.config.model_name,
                "input": texts,
            }))
            .send()
            .await?;
        
        let data: EmbeddingResponse = response.json().await?;
        Ok(data.data.into_iter().map(|d| d.embedding).collect())
    }
}
```

**Deliverables**:
- [ ] `embed_batch()` trait 方法
- [ ] OpenAI 批量实现
- [ ] 集成到搜索流程
- [ ] 性能基准测试

**验收标准**:
- 10 个查询从 500ms -> 100ms
- 支持最多 2048 个批量
- 错误处理完善

---

## 阶段 1: 检索引擎升级（6周）

### Sprint 1.1: 目录递归检索核心（2周）

#### Task 1.1.1: 定义核心数据结构

**代码骨架**:
```rust
// cortex-mem-core/src/search/hierarchical.rs (新文件)
pub struct HierarchicalRetriever {
    vector_store: Arc<dyn VectorStore>,
    embedding_client: Arc<dyn EmbeddingClient>,
    filesystem: Arc<CortexFilesystem>,
    config: HierarchicalConfig,
}

pub struct HierarchicalConfig {
    pub enabled: bool,
    pub max_depth: usize,
    pub score_propagation_alpha: f32,
    pub convergence_rounds: usize,
    pub global_search_topk: usize,
}

pub struct TypedQuery {
    pub query: String,
    pub context_type: ContextType,
    pub target_scope: Option<String>,
    pub limit: usize,
}

pub enum ContextType {
    Memory,
    Resource,
    Agent,
    Session,
}

pub struct HierarchicalResult {
    pub results: Vec<SearchResult>,
    pub trace: Option<SearchTrace>,
}

pub struct SearchTrace {
    pub steps: Vec<String>,
    pub duration_ms: u64,
}
```

**Deliverables**:
- [ ] 数据结构定义
- [ ] 配置默认值
- [ ] 文档注释

---

#### Task 1.1.2: 实现全局搜索

**代码骨架**:
```rust
impl HierarchicalRetriever {
    /// 全局向量搜索，定位高分目录
    async fn global_search(
        &self,
        query: &TypedQuery,
        topk: usize,
    ) -> Result<Vec<DirectoryScore>> {
        // 1. 生成查询向量
        let query_vector = self.embedding_client.embed(&query.query).await?;
        
        // 2. 向量检索（仅检索目录，is_leaf=false）
        let search_opts = SearchOptions {
            limit: topk * 3, // 检索更多候选
            filters: vec![
                ("is_leaf", "false"), // 仅目录
                ("context_type", &query.context_type.to_string()),
            ],
            score_threshold: Some(0.5),
        };
        
        let results = self.vector_store.search(&query_vector, &search_opts).await?;
        
        // 3. 提取目录分数
        let dir_scores: Vec<_> = results.into_iter()
            .map(|r| DirectoryScore {
                uri: r.uri.clone(),
                score: r.score,
                depth: self.calculate_depth(&r.uri),
            })
            .collect();
        
        // 4. 按分数排序，取 topk
        let mut sorted = dir_scores;
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        sorted.truncate(topk);
        
        Ok(sorted)
    }
    
    fn calculate_depth(&self, uri: &str) -> usize {
        uri.split('/').filter(|s| !s.is_empty()).count() - 1
    }
}
```

**Deliverables**:
- [ ] `global_search()` 实现
- [ ] 单元测试
- [ ] 集成测试

---

#### Task 1.1.3: 实现递归搜索

**代码骨架**:
```rust
impl HierarchicalRetriever {
    /// 递归搜索子目录
    async fn recursive_search(
        &self,
        start_dir: &DirectoryScore,
        query: &TypedQuery,
        max_depth: usize,
    ) -> Result<Vec<Candidate>> {
        let mut candidates = vec![];
        let mut to_explore = vec![(start_dir.uri.clone(), start_dir.score, 0)];
        
        while let Some((current_uri, parent_score, depth)) = to_explore.pop() {
            if depth >= max_depth {
                continue;
            }
            
            // 1. 列出子目录
            let children = self.list_children(&current_uri).await?;
            
            // 2. 向量检索子节点
            let child_results = self.search_children(&current_uri, query).await?;
            
            // 3. 应用分数传播
            for result in child_results {
                let propagated_score = self.config.score_propagation_alpha * result.score
                    + (1.0 - self.config.score_propagation_alpha) * parent_score;
                
                if result.is_leaf {
                    // 叶子节点，加入候选
                    candidates.push(Candidate {
                        uri: result.uri.clone(),
                        score: result.score,
                        final_score: propagated_score,
                        parent_uri: current_uri.clone(),
                        depth: depth + 1,
                    });
                } else {
                    // 目录节点，继续递归
                    to_explore.push((result.uri.clone(), propagated_score, depth + 1));
                }
            }
        }
        
        Ok(candidates)
    }
    
    async fn list_children(&self, uri: &str) -> Result<Vec<String>> {
        self.filesystem.list(uri).await
    }
    
    async fn search_children(&self, parent_uri: &str, query: &TypedQuery) -> Result<Vec<SearchResult>> {
        // TODO: 在指定父目录下搜索
        unimplemented!()
    }
}
```

**Deliverables**:
- [ ] `recursive_search()` 实现
- [ ] 单元测试
- [ ] 集成测试
- [ ] 性能基准测试

---

#### Task 1.1.4: 分数传播与排序

**代码骨架**:
```rust
impl HierarchicalRetriever {
    /// 应用分数传播并排序
    fn apply_score_propagation_and_sort(
        &self,
        mut candidates: Vec<Candidate>,
        limit: usize,
    ) -> Vec<SearchResult> {
        // 分数传播已在递归搜索中完成
        
        // 按 final_score 排序
        candidates.sort_by(|a, b| {
            b.final_score.partial_cmp(&a.final_score).unwrap()
        });
        
        // 截断到 limit
        candidates.truncate(limit);
        
        // 转换为 SearchResult
        candidates.into_iter().map(|c| SearchResult {
            uri: c.uri,
            score: c.final_score,
            // ... 其他字段
        }).collect()
    }
}
```

**Deliverables**:
- [ ] 排序逻辑
- [ ] 单元测试

---

### Sprint 1.2: 意图分析集成（2周）

#### Task 1.2.1: 实现轻量级意图分析器

**代码骨架**:
```rust
// cortex-mem-core/src/search/intent_analyzer.rs (新文件)
pub struct LightweightIntentAnalyzer {
    llm_client: Arc<dyn LLMClient>,
    config: IntentAnalyzerConfig,
}

pub struct IntentAnalyzerConfig {
    pub enabled: bool,
    pub max_queries: usize,
    pub use_recent_context: bool,
    pub context_window_messages: usize,
}

impl LightweightIntentAnalyzer {
    pub async fn analyze(
        &self,
        query: &str,
        recent_context: Option<&str>,
    ) -> Result<Vec<TypedQuery>> {
        if !self.config.enabled {
            // 禁用时，返回单一查询
            return Ok(vec![TypedQuery {
                query: query.to_string(),
                context_type: ContextType::Resource,
                target_scope: None,
                limit: 10,
            }]);
        }
        
        let prompt = format!(
            r#"分析用户查询，判断需要搜索的内容类型。

【查询】
{}

【最近上下文】
{}

【要求】
返回 JSON 数组，每个元素包含：
- query: 优化后的查询文本
- context_type: "memory" | "resource" | "agent" | "session"
- target_scope: 可选的目标范围（如 "user/preferences"）

最多返回 {} 个查询。"#,
            query,
            recent_context.unwrap_or("无"),
            self.config.max_queries
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        
        // 解析 JSON
        let queries: Vec<TypedQuery> = serde_json::from_str(&response)
            .map_err(|e| Error::ParseError(format!("Failed to parse intent analysis response: {}", e)))?;
        
        // 限制数量
        Ok(queries.into_iter().take(self.config.max_queries).collect())
    }
}
```

**Deliverables**:
- [ ] `LightweightIntentAnalyzer` 实现
- [ ] Prompt 模板
- [ ] 单元测试
- [ ] 集成测试

---

#### Task 1.2.2: 集成到搜索流程

**代码骨架**:
```rust
// cortex-mem-core/src/search/engine.rs
impl VectorSearchEngine {
    pub async fn search_with_intent(
        &self,
        query: &str,
        recent_context: Option<&str>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 1. 意图分析
        let typed_queries = self.intent_analyzer.analyze(query, recent_context).await?;
        
        // 2. 并发检索
        let search_tasks: Vec<_> = typed_queries.iter().map(|tq| {
            self.hierarchical_retriever.retrieve(tq)
        }).collect();
        
        let results = futures::future::try_join_all(search_tasks).await?;
        
        // 3. 合并结果（去重、排序）
        let merged = self.merge_results(results);
        
        Ok(merged)
    }
    
    fn merge_results(&self, results: Vec<HierarchicalResult>) -> Vec<SearchResult> {
        let mut all_results = vec![];
        for r in results {
            all_results.extend(r.results);
        }
        
        // 去重（按 URI）
        let mut seen = HashSet::new();
        let unique: Vec<_> = all_results.into_iter()
            .filter(|r| seen.insert(r.uri.clone()))
            .collect();
        
        // 按分数排序
        let mut sorted = unique;
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        sorted
    }
}
```

**Deliverables**:
- [ ] `search_with_intent()` 实现
- [ ] `merge_results()` 实现
- [ ] 集成测试
- [ ] 性能基准测试

---

### Sprint 1.3: 测试与优化（2周）

#### Task 1.3.1: LOMOCO 基准测试

**任务描述**:
1. 运行 LOMOCO 评估框架
2. 对比 2.x 和 3.0 性能
3. 调优参数

**Deliverables**:
- [ ] 基准测试脚本
- [ ] 性能报告文档
- [ ] 参数调优记录

**验收标准**:
- Recall@1 > 95%
- MRR > 95%
- NDCG@5 > 85%

---

#### Task 1.3.2: 性能优化

**任务描述**:
1. 分析性能瓶颈
2. 优化热点代码
3. 缓存优化

**Deliverables**:
- [ ] 性能分析报告
- [ ] 优化代码
- [ ] 基准对比

**验收标准**:
- 查询延迟 < 100ms (P95)
- 吞吐量 > 100 QPS

---

## 阶段 2: 记忆管理增强（4周）

### Sprint 2.1: 记忆分类扩展（2周）

#### Task 2.1.1: 扩展 MemoryCategory 枚举

**代码骨架**:
```rust
// cortex-mem-core/src/session/extraction.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl MemoryCategory {
    pub fn to_path(&self) -> &str {
        match self {
            Self::Profile => "user/profile.md",
            Self::Preference => "user/preferences",
            Self::Entity => "user/entities",
            Self::Event => "user/events",
            Self::Case => "agent/cases",
            Self::Pattern => "agent/patterns",
        }
    }
    
    pub fn should_merge(&self) -> bool {
        matches!(self, Self::Profile | Self::Preference)
    }
}
```

**Deliverables**:
- [ ] 枚举扩展
- [ ] 路径映射更新
- [ ] 文档更新

---

#### Task 2.1.2: 实现 Profile 提取

**代码骨架**:
```rust
impl MemoryExtractor {
    async fn extract_profile(
        &self,
        messages: &[Message],
    ) -> Result<Option<CandidateMemory>> {
        let prompt = format!(
            r#"从对话中提取用户画像信息。

【对话】
{}

【要求】
提取：
- 基本信息（职业、技术栈、兴趣）
- 工作习惯
- 偏好特点

返回 Markdown 格式的用户画像，如果没有信息则返回 null。"#,
            self.format_messages(messages)
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        
        if response.trim() == "null" {
            return Ok(None);
        }
        
        Ok(Some(CandidateMemory {
            category: MemoryCategory::Profile,
            abstract_text: self.extract_first_line(&response),
            overview: self.extract_summary(&response, 500),
            content: response,
        }))
    }
    
    /// 合并到现有 Profile
    async fn merge_profile(
        &self,
        existing: &str,
        new: &str,
    ) -> Result<String> {
        let prompt = format!(
            r#"合并两个用户画像，保留完整信息，去除重复。

【现有画像】
{}

【新增信息】
{}

返回合并后的 Markdown 格式画像。"#,
            existing, new
        );
        
        self.llm_client.generate(&prompt).await
    }
}
```

**Deliverables**:
- [ ] `extract_profile()` 实现
- [ ] `merge_profile()` 实现
- [ ] Prompt 模板
- [ ] 单元测试

---

#### Task 2.1.3: 实现 Pattern 提取

**代码骨架**:
```rust
impl MemoryExtractor {
    async fn extract_patterns(
        &self,
        messages: &[Message],
    ) -> Result<Vec<CandidateMemory>> {
        let prompt = format!(
            r#"从对话中提炼可复用的模式、流程和最佳实践。

【对话】
{}

【要求】
提炼：
- 通用的解决流程
- 可复用的方法论
- 最佳实践

返回 JSON 数组，每个模式包含：
- name: 模式名称
- applicability: 适用场景
- steps: 步骤列表
- examples: 示例

如果没有模式则返回空数组。"#,
            self.format_messages(messages)
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        let patterns: Vec<PatternData> = serde_json::from_str(&response)?;
        
        Ok(patterns.into_iter().map(|p| self.pattern_to_candidate(p)).collect())
    }
    
    fn pattern_to_candidate(&self, pattern: PatternData) -> CandidateMemory {
        let content = format!(
            "# 模式: {}\n\n## 适用场景\n{}\n\n## 步骤\n{}\n\n## 示例\n{}",
            pattern.name,
            pattern.applicability,
            pattern.steps.join("\n"),
            pattern.examples.join("\n\n")
        );
        
        CandidateMemory {
            category: MemoryCategory::Pattern,
            abstract_text: pattern.name.clone(),
            overview: pattern.applicability.clone(),
            content,
        }
    }
}
```

**Deliverables**:
- [ ] `extract_patterns()` 实现
- [ ] Prompt 模板
- [ ] 单元测试

---

### Sprint 2.2: 记忆去重优化（2周）

#### Task 2.2.1: 实现去重检测器

**代码骨架**:
```rust
// cortex-mem-core/src/session/deduplicator.rs (新文件)
pub struct MemoryDeduplicator {
    vector_store: Arc<dyn VectorStore>,
    embedding_client: Arc<dyn EmbeddingClient>,
    llm_client: Arc<dyn LLMClient>,
    config: DeduplicatorConfig,
}

pub struct DeduplicatorConfig {
    pub similarity_threshold: f32, // 默认 0.85
    pub enable_llm_check: bool,    // 默认 true
}

pub enum DeduplicationResult {
    NoDuplicate,
    Duplicate { existing_uri: String },
}

impl MemoryDeduplicator {
    pub async fn check_duplicate(
        &self,
        candidate: &CandidateMemory,
    ) -> Result<DeduplicationResult> {
        // 1. 向量相似度检索
        let vector = self.embedding_client.embed(&candidate.abstract_text).await?;
        
        let similar = self.vector_store.search(&vector, &SearchOptions {
            limit: 5,
            filters: vec![
                ("category", &candidate.category.to_string()),
            ],
            score_threshold: Some(self.config.similarity_threshold),
        }).await?;
        
        if similar.is_empty() {
            return Ok(DeduplicationResult::NoDuplicate);
        }
        
        // 2. LLM 精确判断
        if self.config.enable_llm_check {
            for existing in similar {
                let is_dup = self.is_duplicate_by_llm(candidate, &existing).await?;
                if is_dup {
                    return Ok(DeduplicationResult::Duplicate {
                        existing_uri: existing.uri,
                    });
                }
            }
        }
        
        Ok(DeduplicationResult::NoDuplicate)
    }
    
    async fn is_duplicate_by_llm(
        &self,
        candidate: &CandidateMemory,
        existing: &SearchResult,
    ) -> Result<bool> {
        // 读取现有记忆内容
        let existing_content = self.filesystem.read(&existing.uri).await?;
        
        let prompt = format!(
            r#"判断两个记忆是否重复（内容实质相同）。

【现有记忆】
{}

【新记忆】
{}

返回 JSON: {{"is_duplicate": true/false, "reason": "原因"}}"#,
            existing_content,
            candidate.content
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        let result: DuplicateCheckResult = serde_json::from_str(&response)?;
        
        Ok(result.is_duplicate)
    }
}
```

**Deliverables**:
- [ ] `MemoryDeduplicator` 实现
- [ ] 单元测试
- [ ] 集成测试

---

#### Task 2.2.2: 实现记忆合并

**代码骨架**:
```rust
impl MemoryDeduplicator {
    pub async fn merge_memory(
        &self,
        existing_uri: &str,
        new_content: &str,
        category: &MemoryCategory,
    ) -> Result<MergedMemory> {
        let existing_content = self.filesystem.read(existing_uri).await?;
        
        let prompt = format!(
            r#"合并两个记忆，保留完整信息，去除重复。

【现有记忆】
{}

【新增记忆】
{}

返回 JSON:
{{
  "abstract": "一句话摘要（< 200 字符）",
  "overview": "概览（< 2000 字符）",
  "content": "完整内容（Markdown 格式）"
}}"#,
            existing_content, new_content
        );
        
        let response = self.llm_client.generate(&prompt).await?;
        let merged: MergedMemory = serde_json::from_str(&response)?;
        
        // 更新文件
        self.filesystem.write(existing_uri, &merged.content).await?;
        self.filesystem.write(&format!("{}/.abstract", self.get_parent(existing_uri)), &merged.abstract_text).await?;
        self.filesystem.write(&format!("{}/.overview", self.get_parent(existing_uri)), &merged.overview).await?;
        
        Ok(merged)
    }
}
```

**Deliverables**:
- [ ] `merge_memory()` 实现
- [ ] 单元测试
- [ ] 集成测试

---

#### Task 2.2.3: 集成到提取流程

**代码骨架**:
```rust
impl MemoryExtractor {
    pub async fn extract_and_deduplicate(
        &self,
        messages: &[Message],
        session_id: &str,
    ) -> Result<ExtractionResult> {
        // 1. 提取候选记忆
        let candidates = self.extract(messages).await?;
        
        let mut created = vec![];
        let mut merged = vec![];
        let mut skipped = vec![];
        
        // 2. 去重检查
        for candidate in candidates {
            match self.deduplicator.check_duplicate(&candidate).await? {
                DeduplicationResult::NoDuplicate => {
                    // 创建新记忆
                    let uri = self.create_memory(&candidate, session_id).await?;
                    created.push(uri);
                }
                
                DeduplicationResult::Duplicate { existing_uri } => {
                    if candidate.category.should_merge() {
                        // 合并记忆
                        self.deduplicator.merge_memory(
                            &existing_uri,
                            &candidate.content,
                            &candidate.category,
                        ).await?;
                        merged.push(existing_uri);
                    } else {
                        // 独立保存（Event/Case/Pattern）
                        let uri = self.create_memory(&candidate, session_id).await?;
                        created.push(uri);
                    }
                }
            }
        }
        
        Ok(ExtractionResult {
            created,
            merged,
            skipped,
        })
    }
}
```

**Deliverables**:
- [ ] `extract_and_deduplicate()` 实现
- [ ] 集成测试
- [ ] 文档更新

---

## 阶段 3: 可观测性增强（可选，2周）

### Task 3.1: 轻量级检索轨迹

**代码骨架**:
```rust
// cortex-mem-core/src/search/trace.rs (新文件)
pub struct SearchTrace {
    pub query: String,
    pub steps: Vec<String>,
    pub final_count: usize,
    pub duration_ms: u64,
}

impl SearchTrace {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            steps: vec![],
            final_count: 0,
            duration_ms: 0,
        }
    }
    
    pub fn add_step(&mut self, description: String) {
        self.steps.push(description);
    }
    
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

impl HierarchicalRetriever {
    pub async fn retrieve_with_trace(
        &self,
        query: &TypedQuery,
    ) -> Result<(HierarchicalResult, SearchTrace)> {
        let mut trace = SearchTrace::new(&query.query);
        let start = Instant::now();
        
        trace.add_step(format!("全局搜索: 定位高分目录"));
        let top_dirs = self.global_search(query, self.config.global_search_topk).await?;
        trace.add_step(format!("找到 {} 个高分目录", top_dirs.len()));
        
        trace.add_step(format!("递归搜索: 探索子目录（最大深度 {}）", self.config.max_depth));
        let candidates = self.recursive_search_all(&top_dirs, query).await?;
        trace.add_step(format!("收集到 {} 个候选", candidates.len()));
        
        trace.add_step(format!("分数传播与排序"));
        let results = self.apply_score_propagation_and_sort(candidates, query.limit);
        trace.add_step(format!("最终返回 {} 个结果", results.len()));
        
        trace.final_count = results.len();
        trace.duration_ms = start.elapsed().as_millis() as u64;
        
        Ok((HierarchicalResult { results, trace: None }, trace))
    }
}
```

**Deliverables**:
- [ ] `SearchTrace` 实现
- [ ] `retrieve_with_trace()` 实现
- [ ] 可选开关配置
- [ ] JSON 导出

**验收标准**:
- 性能影响 < 5ms
- 可选开关生效
- JSON 格式正确

---

## 总结

### 关键里程碑

| 里程碑 | 时间点 | 验收标准 |
|--------|--------|----------|
| M0 | 第 2 周 | 三层文件 100%<br/>.abstract < 2K<br/>查询 < 80ms |
| M1 | 第 8 周 | Recall@1 > 95%<br/>递归检索生效 |
| M2 | 第 12 周 | 六分类支持<br/>去重准确率 > 90% |
| M3 | 第 14 周 | 3.0 正式发布 |

### 风险管理

1. **技术风险**: 充分测试，灰度发布
2. **性能风险**: 持续基准测试，性能监控
3. **兼容性风险**: 数据迁移脚本，文档指南

**准备就绪，开始实施！🚀**
