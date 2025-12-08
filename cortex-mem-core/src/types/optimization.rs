use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 优化请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRequest {
    pub optimization_id: Option<String>,
    pub strategy: OptimizationStrategy,
    pub filters: OptimizationFilters,
    pub aggressive: bool,
    pub dry_run: bool,
    pub timeout_minutes: Option<u64>,
}

/// 优化策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationStrategy {
    /// 全面优化
    Full,
    /// 增量优化
    Incremental,
    /// 批量优化
    Batch,
    /// 仅去重
    Deduplication,
    /// 仅相关性优化
    Relevance,
    /// 仅质量优化
    Quality,
    /// 仅空间优化
    Space,
}

/// 优化过滤器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptimizationFilters {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub memory_type: Option<super::MemoryType>,
    pub date_range: Option<DateRange>,
    pub importance_range: Option<Range<f32>>,
    pub custom_filters: HashMap<String, serde_json::Value>,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// 数值范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

/// 优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub optimization_id: String,
    pub strategy: OptimizationStrategy,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub issues_found: Vec<OptimizationIssue>,
    pub actions_performed: Vec<OptimizationAction>,
    pub metrics: Option<OptimizationMetrics>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// 优化问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationIssue {
    pub id: String,
    pub kind: IssueKind,
    pub severity: IssueSeverity,
    pub description: String,
    pub affected_memories: Vec<String>,
    pub recommendation: String,
}

/// 问题类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IssueKind {
    Duplicate,
    LowQuality,
    Outdated,
    PoorClassification,
    SpaceInefficient,
}

/// 问题严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 优化操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationAction {
    Merge { memories: Vec<String> },
    Delete { memory_id: String },
    Update { memory_id: String, updates: MemoryUpdates },
    Reclassify { memory_id: String },
    Archive { memory_id: String },
}

/// 内存更新内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUpdates {
    pub content: Option<String>,
    pub memory_type: Option<super::MemoryType>,
    pub importance_score: Option<f32>,
    pub entities: Option<Vec<String>>,
    pub topics: Option<Vec<String>>,
    pub custom_metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 优化计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPlan {
    pub optimization_id: String,
    pub strategy: OptimizationStrategy,
    pub created_at: DateTime<Utc>,
    pub estimated_duration_minutes: u64,
    pub issues: Vec<OptimizationIssue>,
    pub actions: Vec<OptimizationAction>,
    pub filters: OptimizationFilters,
}

/// 优化状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStatus {
    pub optimization_id: String,
    pub status: OptimizationStatusType,
    pub progress: u8,
    pub current_phase: String,
    pub started_at: Option<DateTime<Utc>>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// 优化状态类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStatusType {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// 优化指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetrics {
    pub total_optimizations: u64,
    pub last_optimization: Option<DateTime<Utc>>,
    pub memory_count_before: usize,
    pub memory_count_after: usize,
    pub saved_space_mb: f64,
    pub deduplication_rate: f32,
    pub quality_improvement: f32,
    pub performance_improvement: f32,
}

/// 优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub auto_optimize: bool,
    pub trigger_config: TriggerConfig,
    pub strategy_configs: StrategyConfigs,
    pub execution_config: ExecutionConfig,
    pub safety_config: SafetyConfig,
}

/// 触发器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub auto_triggers: Vec<AutoTriggerConfig>,
    pub schedule_config: ScheduleConfig,
    pub manual_config: ManualConfig,
}

/// 自动触发配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTriggerConfig {
    pub name: String,
    pub enabled: bool,
    pub strategy: OptimizationStrategy,
    pub thresholds: TriggerThresholds,
    pub filters: Option<OptimizationFilters>,
}

/// 触发阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerThresholds {
    pub max_memory_count: usize,
    pub max_storage_size_mb: usize,
    pub duplicate_ratio_threshold: f32,
    pub search_latency_ms: u64,
    pub access_frequency_threshold: f32,
}

/// 定时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub default_cron: String,
    pub time_zone: String,
}

/// 手动配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualConfig {
    pub confirm_required: bool,
    pub preview_enabled: bool,
}

/// 策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfigs {
    pub deduplication: DeduplicationConfig,
    pub relevance: RelevanceConfig,
    pub quality: QualityConfig,
    pub space: SpaceConfig,
}

/// 去重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    pub semantic_threshold: f32,
    pub content_threshold: f32,
    pub metadata_threshold: f32,
    pub merge_threshold: f32,
    pub max_batch_size: usize,
}

/// 相关性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceConfig {
    pub time_decay_days: u32,
    pub min_access_frequency: f32,
    pub importance_threshold: f32,
}

/// 质量配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    pub min_content_length: usize,
    pub quality_score_threshold: f32,
}

/// 空间配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceConfig {
    pub max_memory_per_type: usize,
    pub archive_after_days: u32,
}

/// 执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub batch_size: usize,
    pub max_concurrent_tasks: usize,
    pub timeout_minutes: u64,
    pub retry_attempts: u32,
    pub progress_callback: Option<String>,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub auto_backup: bool,
    pub backup_retention_days: u32,
    pub max_optimization_duration_hours: u32,
}

impl Default for OptimizationRequest {
    fn default() -> Self {
        Self {
            optimization_id: None,
            strategy: OptimizationStrategy::Full,
            filters: Default::default(),
            aggressive: false,
            dry_run: false,
            timeout_minutes: Some(30),
        }
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            auto_optimize: true,
            trigger_config: TriggerConfig {
                auto_triggers: vec![AutoTriggerConfig {
                    name: "weekly_full_optimize".to_string(),
                    enabled: true,
                    strategy: OptimizationStrategy::Full,
                    thresholds: TriggerThresholds {
                        max_memory_count: 10000,
                        max_storage_size_mb: 1024,
                        duplicate_ratio_threshold: 0.2,
                        search_latency_ms: 1000,
                        access_frequency_threshold: 0.1,
                    },
                    filters: None,
                }],
                schedule_config: ScheduleConfig {
                    default_cron: "0 2 * * 0".to_string(),
                    time_zone: "UTC".to_string(),
                },
                manual_config: ManualConfig {
                    confirm_required: true,
                    preview_enabled: true,
                },
            },
            strategy_configs: StrategyConfigs {
                deduplication: DeduplicationConfig {
                    semantic_threshold: 0.85,
                    content_threshold: 0.7,
                    metadata_threshold: 0.8,
                    merge_threshold: 0.9,
                    max_batch_size: 1000,
                },
                relevance: RelevanceConfig {
                    time_decay_days: 30,
                    min_access_frequency: 0.05,
                    importance_threshold: 0.3,
                },
                quality: QualityConfig {
                    min_content_length: 10,
                    quality_score_threshold: 0.4,
                },
                space: SpaceConfig {
                    max_memory_per_type: 5000,
                    archive_after_days: 90,
                },
            },
            execution_config: ExecutionConfig {
                batch_size: 100,
                max_concurrent_tasks: 4,
                timeout_minutes: 30,
                retry_attempts: 3,
                progress_callback: None,
            },
            safety_config: SafetyConfig {
                auto_backup: true,
                backup_retention_days: 7,
                max_optimization_duration_hours: 2,
            },
        }
    }
}