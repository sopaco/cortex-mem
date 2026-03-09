mod vector_engine;
mod weight_model;

pub use vector_engine::{SearchOptions, SearchResult, VectorSearchEngine};
pub use weight_model::{LayerWeights, weights_for_intent};

use serde::{Deserialize, Serialize};

/// 查询意图类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryIntentType {
    /// 实体查询：查询特定命名实体（人名、地名、工具名等）
    EntityLookup,
    /// 事实性查询：询问某个具体事实
    Factual,
    /// 时间性查询：涉及时间引用
    Temporal,
    /// 关系性查询：比较或关联多个概念
    Relational,
    /// 搜索类查询：查找/列出内容
    Search,
    /// 通用查询
    General,
}

/// 时间约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConstraint {
    pub start: Option<String>,
    pub end: Option<String>,
}

/// 增强的查询意图分析结果（由 LLM 单次调用生成）
#[derive(Debug, Clone)]
pub struct EnhancedQueryIntent {
    /// 原始查询
    pub original_query: String,
    /// LLM 改写后的查询（用于 embedding）
    pub rewritten_query: String,
    /// 关键词列表（LLM 提取，支持中文）
    pub keywords: Vec<String>,
    /// 实体列表（人名、地名、技术名词等）
    pub entities: Vec<String>,
    /// 意图类型
    pub intent_type: QueryIntentType,
    /// 时间约束（可选）
    pub time_constraint: Option<TimeConstraint>,
}
