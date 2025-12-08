use crate::error::Result;
use crate::types::OptimizationResult;

/// 优化结果报告器
pub struct ResultReporter {
    config: ResultReporterConfig,
}

#[derive(Debug, Clone)]
pub struct ResultReporterConfig {
    pub enable_detailed_logging: bool,
    pub enable_metrics_collection: bool,
    pub log_file_path: Option<String>,
}

impl Default for ResultReporterConfig {
    fn default() -> Self {
        Self {
            enable_detailed_logging: true,
            enable_metrics_collection: true,
            log_file_path: None,
        }
    }
}

impl ResultReporter {
    pub fn new() -> Self {
        Self {
            config: ResultReporterConfig::default(),
        }
    }
    
    pub fn with_config(config: ResultReporterConfig) -> Self {
        Self { config }
    }
    
    /// 报告优化结果
    pub async fn report_optimization_result(&self, result: &OptimizationResult) -> Result<()> {
        tracing::info!("=== 优化结果报告 ===");
        tracing::info!("优化ID: {}", result.optimization_id);
        tracing::info!("策略: {:?}", result.strategy);
        tracing::info!("开始时间: {}", result.start_time);
        tracing::info!("结束时间: {}", result.end_time);
        tracing::info!("执行时长: {:?}", result.end_time - result.start_time);
        tracing::info!("发现问题: {} 个", result.issues_found.len());
        tracing::info!("执行操作: {} 个", result.actions_performed.len());
        tracing::info!("是否成功: {}", result.success);
        
        if let Some(ref error) = result.error_message {
            tracing::error!("错误信息: {}", error);
        }
        
        if let Some(ref metrics) = result.metrics {
            self.report_metrics(metrics).await?;
        }
        
        self.log_detailed_results(result).await?;
        self.generate_summary_report(result).await?;
        
        Ok(())
    }
    
    /// 报告优化指标
    async fn report_metrics(&self, metrics: &crate::types::OptimizationMetrics) -> Result<()> {
        tracing::info!("=== 优化指标 ===");
        tracing::info!("总优化次数: {}", metrics.total_optimizations);
        if let Some(last_time) = metrics.last_optimization {
            tracing::info!("上次优化时间: {}", last_time);
        }
        tracing::info!("优化前记忆数量: {}", metrics.memory_count_before);
        tracing::info!("优化后记忆数量: {}", metrics.memory_count_after);
        tracing::info!("节省空间: {:.2} MB", metrics.saved_space_mb);
        tracing::info!("去重率: {:.2}%", metrics.deduplication_rate * 100.0);
        tracing::info!("质量改善: {:.2}%", metrics.quality_improvement * 100.0);
        tracing::info!("性能改善: {:.2}%", metrics.performance_improvement * 100.0);
        
        Ok(())
    }
    
    /// 记录详细结果
    async fn log_detailed_results(&self, result: &OptimizationResult) -> Result<()> {
        if !self.config.enable_detailed_logging {
            return Ok(());
        }
        
        // 记录问题详情
        for (index, issue) in result.issues_found.iter().enumerate() {
            tracing::info!("问题 {}: {:?}", index + 1, issue);
        }
        
        // 记录操作详情
        for (index, action) in result.actions_performed.iter().enumerate() {
            tracing::info!("操作 {}: {:?}", index + 1, action);
        }
        
        Ok(())
    }
    
    /// 生成摘要报告
    async fn generate_summary_report(&self, result: &OptimizationResult) -> Result<()> {
        let report = self.create_summary_text(result);
        
        tracing::info!("=== 优化摘要报告 ===");
        tracing::info!("{}", report);
        
        // 如果配置了日志文件路径，写入文件
        if let Some(ref log_path) = self.config.log_file_path {
            if let Err(e) = tokio::fs::write(log_path, report).await {
                tracing::warn!("写入报告文件失败: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 创建摘要文本
    fn create_summary_text(&self, result: &OptimizationResult) -> String {
        let mut summary = String::new();
        
        summary.push_str(&format!("优化执行摘要\n"));
        summary.push_str(&format!("==================\n\n"));
        summary.push_str(&format!("优化ID: {}\n", result.optimization_id));
        summary.push_str(&format!("执行策略: {:?}\n", result.strategy));
        summary.push_str(&format!("执行时间: {}\n", result.start_time));
        summary.push_str(&format!("完成时间: {}\n", result.end_time));
        summary.push_str(&format!("总耗时: {:?}\n\n", result.end_time - result.start_time));
        
        // 统计信息
        summary.push_str(&format!("执行统计:\n"));
        summary.push_str(&format!("- 发现问题: {} 个\n", result.issues_found.len()));
        summary.push_str(&format!("- 执行操作: {} 个\n", result.actions_performed.len()));
        
        if let Some(metrics) = &result.metrics {
            summary.push_str(&format!("- 节省空间: {:.2} MB\n", metrics.saved_space_mb));
            summary.push_str(&format!("- 去重率: {:.1}%\n", metrics.deduplication_rate * 100.0));
        }
        
        // 操作分类统计
        let mut action_stats = ActionStatistics::default();
        for action in &result.actions_performed {
            match action {
                crate::types::OptimizationAction::Merge { .. } => action_stats.merge_count += 1,
                crate::types::OptimizationAction::Delete { .. } => action_stats.delete_count += 1,
                crate::types::OptimizationAction::Update { .. } => action_stats.update_count += 1,
                crate::types::OptimizationAction::Reclassify { .. } => action_stats.reclassify_count += 1,
                crate::types::OptimizationAction::Archive { .. } => action_stats.archive_count += 1,
            }
        }
        
        summary.push_str(&format!("\n操作类型分布:\n"));
        summary.push_str(&format!("- 合并操作: {} 个\n", action_stats.merge_count));
        summary.push_str(&format!("- 删除操作: {} 个\n", action_stats.delete_count));
        summary.push_str(&format!("- 更新操作: {} 个\n", action_stats.update_count));
        summary.push_str(&format!("- 重分类操作: {} 个\n", action_stats.reclassify_count));
        summary.push_str(&format!("- 归档操作: {} 个\n", action_stats.archive_count));
        
        // 问题分类统计
        let mut issue_stats = IssueStatistics::default();
        for issue in &result.issues_found {
            match issue.severity {
                crate::types::IssueSeverity::Low => issue_stats.low_count += 1,
                crate::types::IssueSeverity::Medium => issue_stats.medium_count += 1,
                crate::types::IssueSeverity::High => issue_stats.high_count += 1,
                crate::types::IssueSeverity::Critical => issue_stats.critical_count += 1,
            }
        }
        
        summary.push_str(&format!("\n问题严重程度分布:\n"));
        summary.push_str(&format!("- 低严重程度: {} 个\n", issue_stats.low_count));
        summary.push_str(&format!("- 中等严重程度: {} 个\n", issue_stats.medium_count));
        summary.push_str(&format!("- 高严重程度: {} 个\n", issue_stats.high_count));
        summary.push_str(&format!("- 严重程度: {} 个\n", issue_stats.critical_count));
        
        // 结果状态
        if result.success {
            summary.push_str(&format!("\n✅ 优化执行成功\n"));
        } else {
            summary.push_str(&format!("\n❌ 优化执行失败\n"));
            if let Some(ref error) = result.error_message {
                summary.push_str(&format!("错误信息: {}\n", error));
            }
        }
        
        summary
    }
    
    /// 生成结构化报告（JSON）
    pub async fn generate_structured_report(&self, result: &OptimizationResult) -> Result<String> {
        let report_data = serde_json::to_string_pretty(result)?;
        Ok(report_data)
    }
    
    /// 导出报告到文件
    pub async fn export_report(
        &self,
        result: &OptimizationResult,
        file_path: &str,
        format: ReportFormat,
    ) -> Result<()> {
        let content = match format {
            ReportFormat::Text => self.create_summary_text(result),
            ReportFormat::Json => self.generate_structured_report(result).await?,
            ReportFormat::Yaml => {
                // 简化YAML导出，使用JSON格式代替
                self.generate_structured_report(result).await?
            }
        };
        
        if let Err(e) = tokio::fs::write(file_path, content).await {
            tracing::warn!("写入报告文件失败: {}", e);
        } else {
            tracing::info!("报告已导出到: {}", file_path);
        }
        
        Ok(())
    }
}

/// 报告格式
#[derive(Debug, Clone)]
pub enum ReportFormat {
    Text,
    Json,
    Yaml,
}

/// 操作统计
#[derive(Debug, Clone, Default)]
struct ActionStatistics {
    pub merge_count: usize,
    pub delete_count: usize,
    pub update_count: usize,
    pub reclassify_count: usize,
    pub archive_count: usize,
}

/// 问题统计
#[derive(Debug, Clone, Default)]
struct IssueStatistics {
    pub low_count: usize,
    pub medium_count: usize,
    pub high_count: usize,
    pub critical_count: usize,
}

impl Default for ResultReporter {
    fn default() -> Self {
        Self::new()
    }
}