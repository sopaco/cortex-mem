use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::types::{
    OptimizationAction, OptimizationFilters, OptimizationIssue, OptimizationStrategy,
};

/// 优化计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPlan {
    pub optimization_id: String,
    pub strategy: OptimizationStrategy,
    pub created_at: chrono::DateTime<Utc>,
    pub estimated_duration_minutes: u64,
    pub issues: Vec<OptimizationIssue>,
    pub actions: Vec<OptimizationAction>,
    pub filters: OptimizationFilters,
}

impl OptimizationPlan {
    /// 创建新的优化计划
    pub fn new(
        optimization_id: String,
        strategy: OptimizationStrategy,
        issues: Vec<OptimizationIssue>,
        actions: Vec<OptimizationAction>,
        filters: OptimizationFilters,
    ) -> Self {
        let estimated_duration_minutes = Self::estimate_duration(&strategy, &issues);
        
        Self {
            optimization_id,
            strategy,
            created_at: Utc::now(),
            estimated_duration_minutes,
            issues,
            actions,
            filters,
        }
    }
    
    /// 估算优化执行时间
    fn estimate_duration(strategy: &OptimizationStrategy, issues: &[OptimizationIssue]) -> u64 {
        let base_time = match strategy {
            OptimizationStrategy::Full => 60,        // 60分钟
            OptimizationStrategy::Incremental => 15, // 15分钟
            OptimizationStrategy::Batch => 45,       // 45分钟
            OptimizationStrategy::Deduplication => 20,
            OptimizationStrategy::Relevance => 25,
            OptimizationStrategy::Quality => 30,
            OptimizationStrategy::Space => 35,
        };
        
        // 根据问题数量调整时间
        let issue_factor = (issues.len() as f64 / 100.0).ceil() as u64;
        base_time + issue_factor * 5
    }
    
    /// 获取计划摘要
    pub fn summary(&self) -> String {
        let mut summary = format!(
            "优化策略: {:?}\n预计时间: {} 分钟\n发现问题: {} 个\n建议操作: {} 个",
            self.strategy,
            self.estimated_duration_minutes,
            self.issues.len(),
            self.actions.len()
        );
        
        if !self.filters.user_id.is_none() || !self.filters.agent_id.is_none() {
            summary.push_str(&format!("\n过滤条件: {:?}", self.filters));
        }
        
        summary
    }
    
    /// 获取按类型分组的操作统计
    pub fn action_statistics(&self) -> ActionStatistics {
        let mut stats = ActionStatistics::default();
        
        for action in &self.actions {
            match action {
                OptimizationAction::Merge { .. } => stats.merge_count += 1,
                OptimizationAction::Delete { .. } => stats.delete_count += 1,
                OptimizationAction::Update { .. } => stats.update_count += 1,
                OptimizationAction::Reclassify { .. } => stats.reclassify_count += 1,
                OptimizationAction::Archive { .. } => stats.archive_count += 1,
            }
        }
        
        stats
    }
    
    /// 获取按严重程度分组的统计数据
    pub fn issue_statistics(&self) -> IssueStatistics {
        let mut stats = IssueStatistics::default();
        
        for issue in &self.issues {
            match issue.severity {
                crate::types::IssueSeverity::Low => stats.low_count += 1,
                crate::types::IssueSeverity::Medium => stats.medium_count += 1,
                crate::types::IssueSeverity::High => stats.high_count += 1,
                crate::types::IssueSeverity::Critical => stats.critical_count += 1,
            }
            
            match issue.kind {
                crate::types::IssueKind::Duplicate => stats.duplicate_issues += 1,
                crate::types::IssueKind::LowQuality => stats.quality_issues += 1,
                crate::types::IssueKind::Outdated => stats.relevance_issues += 1,
                crate::types::IssueKind::PoorClassification => stats.classification_issues += 1,
                crate::types::IssueKind::SpaceInefficient => stats.space_issues += 1,
            }
        }
        
        stats
    }
}

/// 操作统计
#[derive(Debug, Clone, Default)]
pub struct ActionStatistics {
    pub merge_count: usize,
    pub delete_count: usize,
    pub update_count: usize,
    pub reclassify_count: usize,
    pub archive_count: usize,
}

impl ActionStatistics {
    pub fn total(&self) -> usize {
        self.merge_count + self.delete_count + self.update_count 
            + self.reclassify_count + self.archive_count
    }
}

/// 问题统计
#[derive(Debug, Clone, Default)]
pub struct IssueStatistics {
    pub low_count: usize,
    pub medium_count: usize,
    pub high_count: usize,
    pub critical_count: usize,
    pub duplicate_issues: usize,
    pub quality_issues: usize,
    pub relevance_issues: usize,
    pub classification_issues: usize,
    pub space_issues: usize,
}

impl IssueStatistics {
    pub fn total(&self) -> usize {
        self.low_count + self.medium_count + self.high_count + self.critical_count
    }
    
    pub fn critical_or_high(&self) -> usize {
        self.high_count + self.critical_count
    }
}
