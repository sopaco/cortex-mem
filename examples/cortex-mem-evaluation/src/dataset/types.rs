//! 数据集类型定义

use cortex_mem_core::{Memory, MemoryType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 召回率测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallTestCase {
    /// 查询ID
    pub query_id: String,
    /// 查询文本
    pub query: String,
    /// 相关记忆ID列表
    pub relevant_memory_ids: Vec<String>,
    /// 查询类别
    pub category: String,
    /// 查询复杂度：simple, medium, complex
    pub complexity: String,
}

/// 召回率测试数据集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallTestDataset {
    /// 测试用例列表
    pub test_cases: Vec<RecallTestCase>,
    /// 记忆库（ID -> 记忆内容）
    pub memories: HashMap<String, Memory>,
    /// 数据集元数据
    pub metadata: DatasetMetadata,
}

/// 有效性测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessTestCase {
    /// 测试用例ID
    pub test_case_id: String,
    /// 输入文本
    pub input_text: String,
    /// 预期提取的关键事实
    pub expected_facts: Vec<String>,
    /// 预期记忆类型
    pub expected_memory_type: MemoryType,
    /// 预期重要性评分（1-10）
    pub expected_importance_score: u8,
    /// 测试类别
    pub category: String,
    /// 是否包含重复内容
    pub contains_duplicate: bool,
    /// 是否需要更新现有记忆
    pub requires_update: bool,
    /// 现有记忆ID（如果需要更新）
    pub existing_memory_id: Option<String>,
}

/// 有效性测试数据集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessTestDataset {
    /// 测试用例列表
    pub test_cases: Vec<EffectivenessTestCase>,
    /// 现有记忆库（用于更新测试）
    pub existing_memories: HashMap<String, Memory>,
    /// 数据集元数据
    pub metadata: DatasetMetadata,
}

/// 数据集元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    /// 数据集名称
    pub name: String,
    /// 创建时间
    pub created_at: String,
    /// 版本
    pub version: String,
    /// 总测试用例数
    pub total_test_cases: usize,
    /// 总记忆数
    pub total_memories: usize,
    /// 平均相关记忆数
    pub avg_relevant_memories: f64,
}

/// 性能测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTestConfig {
    /// 测试的记忆库大小列表
    pub memory_sizes: Vec<usize>,
    /// 并发用户数列表
    pub concurrent_users: Vec<usize>,
    /// 测试持续时间（秒）
    pub test_duration_seconds: u64,
    /// 预热时间（秒）
    pub warmup_seconds: u64,
    /// 操作类型：add, search, update, mixed
    pub operation_types: Vec<String>,
}

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult<T> {
    /// 测试ID
    pub test_id: String,
    /// 测试名称
    pub test_name: String,
    /// 测试结果
    pub result: T,
    /// 测试开始时间
    pub start_time: String,
    /// 测试结束时间
    pub end_time: String,
    /// 测试持续时间（秒）
    pub duration_seconds: f64,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
}

/// 基准测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// 基准测试名称
    pub benchmark_name: String,
    /// 迭代次数
    pub iterations: usize,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f64,
    /// 最小执行时间（毫秒）
    pub min_execution_time_ms: f64,
    /// 最大执行时间（毫秒）
    pub max_execution_time_ms: f64,
    /// 标准差
    pub std_deviation_ms: f64,
    /// 吞吐量（操作/秒）
    pub throughput_ops_per_sec: f64,
    /// 内存使用（MB）
    pub memory_usage_mb: f64,
    /// CPU使用率（%）
    pub cpu_usage_percent: f64,
}

/// 负载测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    /// 负载测试名称
    pub load_test_name: String,
    /// 并发用户数
    pub concurrent_users: usize,
    /// 总请求数
    pub total_requests: usize,
    /// 成功请求数
    pub successful_requests: usize,
    /// 失败请求数
    pub failed_requests: usize,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// P95响应时间（毫秒）
    pub p95_response_time_ms: f64,
    /// P99响应时间（毫秒）
    pub p99_response_time_ms: f64,
    /// 吞吐量（请求/秒）
    pub throughput_requests_per_sec: f64,
    /// 错误率（%）
    pub error_rate_percent: f64,
}

/// 可扩展性测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalabilityTestResult {
    /// 可扩展性测试名称
    pub scalability_test_name: String,
    /// 不同规模下的性能
    pub performance_by_scale: HashMap<usize, ScalePerformance>,
    /// 线性扩展因子
    pub linear_scaling_factor: f64,
    /// 瓶颈点
    pub bottleneck_point: Option<usize>,
}

/// 规模性能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalePerformance {
    /// 规模（记忆数量或用户数量）
    pub scale: usize,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 吞吐量（操作/秒）
    pub throughput_ops_per_sec: f64,
    /// 资源使用分数（0-1）
    pub resource_utilization_score: f64,
}

/// 数据集验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetValidationResult {
    /// 数据集名称
    pub dataset_name: String,
    /// 验证时间
    pub validation_time: String,
    /// 是否有效
    pub is_valid: bool,
    /// 验证问题列表
    pub issues: Vec<ValidationIssue>,
    /// 统计信息
    pub statistics: DatasetStatistics,
}

/// 验证问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// 问题类型
    pub issue_type: String,
    /// 问题描述
    pub description: String,
    /// 严重程度：low, medium, high, critical
    pub severity: String,
    /// 受影响的项目
    pub affected_items: Vec<String>,
    /// 建议的修复方法
    pub suggested_fix: Option<String>,
}

/// 数据集统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStatistics {
    /// 总项目数
    pub total_items: usize,
    /// 有效项目数
    pub valid_items: usize,
    /// 无效项目数
    pub invalid_items: usize,
    /// 平均项目长度
    pub avg_item_length: f64,
    /// 类别分布
    pub category_distribution: HashMap<String, usize>,
    /// 复杂度分布
    pub complexity_distribution: HashMap<String, usize>,
    /// 记忆类型分布
    pub memory_type_distribution: HashMap<String, usize>,
}