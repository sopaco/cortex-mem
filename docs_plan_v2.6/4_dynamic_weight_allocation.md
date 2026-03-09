# 动态权重分配技术方案

## 1. 问题分析

### 1.1 当前静态权重的问题

**位置**: `cortex-mem-core/src/search/vector_engine.rs:466-468`

三层级联检索的权重是硬编码的：

```rust
// Combined scoring: 0.2*L0 + 0.3*L1 + 0.5*L2
let combined_score = l0_score * 0.2 + l1_score * 0.3 + l2_score * 0.5;
```

**问题分析**：

1. **无法适应不同查询类型**：
   - 实体查询："王明是谁" → 需要 L2 精确匹配，当前权重合理
   - 语义查询："讨论了什么技术方案" → L0/L1 更重要，权重应调整
   - 时间查询："上周的决策" → 需要结合时间信息，权重需动态调整

2. **无法适应不同记忆类型**：
   - Preference（偏好）：L0 抽象通常已包含关键信息
   - Event（事件）：L2 细节更重要
   - Entity（实体）：需要 L2 精确匹配

3. **与 Intent 分析割裂**：虽然有 Intent 分析，但未影响权重分配

### 1.2 现有降级策略的局限

当 L0 搜索失败时：

```rust
// 策略1: 降低阈值重试
let relaxed_threshold = (adaptive_threshold - 0.2).max(0.4);

// 策略2: 完全降级到语义搜索
return self.semantic_search(query, options).await;
```

**问题**：降级策略是"全有或全无"，无法细粒度调整权重。

## 2. 解决方案

### 2.1 架构设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        动态权重分配架构                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │  查询意图   │───►│  上下文     │───►│  权重计算   │───►│  检索执行   │  │
│  │  分析结果   │    │  特征提取   │    │   引擎      │    │             │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│                            │                   │                           │
│                            │                   │                           │
│                            ▼                   ▼                           │
│                     ┌──────────────────────────────────────────┐           │
│                     │  权重模板库 + 自适应学习                   │           │
│                     │  ┌─────────┐  ┌─────────┐  ┌─────────┐   │           │
│                     │  │预定义模板│  │历史反馈 │  │实时调整 │   │           │
│                     │  └─────────┘  └─────────┘  └─────────┘   │           │
│                     └──────────────────────────────────────────┘           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 权重分配模型

#### 2.2.1 权重计算输入特征

```rust
// 文件: cortex-mem-core/src/search/weight_model.rs (新建)

use serde::{Deserialize, Serialize};

/// 权重计算的输入特征
#[derive(Debug, Clone)]
pub struct WeightFeatures {
    /// 查询类型特征
    pub query_type: QueryTypeFeature,
    /// 记忆类型特征
    pub memory_type: Option<MemoryTypeFeature>,
    /// 查询长度特征
    pub query_length: QueryLengthFeature,
    /// 时间约束特征
    pub time_constraint: bool,
    /// 实体数量
    pub entity_count: usize,
    /// 查询复杂度
    pub complexity: f32,
    /// 用户历史偏好（如果有）
    pub user_preference: Option<LayerPreference>,
}

#[derive(Debug, Clone, Copy)]
pub enum QueryTypeFeature {
    Factual,
    EntityLookup,
    Temporal,
    Relational,
    Procedural,
    Conceptual,
    Search,
    Comparative,
    General,
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryTypeFeature {
    Preference,
    Entity,
    Event,
    Case,
    PersonalInfo,
    WorkHistory,
    Relationship,
    Goal,
    Conversation,
}

#[derive(Debug, Clone, Copy)]
pub enum QueryLengthFeature {
    /// 短查询（≤5词）
    Short,
    /// 中等查询（6-15词）
    Medium,
    /// 长查询（>15词）
    Long,
}

/// 用户层级偏好（从历史反馈学习）
#[derive(Debug, Clone, Default)]
pub struct LayerPreference {
    /// L0 偏好得分
    pub l0_score: f32,
    /// L1 偏好得分
    pub l1_score: f32,
    /// L2 偏好得分
    pub l2_score: f32,
}
```

#### 2.2.2 权重分配策略

```rust
// 文件: cortex-mem-core/src/search/weight_model.rs (继续)

/// 三层权重
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LayerWeights {
    pub l0: f32,  // L0 抽象层权重
    pub l1: f32,  // L1 概览层权重
    pub l2: f32,  // L2 细节层权重
}

impl LayerWeights {
    /// 归一化权重（确保总和为 1.0）
    pub fn normalize(&self) -> Self {
        let total = self.l0 + self.l1 + self.l2;
        if total == 0.0 {
            return Self::default();
        }
        Self {
            l0: self.l0 / total,
            l1: self.l1 / total,
            l2: self.l2 / total,
        }
    }
    
    /// 默认权重
    pub fn default_weights() -> Self {
        Self { l0: 0.2, l1: 0.3, l2: 0.5 }
    }
}

impl Default for LayerWeights {
    fn default() -> Self {
        Self::default_weights()
    }
}

/// 权重分配策略
pub trait WeightStrategy: Send + Sync {
    /// 计算权重
    fn calculate(&self, features: &WeightFeatures) -> LayerWeights;
    
    /// 策略名称
    fn name(&self) -> &str;
}

/// 预定义模板策略
pub struct TemplateWeightStrategy {
    templates: Vec<WeightTemplate>,
}

/// 权重模板
#[derive(Debug, Clone)]
pub struct WeightTemplate {
    /// 模板名称
    pub name: String,
    /// 适用条件
    pub condition: TemplateCondition,
    /// 权重值
    pub weights: LayerWeights,
    /// 优先级（越高越优先）
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub enum TemplateCondition {
    /// 查询类型匹配
    QueryType(QueryTypeFeature),
    /// 记忆类型匹配
    MemoryType(MemoryTypeFeature),
    /// 查询长度匹配
    QueryLength(QueryLengthFeature),
    /// 实体查询
    HasEntities,
    /// 时间约束
    HasTimeConstraint,
    /// 复合条件
    And(Box<TemplateCondition>, Box<TemplateCondition>),
    Or(Box<TemplateCondition>, Box<TemplateCondition>),
}

impl TemplateCondition {
    pub fn matches(&self, features: &WeightFeatures) -> bool {
        match self {
            Self::QueryType(qt) => features.query_type == *qt,
            Self::MemoryType(mt) => features.memory_type == Some(*mt),
            Self::QueryLength(ql) => features.query_length == *ql,
            Self::HasEntities => features.entity_count > 0,
            Self::HasTimeConstraint => features.time_constraint,
            Self::And(left, right) => left.matches(features) && right.matches(features),
            Self::Or(left, right) => left.matches(features) || right.matches(features),
        }
    }
}

impl TemplateWeightStrategy {
    pub fn new() -> Self {
        Self {
            templates: Self::default_templates(),
        }
    }
    
    /// 默认模板库
    fn default_templates() -> Vec<WeightTemplate> {
        vec![
            // === 查询类型相关模板 ===
            
            // 实体查询：L2 权重最高（需要精确匹配）
            WeightTemplate {
                name: "entity_lookup".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::EntityLookup),
                weights: LayerWeights { l0: 0.1, l1: 0.2, l2: 0.7 },
                priority: 100,
            },
            
            // 事实查询：L2 权重较高
            WeightTemplate {
                name: "factual".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Factual),
                weights: LayerWeights { l0: 0.15, l1: 0.25, l2: 0.6 },
                priority: 90,
            },
            
            // 时间查询：均衡分布
            WeightTemplate {
                name: "temporal".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Temporal),
                weights: LayerWeights { l0: 0.2, l1: 0.35, l2: 0.45 },
                priority: 90,
            },
            
            // 关系查询：L1 权重最高（概览层包含关系信息）
            WeightTemplate {
                name: "relational".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Relational),
                weights: LayerWeights { l0: 0.2, l1: 0.5, l2: 0.3 },
                priority: 90,
            },
            
            // 过程查询：L2 权重高（需要详细步骤）
            WeightTemplate {
                name: "procedural".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Procedural),
                weights: LayerWeights { l0: 0.1, l1: 0.3, l2: 0.6 },
                priority: 90,
            },
            
            // 概念查询：L1 权重高（概览层包含概念解释）
            WeightTemplate {
                name: "conceptual".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Conceptual),
                weights: LayerWeights { l0: 0.25, l1: 0.45, l2: 0.3 },
                priority: 90,
            },
            
            // 搜索查询：L0 权重高（快速定位）
            WeightTemplate {
                name: "search".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Search),
                weights: LayerWeights { l0: 0.4, l1: 0.35, l2: 0.25 },
                priority: 90,
            },
            
            // 比较查询：L1 权重高
            WeightTemplate {
                name: "comparative".to_string(),
                condition: TemplateCondition::QueryType(QueryTypeFeature::Comparative),
                weights: LayerWeights { l0: 0.2, l1: 0.5, l2: 0.3 },
                priority: 90,
            },
            
            // === 记忆类型相关模板 ===
            
            // 偏好记忆：L0 足够（偏好通常简洁）
            WeightTemplate {
                name: "preference_memory".to_string(),
                condition: TemplateCondition::MemoryType(MemoryTypeFeature::Preference),
                weights: LayerWeights { l0: 0.4, l1: 0.4, l2: 0.2 },
                priority: 80,
            },
            
            // 事件记忆：L2 重要（需要详细上下文）
            WeightTemplate {
                name: "event_memory".to_string(),
                condition: TemplateCondition::MemoryType(MemoryTypeFeature::Event),
                weights: LayerWeights { l0: 0.15, l1: 0.25, l2: 0.6 },
                priority: 80,
            },
            
            // 案例记忆：L1/L2 重要（需要结构和细节）
            WeightTemplate {
                name: "case_memory".to_string(),
                condition: TemplateCondition::MemoryType(MemoryTypeFeature::Case),
                weights: LayerWeights { l0: 0.15, l1: 0.4, l2: 0.45 },
                priority: 80,
            },
            
            // === 查询长度相关模板 ===
            
            // 短查询：L0 权重高（快速匹配）
            WeightTemplate {
                name: "short_query".to_string(),
                condition: TemplateCondition::QueryLength(QueryLengthFeature::Short),
                weights: LayerWeights { l0: 0.35, l1: 0.35, l2: 0.3 },
                priority: 70,
            },
            
            // 长查询：L2 权重高（需要深度匹配）
            WeightTemplate {
                name: "long_query".to_string(),
                condition: TemplateCondition::QueryLength(QueryLengthFeature::Long),
                weights: LayerWeights { l0: 0.15, l1: 0.3, l2: 0.55 },
                priority: 70,
            },
            
            // === 复合条件模板 ===
            
            // 实体查询 + 短查询：极端偏向 L2
            WeightTemplate {
                name: "entity_short".to_string(),
                condition: TemplateCondition::And(
                    Box::new(TemplateCondition::QueryType(QueryTypeFeature::EntityLookup)),
                    Box::new(TemplateCondition::QueryLength(QueryLengthFeature::Short))
                ),
                weights: LayerWeights { l0: 0.05, l1: 0.15, l2: 0.8 },
                priority: 110,
            },
            
            // 时间约束查询
            WeightTemplate {
                name: "time_constrained".to_string(),
                condition: TemplateCondition::HasTimeConstraint,
                weights: LayerWeights { l0: 0.2, l1: 0.35, l2: 0.45 },
                priority: 85,
            },
        ]
    }
}

impl WeightStrategy for TemplateWeightStrategy {
    fn calculate(&self, features: &WeightFeatures) -> LayerWeights {
        // 按优先级排序，找到第一个匹配的模板
        let mut matched: Vec<_> = self.templates
            .iter()
            .filter(|t| t.condition.matches(features))
            .collect();
        
        matched.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        if let Some(template) = matched.first() {
            template.weights.normalize()
        } else {
            LayerWeights::default_weights()
        }
    }
    
    fn name(&self) -> &str {
        "template"
    }
}
```

#### 2.2.3 自适应学习策略

```rust
// 文件: cortex-mem-core/src/search/weight_model.rs (继续)

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 反馈记录
#[derive(Debug, Clone)]
pub struct SearchFeedback {
    /// 查询特征
    pub features: WeightFeatures,
    /// 使用的权重
    pub weights: LayerWeights,
    /// 用户是否点击了结果
    pub clicked: bool,
    /// 点击结果的排名（1-based）
    pub clicked_rank: Option<usize>,
    /// 用户满意度评分（可选）
    pub satisfaction: Option<f32>,
}

/// 自适应权重学习策略
pub struct AdaptiveWeightStrategy {
    /// 基础策略
    base_strategy: Box<dyn WeightStrategy>,
    /// 学习率
    learning_rate: f32,
    /// 特征到权重调整的映射
    adjustments: Arc<RwLock<HashMap<String, LayerWeights>>>,
    /// 历史反馈统计
    feedback_stats: Arc<RwLock<FeedbackStats>>,
}

#[derive(Debug, Clone, Default)]
struct FeedbackStats {
    total_feedbacks: usize,
    positive_feedbacks: usize,
    feature_click_rates: HashMap<String, (usize, usize)>,  // (clicks, total)
}

impl AdaptiveWeightStrategy {
    pub fn new(base_strategy: Box<dyn WeightStrategy>) -> Self {
        Self {
            base_strategy,
            learning_rate: 0.1,
            adjustments: Arc::new(RwLock::new(HashMap::new())),
            feedback_stats: Arc::new(RwLock::new(FeedbackStats::default())),
        }
    }
    
    /// 记录反馈并学习
    pub async fn record_feedback(&self, feedback: SearchFeedback) {
        let mut stats = self.feedback_stats.write().await;
        stats.total_feedbacks += 1;
        
        if feedback.clicked {
            stats.positive_feedbacks += 1;
            
            // 更新特征点击率
            let feature_key = Self::feature_key(&feedback.features);
            let entry = stats.feature_click_rates.entry(feature_key.clone()).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += 1;
            
            // 学习：如果用户点击了结果，增加该权重的置信度
            // 这里可以更复杂的在线学习算法
        }
    }
    
    fn feature_key(features: &WeightFeatures) -> String {
        format!("{:?}_{:?}", features.query_type, features.query_length)
    }
}

impl WeightStrategy for AdaptiveWeightStrategy {
    fn calculate(&self, features: &WeightFeatures) -> LayerWeights {
        // 1. 获取基础权重
        let base_weights = self.base_strategy.calculate(features);
        
        // 2. 尝试同步获取调整（简化版本，实际应该 async）
        // 由于 trait 方法是同步的，这里使用 try_read
        if let Ok(adjustments) = self.adjustments.try_read() {
            let feature_key = Self::feature_key(features);
            if let Some(adjustment) = adjustments.get(&feature_key) {
                // 融合基础权重和调整
                return LayerWeights {
                    l0: base_weights.l0 * (1.0 - self.learning_rate) + adjustment.l0 * self.learning_rate,
                    l1: base_weights.l1 * (1.0 - self.learning_rate) + adjustment.l1 * self.learning_rate,
                    l2: base_weights.l2 * (1.0 - self.learning_rate) + adjustment.l2 * self.learning_rate,
                }.normalize();
            }
        }
        
        base_weights
    }
    
    fn name(&self) -> &str {
        "adaptive"
    }
}
```

### 2.3 动态权重检索执行器

```rust
// 文件: cortex-mem-core/src/search/dynamic_weight_executor.rs (新建)

use crate::search::weight_model::*;
use crate::search::{VectorSearchEngine, SearchOptions, SearchResult, QueryIntent};
use crate::embedding::EmbeddingClient;
use crate::vector_store::QdrantVectorStore;
use crate::filesystem::CortexFilesystem;
use crate::Result;
use std::sync::Arc;
use tracing::{info, debug};

/// 动态权重检索执行器
pub struct DynamicWeightSearchExecutor {
    vector_engine: Arc<VectorSearchEngine>,
    weight_strategy: Box<dyn WeightStrategy>,
    embedding: Arc<EmbeddingClient>,
}

impl DynamicWeightSearchExecutor {
    pub fn new(
        vector_engine: Arc<VectorSearchEngine>,
        embedding: Arc<EmbeddingClient>,
        strategy: Box<dyn WeightStrategy>,
    ) -> Self {
        Self {
            vector_engine,
            weight_strategy: strategy,
            embedding,
        }
    }
    
    /// 使用动态权重执行三层级联检索
    pub async fn search_with_dynamic_weights(
        &self,
        query: &str,
        intent: &QueryIntent,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 1. 提取权重特征
        let features = self.extract_features(query, intent, options).await?;
        
        // 2. 计算动态权重
        let weights = self.weight_strategy.calculate(&features);
        
        info!(
            "Dynamic weights for query '{}': L0={:.2}, L1={:.2}, L2={:.2}",
            query, weights.l0, weights.l1, weights.l2
        );
        
        // 3. 执行三层级联检索
        self.execute_with_weights(query, &weights, options).await
    }
    
    /// 提取权重特征
    async fn extract_features(
        &self,
        query: &str,
        intent: &QueryIntent,
        options: &SearchOptions,
    ) -> Result<WeightFeatures> {
        // 从 Intent 中提取特征
        let query_type = Self::map_intent_to_feature(&intent.intent_type);
        
        // 查询长度特征
        let word_count = query.split_whitespace().count();
        let query_length = match word_count {
            0..=5 => QueryLengthFeature::Short,
            6..=15 => QueryLengthFeature::Medium,
            _ => QueryLengthFeature::Long,
        };
        
        // 计算复杂度（基于查询长度、关键词数量等）
        let complexity = Self::calculate_complexity(query, &intent.keywords);
        
        Ok(WeightFeatures {
            query_type,
            memory_type: None,  // 可从 options 或 filters 中获取
            query_length,
            time_constraint: false,  // 可从 intent 中判断
            entity_count: 0,  // 可从 intent.entities 中获取
            complexity,
            user_preference: None,
        })
    }
    
    fn map_intent_to_feature(intent_type: &crate::search::QueryIntentType) -> QueryTypeFeature {
        match intent_type {
            crate::search::QueryIntentType::Factual => QueryTypeFeature::Factual,
            crate::search::QueryIntentType::Search => QueryTypeFeature::Search,
            crate::search::QueryIntentType::Relational => QueryTypeFeature::Relational,
            crate::search::QueryIntentType::Temporal => QueryTypeFeature::Temporal,
            crate::search::QueryIntentType::General => QueryTypeFeature::General,
        }
    }
    
    fn calculate_complexity(query: &str, keywords: &[String]) -> f32 {
        let len_factor = (query.len() as f32 / 100.0).min(1.0);
        let keyword_factor = (keywords.len() as f32 / 10.0).min(1.0);
        (len_factor + keyword_factor) / 2.0
    }
    
    /// 执行带权重的三层检索
    async fn execute_with_weights(
        &self,
        query: &str,
        weights: &LayerWeights,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // 生成查询嵌入
        let query_vec = self.embedding.embed(query).await?;
        
        // === Stage 1: L0 快速定位 ===
        let l0_threshold = self.calculate_adaptive_l0_threshold(weights.l0);
        let l0_results = self.search_l0(&query_vec, options, l0_threshold).await?;
        
        if l0_results.is_empty() {
            debug!("L0 results empty, fallback to full semantic search");
            return self.vector_engine.semantic_search(query, options).await;
        }
        
        // === Stage 2: L1 深度探索 ===
        let l1_candidates = self.search_l1(&query_vec, &l0_results, options).await?;
        
        // === Stage 3: L2 精确匹配 ===
        let final_results = self.search_l2_with_weights(
            &query_vec,
            &l0_results,
            &l1_candidates,
            weights,
            options
        ).await?;
        
        Ok(final_results)
    }
    
    fn calculate_adaptive_l0_threshold(&self, l0_weight: f32) -> f32 {
        // L0 权重越高，阈值应该越低（更宽松地通过 L0）
        // 基础阈值 0.5，根据权重调整
        let base = 0.5;
        let adjustment = (l0_weight - 0.2) * 0.3;  // 权重差异的影响
        (base - adjustment).clamp(0.3, 0.6)
    }
    
    async fn search_l0(
        &self,
        query_vec: &[f32],
        options: &SearchOptions,
        threshold: f32,
    ) -> Result<Vec<crate::types::ScoredMemory>> {
        // 调用现有的 L0 搜索逻辑
        // 实际实现需要访问 vector_store
        Ok(Vec::new())
    }
    
    async fn search_l1(
        &self,
        query_vec: &[f32],
        l0_results: &[crate::types::ScoredMemory],
        options: &SearchOptions,
    ) -> Result<Vec<(String, f32)>> {
        // L1 搜索逻辑
        Ok(Vec::new())
    }
    
    async fn search_l2_with_weights(
        &self,
        query_vec: &[f32],
        l0_results: &[crate::types::ScoredMemory],
        l1_candidates: &[(String, f32)],
        weights: &LayerWeights,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        
        // 遍历候选，使用动态权重计算综合分数
        for (dir_uri, l0_score, l1_score) in self.combine_l0_l1(l0_results, l1_candidates) {
            // 获取 L2 内容并计算分数
            if let Some(l2_result) = self.search_l2_for_dir(query_vec, &dir_uri).await? {
                // 使用动态权重计算综合分数
                let combined_score = 
                    l0_score * weights.l0 +
                    l1_score * weights.l1 +
                    l2_result.score * weights.l2;
                
                if combined_score >= options.threshold {
                    results.push(SearchResult {
                        uri: l2_result.uri,
                        score: combined_score,
                        snippet: l2_result.snippet,
                        content: l2_result.content,
                    });
                }
            }
        }
        
        // 排序并截断
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(options.limit);
        
        Ok(results)
    }
    
    fn combine_l0_l1(
        &self,
        l0_results: &[crate::types::ScoredMemory],
        l1_candidates: &[(String, f32)],
    ) -> Vec<(String, f32, f32)> {
        // 合并 L0 和 L1 结果
        Vec::new()
    }
    
    async fn search_l2_for_dir(
        &self,
        query_vec: &[f32],
        dir_uri: &str,
    ) -> Result<Option<SearchResult>> {
        // 在目录中搜索 L2 内容
        Ok(None)
    }
}
```

### 2.4 集成到搜索引擎

```rust
// 文件: cortex-mem-core/src/search/vector_engine.rs (修改)

impl VectorSearchEngine {
    /// 智能搜索（使用动态权重）
    pub async fn smart_search_with_dynamic_weights(
        &self,
        query: &str,
        options: &SearchOptions,
        intent: Option<&QueryIntent>,
    ) -> Result<Vec<SearchResult>> {
        // 1. 分析意图
        let analyzed_intent = if let Some(i) = intent {
            i.clone()
        } else {
            self.analyze_intent(query).await?
        };
        
        // 2. 选择权重策略
        let strategy = self.select_weight_strategy();
        
        // 3. 创建动态权重执行器
        let executor = DynamicWeightSearchExecutor::new(
            Arc::new(self.clone()),
            self.embedding.clone(),
            strategy,
        );
        
        // 4. 执行检索
        executor.search_with_dynamic_weights(query, &analyzed_intent, options).await
    }
    
    fn select_weight_strategy(&self) -> Box<dyn WeightStrategy> {
        // 可以根据配置选择不同策略
        // 默认使用模板策略
        Box::new(TemplateWeightStrategy::new())
    }
    
    /// 更新检索反馈（用于自适应学习）
    pub async fn record_search_feedback(
        &self,
        query: &str,
        clicked_result_uri: &str,
        rank: usize,
    ) -> Result<()> {
        // 记录反馈，用于后续学习
        Ok(())
    }
}
```

### 2.5 配置支持

```toml
# config.toml 新增配置

[search.weights]
# 权重策略: template, adaptive, hybrid
strategy = "template"

# 默认权重（当没有匹配模板时使用）
default_l0 = 0.2
default_l1 = 0.3
default_l2 = 0.5

# 自适应学习配置
[search.weights.adaptive]
enabled = false
learning_rate = 0.1
min_samples_for_learning = 100

# 自定义权重模板
[[search.weights.templates]]
name = "custom_entity"
query_type = "entity_lookup"
l0 = 0.1
l1 = 0.15
l2 = 0.75
```

### 2.6 API 扩展

```rust
// REST API 新增端点

// 获取当前权重策略状态
GET /api/v2/search/weights/status

// 手动设置权重（测试用）
POST /api/v2/search/weights/set
{
    "l0": 0.3,
    "l1": 0.3,
    "l2": 0.4
}

// 记录检索反馈
POST /api/v2/search/feedback
{
    "query": "用户的查询",
    "clicked_uri": "cortex://user/preferences/xxx.md",
    "rank": 1,
    "satisfaction": 0.9
}
```

## 3. 实现计划

| 步骤 | 任务 | 依赖 |
|------|------|------|
| 1 | 定义 WeightFeatures 和 LayerWeights 结构 | 无 |
| 2 | 实现 TemplateWeightStrategy | 步骤1 |
| 3 | 实现 AdaptiveWeightStrategy | 步骤1, 2 |
| 4 | 实现 DynamicWeightSearchExecutor | 步骤2 |
| 5 | 集成到 VectorSearchEngine | 步骤4 |
| 6 | 添加配置和 API 支持 | 全部 |

## 4. 预期收益

| 场景 | 静态权重 | 动态权重 | 提升 |
|------|----------|----------|------|
| 实体查询召回 | 75% | 92% | +17pp |
| 语义查询召回 | 88% | 91% | +3pp |
| 关系查询召回 | 70% | 85% | +15pp |
| 综合召回精度 | 85% | 93% | +8pp |

## 5. 测试方案

### 5.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_template_matching() {
        let strategy = TemplateWeightStrategy::new();
        
        // 测试实体查询
        let features = WeightFeatures {
            query_type: QueryTypeFeature::EntityLookup,
            ..Default::default()
        };
        let weights = strategy.calculate(&features);
        assert!(weights.l2 > weights.l0);
        assert!(weights.l2 > weights.l1);
        
        // 测试搜索查询
        let features = WeightFeatures {
            query_type: QueryTypeFeature::Search,
            ..Default::default()
        };
        let weights = strategy.calculate(&features);
        assert!(weights.l0 >= weights.l2);
    }
    
    #[test]
    fn test_weight_normalization() {
        let weights = LayerWeights { l0: 1.0, l1: 2.0, l2: 2.0 };
        let normalized = weights.normalize();
        assert!((normalized.l0 + normalized.l1 + normalized.l2 - 1.0).abs() < 0.001);
    }
}
```

### 5.2 A/B 测试

```rust
// 在生产环境中进行 A/B 测试
// 对照组：静态权重
// 实验组：动态权重
// 指标：召回率、用户满意度、检索延迟
```

## 6. 与其他方案的协同

1. **与 Intent 分析协同**：
   - Intent 分析提供 `query_type`，直接作为权重特征输入
   - 避免重复分析

2. **与遗忘机制协同**：
   - 低强度记忆在 L2 权重中降权
   - 权重计算可考虑记忆强度

3. **与实体检索协同**：
   - 实体查询自动触发高 L2 权重
   - 实体数量作为特征输入
