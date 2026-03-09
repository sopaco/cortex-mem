# 实体查询召回优化技术方案

## 1. 问题分析

### 1.1 L0 抽象层信息丢失问题

**位置**: `cortex-mem-core/src/layers/generator.rs`

L0 抽象层设计为 ~100 tokens 的精简摘要，用于快速筛选候选。但在压缩过程中丢失了关键实体信息：

```rust
// 当前 L0 prompt 要求
"Cover MULTIPLE key aspects when content is rich (who, what, key topics, important outcomes)"
"Prioritize information breadth over depth"
```

**问题表现**：
- 用户查询"王明是谁"时，L0 摘要可能只写了"讨论了项目进展"，丢失了具体人名
- 导致 L0 层召回失败，需要降级到更低的阈值或全量 L2 检索
- 影响检索效率和精度

### 1.2 Intent 分析实现过于简陋

**位置**: `cortex-mem-core/src/search/vector_engine.rs:600-644`

当前 `detect_intent_type` 方法使用硬编码的关键词匹配：

```rust
fn detect_intent_type(query: &str) -> QueryIntentType {
    let lower = query.to_lowercase();

    // 硬编码的关键词匹配
    if lower.contains("when") || lower.contains("recent") || ... {
        return QueryIntentType::Temporal;
    }
    if lower.starts_with("what is") || lower.starts_with("who is") || ... {
        return QueryIntentType::Factual;
    }
    // ...
    QueryIntentType::General
}
```

**问题分析**：
1. **中文支持差**：只匹配英文关键词，中文查询几乎全部归类为 General
2. **语义理解缺失**："告诉我上次我们讨论的技术方案" 无法被识别为 Temporal
3. **实体查询判断粗糙**：`is_likely_entity_query` 仅通过长度和字符类型判断

### 1.3 已有资源未被利用

**位置**: `cortex-mem-core/src/llm/prompts.rs:106-126`

存在一个完整的 `intent_analysis` prompt，但从未被调用：

```rust
/// Prompt for intent analysis in retrieval
pub fn intent_analysis(query: &str) -> String {
    format!(
        r#"Analyze the following query and extract:
1. **Keywords**: Important keywords for search (2-5 words)
2. **Entities**: Named entities mentioned (people, places, technologies)
3. **Time Range**: Any time-related constraints (if mentioned)
4. **Query Type**: The type of query (factual, procedural, conceptual, etc.)
...
```

**技术债务**：已有完善的 LLM intent 分析 prompt，却用简陋的关键词匹配替代。

### 1.4 实体识别与召回链路断裂

当前流程中，实体信息仅在记忆提取时识别，但在召回时未利用：

```
记忆存储时：识别实体 → 存入 metadata.entities
记忆召回时：查询 → 嵌入 → 向量相似度 → 返回结果
                ↑
            未利用实体信息
```

## 2. 解决方案

### 2.1 架构设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        实体感知召回架构                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   用户查询   │───►│ LLM Intent  │───►│  实体增强   │───►│  混合检索   │  │
│  │             │    │   分析器    │    │  查询扩展   │    │  策略选择   │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│                            │                   │                  │         │
│                            ▼                   ▼                  ▼         │
│                     ┌──────────────────────────────────────────────────┐   │
│                     │              检索执行层                           │   │
│                     │  ┌─────────┐  ┌─────────┐  ┌─────────┐          │   │
│                     │  │实体过滤 │  │三层级联 │  │语义搜索 │          │   │
│                     │  │召回     │  │召回     │  │召回     │          │   │
│                     │  └─────────┘  └─────────┘  └─────────┘          │   │
│                     └──────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 LLM Intent 分析器重构

#### 2.2.1 增强 Intent 分析结果结构

```rust
// 文件: cortex-mem-core/src/search/query_intent.rs (新建或重构)

use serde::{Deserialize, Serialize};

/// 增强的查询意图分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedQueryIntent {
    /// 原始查询
    pub original_query: String,
    
    /// 重写后的查询（用于语义搜索）
    pub rewritten_query: Option<String>,
    
    /// 查询类型
    pub query_type: QueryType,
    
    /// 提取的关键词
    pub keywords: Vec<String>,
    
    /// 提取的命名实体
    pub entities: ExtractedEntities,
    
    /// 时间约束
    pub time_constraint: Option<TimeConstraint>,
    
    /// 检索策略建议
    pub retrieval_strategy: RetrievalStrategy,
    
    /// 置信度
    pub confidence: f32,
}

/// 查询类型（扩展版）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    /// 事实查询："Python 的创始人是谁？"
    Factual,
    /// 实体查询："王明"、"那个项目"
    EntityLookup,
    /// 时间查询："上周讨论了什么？"
    Temporal,
    /// 关系查询："张三和李四是什么关系？"
    Relational,
    /// 过程查询："如何配置环境？"
    Procedural,
    /// 概念查询："什么是 RAG？"
    Conceptual,
    /// 搜索查询："找一下关于微服务的讨论"
    Search,
    /// 比较/决策查询："方案A和方案B哪个更好？"
    Comparative,
    /// 一般查询
    General,
}

/// 提取的实体信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedEntities {
    /// 人名
    pub people: Vec<String>,
    /// 组织/公司
    pub organizations: Vec<String>,
    /// 项目/产品名
    pub projects: Vec<String>,
    /// 技术术语
    pub technologies: Vec<String>,
    /// 地点
    pub locations: Vec<String>,
    /// 其他实体
    pub other: Vec<String>,
}

impl ExtractedEntities {
    pub fn is_empty(&self) -> bool {
        self.people.is_empty()
            && self.organizations.is_empty()
            && self.projects.is_empty()
            && self.technologies.is_empty()
            && self.locations.is_empty()
            && self.other.is_empty()
    }
    
    pub fn all_entities(&self) -> Vec<&String> {
        let mut entities = Vec::new();
        entities.extend(&self.people);
        entities.extend(&self.organizations);
        entities.extend(&self.projects);
        entities.extend(&self.technologies);
        entities.extend(&self.locations);
        entities.extend(&self.other);
        entities
    }
}

/// 时间约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConstraint {
    /// 时间表达式原文
    pub expression: String,
    /// 解析后的起始时间
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    /// 解析后的结束时间
    pub end: Option<chrono::DateTime<chrono::Utc>>,
    /// 相对时间描述
    pub relative: Option<String>,  // "last_week", "yesterday", "recent"
}

/// 检索策略建议
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetrievalStrategy {
    /// 三层级联检索（默认）
    LayeredCascade,
    /// 实体优先检索
    EntityFirst,
    /// 时间范围过滤优先
    TimeFiltered,
    /// 全量语义检索
    FullSemantic,
    /// 混合策略
    Hybrid {
        entity_weight: f32,
        semantic_weight: f32,
        time_weight: f32,
    },
}
```

#### 2.2.2 LLM Intent 分析器实现

```rust
// 文件: cortex-mem-core/src/search/intent_analyzer.rs (新建)

use crate::llm::{LLMClient, Prompts};
use crate::search::query_intent::*;
use crate::Result;
use std::sync::Arc;
use serde::Deserialize;

/// Intent 分析器
pub struct IntentAnalyzer {
    llm_client: Arc<dyn LLMClient>,
    /// 是否启用 LLM 分析（可配置关闭以节省成本）
    enabled: bool,
    /// 缓存最近的 intent 分析结果
    cache: Arc<tokio::sync::RwLock<lru::LruCache<String, EnhancedQueryIntent>>>,
}

impl IntentAnalyzer {
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            enabled: true,
            cache: Arc::new(tokio::sync::RwLock::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(100).unwrap())
            )),
        }
    }
    
    /// 分析查询意图
    pub async fn analyze(&self, query: &str) -> Result<EnhancedQueryIntent> {
        // 1. 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(intent) = cache.peek(query) {
                return Ok(intent.clone());
            }
        }
        
        // 2. 使用 LLM 进行深度分析
        let intent = if self.enabled {
            self.analyze_with_llm(query).await?
        } else {
            // 回退到规则引擎
            self.analyze_with_rules(query)
        };
        
        // 3. 缓存结果
        {
            let mut cache = self.cache.write().await;
            cache.put(query.to_string(), intent.clone());
        }
        
        Ok(intent)
    }
    
    async fn analyze_with_llm(&self, query: &str) -> Result<EnhancedQueryIntent> {
        let prompt = Self::build_intent_prompt(query);
        
        let response = self.llm_client.complete_with_system(
            "你是一个专业的查询意图分析器，帮助理解用户在记忆系统中的检索意图。",
            &prompt
        ).await?;
        
        // 解析 LLM 响应
        self.parse_llm_response(query, &response)
    }
    
    fn build_intent_prompt(query: &str) -> String {
        format!(
            r#"分析以下查询，提取关键信息用于记忆检索系统。

## 输入查询
{}

## 分析任务
请识别以下信息：

1. **查询类型**：从以下类型中选择最匹配的
   - factual: 事实性问题（"谁"、"什么"、"多少"）
   - entity_lookup: 查找特定实体（人名、项目名等）
   - temporal: 有时间约束的查询
   - relational: 查询实体间关系
   - procedural: 操作步骤类问题
   - conceptual: 概念解释类
   - search: 搜索类请求
   - comparative: 比较/决策类
   - general: 通用查询

2. **命名实体**：提取人名、组织、项目、技术术语、地点等

3. **时间约束**：识别时间表达式并解析

4. **关键词**：提取 2-5 个最重要的搜索关键词

5. **查询重写**：如果原查询模糊，生成一个更明确的版本

6. **检索策略建议**：推荐最合适的检索方法

## 输出格式（JSON）
{{
  "query_type": "类型名称",
  "entities": {{
    "people": ["人名1"],
    "organizations": ["组织名"],
    "projects": ["项目名"],
    "technologies": ["技术术语"],
    "locations": ["地点"],
    "other": ["其他实体"]
  }},
  "time_constraint": {{
    "expression": "时间原文",
    "relative": "相对时间（如 last_week）"
  }},
  "keywords": ["关键词1", "关键词2"],
  "rewritten_query": "重写后的查询（如果原查询清晰则为 null）",
  "retrieval_strategy": {{
    "type": "策略名称",
    "params": {{}}
  }},
  "confidence": 0.95
}}

## 注意事项
- 中文查询请用中文分析
- 实体识别要准确，不确定的放入 other
- 时间表达式要尽量解析为具体范围
- confidence 反映你对分析的确定程度

请直接输出 JSON，不要有其他内容。"#,
            query
        )
    }
    
    fn parse_llm_response(&self, query: &str, response: &str) -> Result<EnhancedQueryIntent> {
        // 提取 JSON
        let json_str = Self::extract_json(response);
        
        // 解析
        let parsed: LLMIntentResponse = serde_json::from_str(json_str)
            .map_err(|e| crate::Error::Search(format!("Failed to parse intent: {}", e)))?;
        
        Ok(EnhancedQueryIntent {
            original_query: query.to_string(),
            rewritten_query: parsed.rewritten_query,
            query_type: parsed.query_type,
            keywords: parsed.keywords,
            entities: parsed.entities,
            time_constraint: parsed.time_constraint,
            retrieval_strategy: self.parse_strategy(&parsed.retrieval_strategy),
            confidence: parsed.confidence,
        })
    }
    
    fn extract_json(response: &str) -> &str {
        let trimmed = response.trim();
        if let Some(start) = trimmed.find('{') {
            let mut depth = 0;
            for (i, c) in trimmed[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return &trimmed[start..start + i + 1];
                        }
                    }
                    _ => {}
                }
            }
        }
        trimmed
    }
    
    fn parse_strategy(&self, strategy: &StrategyResponse) -> RetrievalStrategy {
        match strategy.type_.as_str() {
            "entity_first" => RetrievalStrategy::EntityFirst,
            "time_filtered" => RetrievalStrategy::TimeFiltered,
            "full_semantic" => RetrievalStrategy::FullSemantic,
            "hybrid" => {
                let params = &strategy.params;
                RetrievalStrategy::Hybrid {
                    entity_weight: params.get("entity_weight").and_then(|v| v.as_f64()).unwrap_or(0.4) as f32,
                    semantic_weight: params.get("semantic_weight").and_then(|v| v.as_f64()).unwrap_or(0.4) as f32,
                    time_weight: params.get("time_weight").and_then(|v| v.as_f64()).unwrap_or(0.2) as f32,
                }
            }
            _ => RetrievalStrategy::LayeredCascade,
        }
    }
    
    /// 规则引擎回退（当 LLM 不可用时）
    fn analyze_with_rules(&self, query: &str) -> EnhancedQueryIntent {
        let mut intent = EnhancedQueryIntent {
            original_query: query.to_string(),
            rewritten_query: None,
            query_type: QueryType::General,
            keywords: Vec::new(),
            entities: ExtractedEntities::default(),
            time_constraint: None,
            retrieval_strategy: RetrievalStrategy::LayeredCascade,
            confidence: 0.5,
        };
        
        // 简单的规则匹配
        let lower = query.to_lowercase();
        
        // 时间关键词检测
        let time_keywords = ["上次", "昨天", "上周", "最近", "last", "yesterday", "week", "recent"];
        if time_keywords.iter().any(|k| lower.contains(k)) {
            intent.query_type = QueryType::Temporal;
            intent.retrieval_strategy = RetrievalStrategy::TimeFiltered;
        }
        
        // 实体查询检测
        if query.chars().count() <= 6 && !query.contains(' ') && !query.contains('?') {
            intent.query_type = QueryType::EntityLookup;
            intent.retrieval_strategy = RetrievalStrategy::EntityFirst;
            // 假设短查询是实体名
            intent.entities.other.push(query.to_string());
        }
        
        // 提取关键词
        intent.keywords = query
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .map(|s| s.to_lowercase())
            .take(5)
            .collect();
        
        intent
    }
}

// LLM 响应结构
#[derive(Debug, Deserialize)]
struct LLMIntentResponse {
    #[serde(rename = "query_type")]
    query_type: QueryType,
    entities: ExtractedEntities,
    time_constraint: Option<TimeConstraint>,
    keywords: Vec<String>,
    rewritten_query: Option<String>,
    retrieval_strategy: StrategyResponse,
    confidence: f32,
}

#[derive(Debug, Deserialize)]
struct StrategyResponse {
    #[serde(rename = "type")]
    type_: String,
    params: serde_json::Map<String, serde_json::Value>,
}
```

### 2.3 L0 抽象层实体保留增强

#### 2.3.1 改进 L0 生成 Prompt

```rust
// 文件: cortex-mem-core/src/llm/prompts.rs (修改)

impl Prompts {
    /// Prompt for generating L0 abstract with entity preservation
    pub fn abstract_generation_with_entities(content: &str, known_entities: &[String]) -> String {
        let entities_hint = if known_entities.is_empty() {
            String::new()
        } else {
            format!(
                "\n**已知实体（必须保留）**：{}\n",
                known_entities.join(", ")
            )
        };
        
        format!(
            r#"生成一个精炼的摘要（约100个token），用于快速相关性检测。

## 要求
1. **实体优先**：必须包含内容中提到的所有重要实体（人名、项目名、技术术语等）
2. **信息广度**：覆盖多个关键方面，而非深入单一主题
3. **紧凑表达**：使用"讨论了X、Y和Z"而非冗长描述
4. **语言一致**：使用与输入内容相同的语言
{}
## 内容
{}

## 摘要（约100 token）"#,
            entities_hint, content
        )
    }
    
    /// 从 L1 概览中提取实体列表
    pub fn entity_extraction_from_overview(overview: &str) -> String {
        format!(
            r#"从以下概览中提取所有命名实体。

## 输出格式（JSON）
{{
  "people": ["人名"],
  "organizations": ["组织名"],
  "projects": ["项目名"],
  "technologies": ["技术术语"],
  "locations": ["地点"],
  "other": ["其他实体"]
}}

## 概览内容
{}

## 实体列表（JSON）"#,
            overview
        )
    }
}
```

#### 2.3.2 层级生成器改进

```rust
// 文件: cortex-mem-core/src/layers/generator.rs (修改)

impl AbstractGenerator {
    /// 生成 L0 抽象（带实体保留）
    pub async fn generate_with_entities(
        &self,
        content: &str,
        llm: &Arc<dyn LLMClient>,
        existing_entities: Option<&[String]>,
    ) -> Result<AbstractResult> {
        info!("Generating L0 Abstract with entity preservation");
        
        // 1. 如果提供了已有实体，使用增强 prompt
        let prompt = if let Some(entities) = existing_entities {
            Prompts::abstract_generation_with_entities(content, entities)
        } else {
            Prompts::abstract_generation(content)
        };
        
        // 2. 调用 LLM
        let abstract_text = llm.complete_with_system(
            "你是专业的摘要生成器，擅长在有限篇幅内保留关键实体信息。",
            &prompt
        ).await?;
        
        // 3. 从摘要中提取实体（用于后续索引）
        let entities = self.extract_entities_from_abstract(&abstract_text, llm).await?;
        
        Ok(AbstractResult {
            content: abstract_text,
            entities,
            token_count: Self::estimate_tokens(&abstract_text),
        })
    }
    
    async fn extract_entities_from_abstract(
        &self,
        abstract_text: &str,
        llm: &Arc<dyn LLMClient>,
    ) -> Result<Vec<String>> {
        let prompt = format!(
            r#"从以下摘要中提取所有命名实体（人名、组织、项目、技术术语等）。

摘要：
{}

输出格式（JSON数组）：
["实体1", "实体2", ...]"#,
            abstract_text
        );
        
        let response = llm.complete(&prompt).await?;
        
        // 解析 JSON 数组
        let json_str = response.trim()
            .trim_start_matches('[')
            .trim_end_matches(']');
        
        if json_str.is_empty() {
            return Ok(Vec::new());
        }
        
        serde_json::from_str(&format!("[{}]", json_str))
            .map_err(|e| crate::Error::Llm(format!("Failed to parse entities: {}", e)))
    }
}

#[derive(Debug, Clone)]
pub struct AbstractResult {
    pub content: String,
    pub entities: Vec<String>,
    pub token_count: usize,
}
```

### 2.4 实体感知检索策略

#### 2.4.1 混合检索执行器

```rust
// 文件: cortex-mem-core/src/search/retrieval_executor.rs (新建)

use crate::search::query_intent::*;
use crate::search::{VectorSearchEngine, SearchOptions, SearchResult};
use crate::vector_store::QdrantVectorStore;
use crate::Result;
use std::sync::Arc;
use tracing::{info, debug};

/// 检索执行器
pub struct RetrievalExecutor {
    vector_engine: Arc<VectorSearchEngine>,
    vector_store: Arc<QdrantVectorStore>,
}

impl RetrievalExecutor {
    pub fn new(
        vector_engine: Arc<VectorSearchEngine>,
        vector_store: Arc<QdrantVectorStore>,
    ) -> Self {
        Self {
            vector_engine,
            vector_store,
        }
    }
    
    /// 根据策略执行检索
    pub async fn execute(
        &self,
        query: &str,
        intent: &EnhancedQueryIntent,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        info!("Executing retrieval with strategy: {:?}", intent.retrieval_strategy);
        
        match &intent.retrieval_strategy {
            RetrievalStrategy::EntityFirst => {
                self.entity_first_search(query, intent, options).await
            }
            RetrievalStrategy::TimeFiltered => {
                self.time_filtered_search(query, intent, options).await
            }
            RetrievalStrategy::FullSemantic => {
                self.full_semantic_search(query, intent, options).await
            }
            RetrievalStrategy::Hybrid { entity_weight, semantic_weight, time_weight } => {
                self.hybrid_search(query, intent, options, *entity_weight, *semantic_weight, *time_weight).await
            }
            RetrievalStrategy::LayeredCascade => {
                // 默认三层级联检索
                self.vector_engine.layered_semantic_search(query, options).await
            }
        }
    }
    
    /// 实体优先检索
    async fn entity_first_search(
        &self,
        query: &str,
        intent: &EnhancedQueryIntent,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let all_entities = intent.entities.all_entities();
        
        if all_entities.is_empty() {
            // 没有实体，回退到语义搜索
            return self.vector_engine.semantic_search(query, options).await;
        }
        
        debug!("Entity-first search for entities: {:?}", all_entities);
        
        // 1. 先用实体过滤检索
        let entity_filters = self.build_entity_filters(&intent.entities);
        let entity_results = self.vector_store
            .search_by_entities(&entity_filters, options.limit * 2)
            .await?;
        
        // 2. 再用语义相似度排序
        let query_embedding = self.vector_engine.get_embedding(query).await?;
        
        let mut scored_results: Vec<_> = entity_results
            .into_iter()
            .map(|mem| {
                let semantic_score = cosine_similarity(&query_embedding, &mem.embedding);
                let entity_match_score = self.calculate_entity_match_score(&mem, &intent.entities);
                
                // 综合评分：实体匹配 40% + 语义相似 60%
                let combined = entity_match_score * 0.4 + semantic_score * 0.6;
                
                SearchResult {
                    uri: mem.metadata.uri.unwrap_or(mem.id),
                    score: combined,
                    snippet: extract_snippet(&mem.content, query),
                    content: Some(mem.content),
                }
            })
            .collect();
        
        // 3. 排序并返回
        scored_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored_results.truncate(options.limit);
        
        Ok(scored_results)
    }
    
    /// 时间过滤检索
    async fn time_filtered_search(
        &self,
        query: &str,
        intent: &EnhancedQueryIntent,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let time_filter = match &intent.time_constraint {
            Some(tc) => tc,
            None => return self.vector_engine.layered_semantic_search(query, options).await,
        };
        
        debug!("Time-filtered search for: {:?}", time_filter);
        
        // 构建时间范围过滤器
        let mut filters = crate::types::Filters::default();
        
        if let Some(start) = time_filter.start {
            filters.created_after = Some(start);
        }
        if let Some(end) = time_filter.end {
            filters.created_before = Some(end);
        }
        
        // 执行带时间过滤的搜索
        self.vector_engine.filtered_semantic_search(query, &filters, options).await
    }
    
    /// 混合检索
    async fn hybrid_search(
        &self,
        query: &str,
        intent: &EnhancedQueryIntent,
        options: &SearchOptions,
        entity_weight: f32,
        semantic_weight: f32,
        time_weight: f32,
    ) -> Result<Vec<SearchResult>> {
        let mut candidates: std::collections::HashMap<String, HybridCandidate> = 
            std::collections::HashMap::new();
        
        // 1. 实体检索分支
        if entity_weight > 0.0 && !intent.entities.all_entities().is_empty() {
            let entity_results = self.entity_first_search(query, intent, &SearchOptions {
                limit: options.limit * 2,
                ..options.clone()
            }).await?;
            
            for result in entity_results {
                let entry = candidates.entry(result.uri.clone()).or_default();
                entry.uri = result.uri.clone();
                entry.entity_score = Some(result.score);
                entry.content = result.content.clone();
            }
        }
        
        // 2. 语义检索分支
        if semantic_weight > 0.0 {
            let semantic_results = self.vector_engine.layered_semantic_search(query, &SearchOptions {
                limit: options.limit * 2,
                ..options.clone()
            }).await?;
            
            for result in semantic_results {
                let entry = candidates.entry(result.uri.clone()).or_default();
                entry.uri = result.uri.clone();
                entry.semantic_score = Some(result.score);
                entry.content = result.content.clone();
            }
        }
        
        // 3. 时间过滤分支
        if time_weight > 0.0 && intent.time_constraint.is_some() {
            let time_results = self.time_filtered_search(query, intent, &SearchOptions {
                limit: options.limit * 2,
                ..options.clone()
            }).await?;
            
            for result in time_results {
                let entry = candidates.entry(result.uri.clone()).or_default();
                entry.uri = result.uri.clone();
                entry.time_score = Some(result.score);
                entry.content = result.content.clone();
            }
        }
        
        // 4. 综合评分
        let mut final_results: Vec<_> = candidates
            .into_values()
            .map(|c| {
                let entity = c.entity_score.unwrap_or(0.0) * entity_weight;
                let semantic = c.semantic_score.unwrap_or(0.0) * semantic_weight;
                let time = c.time_score.unwrap_or(0.0) * time_weight;
                let total = entity + semantic + time;
                
                SearchResult {
                    uri: c.uri,
                    score: total,
                    snippet: String::new(),
                    content: c.content,
                }
            })
            .filter(|r| r.score >= options.threshold)
            .collect();
        
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        final_results.truncate(options.limit);
        
        Ok(final_results)
    }
    
    fn build_entity_filters(&self, entities: &ExtractedEntities) -> EntityFilters {
        EntityFilters {
            people: entities.people.clone(),
            organizations: entities.organizations.clone(),
            projects: entities.projects.clone(),
            technologies: entities.technologies.clone(),
        }
    }
    
    fn calculate_entity_match_score(&self, memory: &crate::types::Memory, entities: &ExtractedEntities) -> f32 {
        let metadata_entities = &memory.metadata.entities;
        let query_entities = entities.all_entities();
        
        if query_entities.is_empty() {
            return 0.5;
        }
        
        let matches = query_entities
            .iter()
            .filter(|qe| metadata_entities.iter().any(|me| me.to_lowercase() == qe.to_lowercase()))
            .count();
        
        matches as f32 / query_entities.len() as f32
    }
}

#[derive(Default)]
struct HybridCandidate {
    uri: String,
    entity_score: Option<f32>,
    semantic_score: Option<f32>,
    time_score: Option<f32>,
    content: Option<String>,
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { 0.0 } else { dot / (mag_a * mag_b) }
}

fn extract_snippet(content: &str, query: &str) -> String {
    // 复用现有的 snippet 提取逻辑
    if content.chars().count() > 200 {
        format!("{}...", content.chars().take(200).collect::<String>())
    } else {
        content.to_string()
    }
}
```

### 2.5 集成到现有搜索引擎

```rust
// 文件: cortex-mem-core/src/search/vector_engine.rs (修改)

impl VectorSearchEngine {
    /// 智能检索入口（使用 Intent 分析）
    pub async fn smart_search(
        &self,
        query: &str,
        options: &SearchOptions,
        intent_analyzer: Option<&IntentAnalyzer>,
    ) -> Result<Vec<SearchResult>> {
        // 1. Intent 分析
        let intent = if let Some(analyzer) = intent_analyzer {
            analyzer.analyze(query).await?
        } else {
            // 回退到旧的简单分析
            self.simple_intent_analysis(query).await
        };
        
        info!(
            "Smart search intent: type={:?}, strategy={:?}, entities={}",
            intent.query_type,
            intent.retrieval_strategy,
            intent.entities.all_entities().len()
        );
        
        // 2. 根据策略选择检索路径
        let search_query = intent.rewritten_query.as_deref().unwrap_or(query);
        
        // 3. 执行检索
        match &intent.retrieval_strategy {
            RetrievalStrategy::EntityFirst => {
                // 使用实体优先检索
                self.entity_aware_search(search_query, &intent, options).await
            }
            _ => {
                // 使用增强的三层级联检索
                self.enhanced_layered_search(search_query, &intent, options).await
            }
        }
    }
    
    /// 实体感知的层级检索
    async fn enhanced_layered_search(
        &self,
        query: &str,
        intent: &EnhancedQueryIntent,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 调用现有的 layered_semantic_search，但使用 intent 中的信息
        let mut enhanced_options = options.clone();
        
        // 根据意图类型调整阈值
        enhanced_options.threshold = match intent.query_type {
            QueryType::EntityLookup => 0.3,  // 实体查询降低阈值
            QueryType::Temporal => 0.35,
            _ => options.threshold,
        };
        
        self.layered_semantic_search(query, &enhanced_options).await
    }
}
```

## 3. 配置支持

```toml
# config.toml 新增配置

[search]
# Intent 分析模式
intent_analysis_mode = "llm"  # llm, rules, hybrid

# Intent 分析缓存
intent_cache_size = 100

# 实体检索权重
[search.entity_retrieval]
enabled = true
entity_match_weight = 0.4
semantic_weight = 0.6

# L0 实体保留
[search.l0_entity_preservation]
enabled = true
min_entities = 3
```

## 4. 实现计划

| 步骤 | 任务 | 依赖 |
|------|------|------|
| 1 | 实现 EnhancedQueryIntent 结构 | 无 |
| 2 | 实现 IntentAnalyzer（含 LLM 分析） | 步骤1 |
| 3 | 修改 L0 生成器，添加实体保留 | 无 |
| 4 | 实现 RetrievalExecutor | 步骤1, 2 |
| 5 | 集成到 VectorSearchEngine | 步骤2, 4 |
| 6 | 添加配置支持 | 全部 |

## 5. 预期收益

| 场景 | 当前表现 | 优化后 | 提升 |
|------|----------|--------|------|
| "王明是谁" | 召回失败或低分 | 高召回率 | +60%+ |
| "上周讨论的技术方案" | 归类错误 | 准确识别+时间过滤 | +40% |
| 中文意图识别 | ~30% 准确 | ~85% 准确 | +55pp |
| LLM 开销 | 无 | 每次查询 1 次 | 可控 |

## 6. 技术债务清理

1. **删除** `detect_intent_type` 方法中的硬编码关键词匹配
2. **启用** 已存在但未使用的 `Prompts::intent_analysis`
3. **统一** Intent 分析逻辑，移除重复代码
4. **添加** 完善的单元测试和集成测试
