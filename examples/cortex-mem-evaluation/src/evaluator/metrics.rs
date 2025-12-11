//! 评估指标定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 召回率评估指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallMetrics {
    /// 精确率@K
    pub precision_at_k: HashMap<usize, f64>,
    /// 召回率@K
    pub recall_at_k: HashMap<usize, f64>,
    /// 平均精确率均值
    pub mean_average_precision: f64,
    /// 归一化折损累计增益
    pub normalized_discounted_cumulative_gain: f64,
    /// 不同相似度阈值下的指标
    pub metrics_by_threshold: HashMap<String, ThresholdMetrics>, // 使用字符串作为键
    /// 查询级别的详细结果
    pub query_level_results: Vec<QueryResult>,
}

/// 相似度阈值相关的指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdMetrics {
    /// 阈值
    pub threshold: f64,
    /// 精确率
    pub precision: f64,
    /// 召回率
    pub recall: f64,
    /// F1分数
    pub f1_score: f64,
    /// 返回结果的平均数量
    pub avg_results_returned: f64,
}

/// 查询级别的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// 查询ID
    pub query_id: String,
    /// 查询内容
    pub query: String,
    /// 相关记忆数量
    pub relevant_memories: usize,
    /// 检索到的相关记忆数量
    pub retrieved_relevant: usize,
    /// 检索到的总记忆数量
    pub retrieved_total: usize,
    /// 精确率
    pub precision: f64,
    /// 召回率
    pub recall: f64,
    /// 平均精确率
    pub average_precision: f64,
}

/// 记忆有效性评估指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessMetrics {
    /// 事实提取准确性
    pub fact_extraction_accuracy: FactExtractionMetrics,
    /// 记忆分类准确性
    pub classification_accuracy: ClassificationMetrics,
    /// 重要性评估合理性
    pub importance_evaluation_quality: ImportanceMetrics,
    /// 去重效果
    pub deduplication_effectiveness: DeduplicationMetrics,
    /// 记忆更新正确性
    pub memory_update_correctness: UpdateMetrics,
    /// 综合得分
    pub overall_score: f64,
}

/// 事实提取指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactExtractionMetrics {
    /// 精确率
    pub precision: f64,
    /// 召回率
    pub recall: f64,
    /// F1分数
    pub f1_score: f64,
    /// 提取的关键事实数量
    pub facts_extracted: usize,
    /// 正确提取的事实数量
    pub correct_facts: usize,
    /// 详细结果
    pub detailed_results: Vec<FactExtractionResult>,
}

/// 事实提取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactExtractionResult {
    /// 输入文本
    pub input_text: String,
    /// 提取的事实
    pub extracted_facts: Vec<String>,
    /// 基准事实
    pub ground_truth_facts: Vec<String>,
    /// 匹配的事实数量
    pub matched_facts: usize,
    /// 是否完全匹配
    pub is_perfect_match: bool,
}

/// 分类指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetrics {
    /// 准确率
    pub accuracy: f64,
    /// 精确率（按类别）
    pub precision_by_class: HashMap<String, f64>,
    /// 召回率（按类别）
    pub recall_by_class: HashMap<String, f64>,
    /// F1分数（按类别）
    pub f1_by_class: HashMap<String, f64>,
    /// 混淆矩阵
    pub confusion_matrix: HashMap<String, HashMap<String, usize>>,
}

/// 重要性评估指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceMetrics {
    /// 相关性分数（与人工标注的相关性）
    pub correlation_score: f64,
    /// 平均绝对误差
    pub mean_absolute_error: f64,
    /// 均方根误差
    pub root_mean_squared_error: f64,
    /// 评分分布
    pub score_distribution: HashMap<usize, usize>,
}

/// 去重指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationMetrics {
    /// 重复检测精确率
    pub duplicate_detection_precision: f64,
    /// 重复检测召回率
    pub duplicate_detection_recall: f64,
    /// 合并正确率
    pub merge_accuracy: f64,
    /// 检测到的重复对数量
    pub duplicate_pairs_detected: usize,
    /// 实际重复对数量
    pub actual_duplicate_pairs: usize,
}

/// 记忆更新指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMetrics {
    /// 更新操作正确率
    pub update_operation_accuracy: f64,
    /// 合并操作正确率
    pub merge_operation_accuracy: f64,
    /// 冲突解决正确率
    pub conflict_resolution_accuracy: f64,
    /// 更新后的记忆质量评分
    pub updated_memory_quality: f64,
}

/// 性能评估指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 延迟指标
    pub latency: LatencyMetrics,
    /// 吞吐量指标
    pub throughput: ThroughputMetrics,
    /// 资源使用指标
    pub resource_usage: ResourceMetrics,
    /// 并发性能指标
    pub concurrency: ConcurrencyMetrics,
    /// 可扩展性指标
    pub scalability: ScalabilityMetrics,
}

/// 延迟指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// 添加记忆的平均延迟（毫秒）
    pub add_memory_avg_ms: f64,
    /// 搜索记忆的平均延迟（毫秒）
    pub search_memory_avg_ms: f64,
    /// 更新记忆的平均延迟（毫秒）
    pub update_memory_avg_ms: f64,
    /// 删除记忆的平均延迟（毫秒）
    pub delete_memory_avg_ms: f64,
    /// P95延迟（毫秒）
    pub p95_latency_ms: f64,
    /// P99延迟（毫秒）
    pub p99_latency_ms: f64,
    /// 最大延迟（毫秒）
    pub max_latency_ms: f64,
    /// 延迟分布
    pub latency_distribution: HashMap<String, f64>,
}

/// 吞吐量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// 每秒操作数
    pub operations_per_second: f64,
    /// 每秒添加操作数
    pub add_ops_per_second: f64,
    /// 每秒搜索操作数
    pub search_ops_per_second: f64,
    /// 每秒更新操作数
    pub update_ops_per_second: f64,
    /// 峰值吞吐量
    pub peak_throughput: f64,
    /// 可持续吞吐量
    pub sustainable_throughput: f64,
}

/// 资源使用指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// 内存使用（MB）
    pub memory_usage_mb: f64,
    /// CPU使用率（%）
    pub cpu_usage_percent: f64,
    /// 磁盘使用（MB）
    pub disk_usage_mb: f64,
    /// 网络使用（KB/s）
    pub network_usage_kbps: f64,
    /// 资源使用趋势
    pub usage_trend: Vec<ResourceSnapshot>,
}

/// 资源快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// 时间戳
    pub timestamp: i64,
    /// 内存使用（MB）
    pub memory_mb: f64,
    /// CPU使用率（%）
    pub cpu_percent: f64,
}

/// 并发性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyMetrics {
    /// 并发用户数
    pub concurrent_users: usize,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 吞吐量（操作/秒）
    pub throughput_ops_per_sec: f64,
    /// 错误率（%）
    pub error_rate_percent: f64,
    /// 成功率（%）
    pub success_rate_percent: f64,
}

/// 可扩展性指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalabilityMetrics {
    /// 不同记忆库大小下的性能
    pub performance_by_memory_size: HashMap<usize, SizePerformance>,
    /// 线性扩展因子
    pub linear_scaling_factor: f64,
    /// 瓶颈点
    pub bottleneck_point: Option<usize>,
}

/// 大小性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizePerformance {
    /// 记忆库大小
    pub memory_size: usize,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 吞吐量（操作/秒）
    pub throughput_ops_per_sec: f64,
    /// 内存使用（MB）
    pub memory_usage_mb: f64,
}

/// 综合评估报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveReport {
    /// 评估时间戳
    pub evaluation_timestamp: i64,
    /// 评估配置摘要
    pub config_summary: String,
    /// 召回率评估结果
    pub recall_metrics: Option<RecallMetrics>,
    /// 有效性评估结果
    pub effectiveness_metrics: Option<EffectivenessMetrics>,
    /// 性能评估结果
    pub performance_metrics: Option<PerformanceMetrics>,
    /// 总体评分
    pub overall_score: f64,
    /// 关键发现
    pub key_findings: Vec<String>,
    /// 改进建议
    pub recommendations: Vec<String>,
    /// 评估版本
    pub evaluation_version: String,
}