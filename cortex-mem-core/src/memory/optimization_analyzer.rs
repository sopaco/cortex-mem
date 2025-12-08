use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::Result,
    types::{
        OptimizationAction, OptimizationFilters, OptimizationIssue, OptimizationStrategy,
        IssueKind, IssueSeverity,
    },
    memory::MemoryManager,
};

use super::optimization_plan::{OptimizationPlan, ActionStatistics};

/// 优化分析器 - 负责分析问题并制定优化策略
pub struct OptimizationAnalyzer {
    // 分析器配置
    config: OptimizationAnalyzerConfig,
    #[allow(dead_code)]
    memory_manager: Arc<MemoryManager>,
}

#[derive(Debug, Clone)]
pub struct OptimizationAnalyzerConfig {
    pub max_actions_per_plan: usize,
    pub conservative_mode: bool,
}

impl Default for OptimizationAnalyzerConfig {
    fn default() -> Self {
        Self {
            max_actions_per_plan: 5000,
            conservative_mode: false,
        }
    }
}

impl OptimizationAnalyzer {
    pub fn new() -> Self {
        panic!("OptimizationAnalyzer requires MemoryManager. Use with_memory_manager() instead.");
    }
    
    pub fn with_memory_manager(memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            config: OptimizationAnalyzerConfig::default(),
            memory_manager,
        }
    }
    
    pub fn with_config(config: OptimizationAnalyzerConfig, memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            config,
            memory_manager,
        }
    }
    
    /// 根据问题制定优化计划
    pub async fn create_optimization_plan(
        &self,
        issues: &[OptimizationIssue],
        strategy: &OptimizationStrategy,
        filters: &OptimizationFilters,
    ) -> Result<OptimizationPlan> {
        let optimization_id = Uuid::new_v4().to_string();
        
        tracing::info!(optimization_id = optimization_id, "制定优化计划, 策略: {:?}, 问题数量: {}", strategy, issues.len());
        
        let actions = self.generate_optimization_actions(issues, strategy).await?;
        
        let plan = OptimizationPlan::new(
            optimization_id,
            strategy.clone(),
            issues.to_vec(),
            actions,
            filters.clone(),
        );
        
        tracing::info!(optimization_id = plan.optimization_id, "计划制定完成: {} 个操作", plan.actions.len());
        Ok(plan)
    }
    
    /// 生成优化操作
    async fn generate_optimization_actions(
        &self,
        issues: &[OptimizationIssue],
        strategy: &OptimizationStrategy,
    ) -> Result<Vec<OptimizationAction>> {
        let mut actions = Vec::new();
        
        // 根据策略过滤相关问题
        let relevant_issues = self.filter_issues_by_strategy(issues, strategy);
        
        tracing::info!("策略 {:?} 相关的 {} 个问题", strategy, relevant_issues.len());
        
        for issue in relevant_issues {
            let issue_actions = self.analyze_issue_and_generate_actions(&issue).await?;
            actions.extend(issue_actions);
            
            // 限制操作数量以防止计划过大
            if actions.len() >= self.config.max_actions_per_plan {
                tracing::warn!("达到最大操作数量限制: {}", self.config.max_actions_per_plan);
                break;
            }
        }
        
        // 如果是保守模式，进一步过滤操作
        if self.config.conservative_mode {
            actions = self.filter_actions_conservatively(actions);
        }
        
        Ok(actions)
    }
    
    /// 根据策略过滤相关问题
    fn filter_issues_by_strategy<'a>(
        &'a self,
        issues: &'a [OptimizationIssue],
        strategy: &'a OptimizationStrategy,
    ) -> Vec<&'a OptimizationIssue> {
        match strategy {
            OptimizationStrategy::Full => issues.iter().collect(),
            OptimizationStrategy::Incremental => {
                // 只处理高严重程度的问题
                issues.iter()
                    .filter(|issue| {
                        matches!(issue.severity, IssueSeverity::High | IssueSeverity::Critical)
                    })
                    .collect()
            }
            OptimizationStrategy::Batch => {
                // 处理所有Medium及以上的问题
                issues.iter()
                    .filter(|issue| {
                        !matches!(issue.severity, IssueSeverity::Low)
                    })
                    .collect()
            }
            OptimizationStrategy::Deduplication => {
                issues.iter()
                    .filter(|issue| matches!(issue.kind, IssueKind::Duplicate))
                    .collect()
            }
            OptimizationStrategy::Relevance => {
                issues.iter()
                    .filter(|issue| matches!(issue.kind, IssueKind::Outdated))
                    .collect()
            }
            OptimizationStrategy::Quality => {
                issues.iter()
                    .filter(|issue| matches!(issue.kind, IssueKind::LowQuality))
                    .collect()
            }
            OptimizationStrategy::Space => {
                issues.iter()
                    .filter(|issue| matches!(issue.kind, IssueKind::SpaceInefficient))
                    .collect()
            }
        }
    }
    
    /// 分析单个问题并生成相应的操作
    async fn analyze_issue_and_generate_actions(
        &self,
        issue: &OptimizationIssue,
    ) -> Result<Vec<OptimizationAction>> {
        let mut actions = Vec::new();
        
        match issue.kind {
            IssueKind::Duplicate => {
                if issue.affected_memories.len() > 1 {
                    actions.push(OptimizationAction::Merge {
                        memories: issue.affected_memories.clone(),
                    });
                }
            }
            IssueKind::LowQuality => {
                // 为每个低质量记忆生成操作
                for memory_id in &issue.affected_memories {
                    // 对于质量极低的记忆，建议删除
                    // 对于中等质量的问题，建议更新重要性分数
                    actions.push(OptimizationAction::Delete {
                        memory_id: memory_id.clone(),
                    });
                }
            }
            IssueKind::Outdated => {
                // 过时记忆可能需要删除或归档
                for memory_id in &issue.affected_memories {
                    if issue.severity == IssueSeverity::Critical {
                        actions.push(OptimizationAction::Delete {
                            memory_id: memory_id.clone(),
                        });
                    } else {
                        actions.push(OptimizationAction::Archive {
                            memory_id: memory_id.clone(),
                        });
                    }
                }
            }
            IssueKind::PoorClassification => {
                // 重新分类记忆
                for memory_id in &issue.affected_memories {
                    actions.push(OptimizationAction::Reclassify {
                        memory_id: memory_id.clone(),
                    });
                }
            }
            IssueKind::SpaceInefficient => {
                // 空间效率问题一般通过归档处理
                for memory_id in &issue.affected_memories {
                    actions.push(OptimizationAction::Archive {
                        memory_id: memory_id.clone(),
                    });
                }
            }
        }
        
        Ok(actions)
    }
    
    /// 保守模式过滤操作
    fn filter_actions_conservatively(&self, actions: Vec<OptimizationAction>) -> Vec<OptimizationAction> {
        let mut filtered = Vec::new();
        
        for action in actions {
            match action {
                // 在保守模式下，避免删除操作
                OptimizationAction::Delete { .. } => {
                    tracing::info!("保守模式: 跳过删除操作");
                    continue;
                }
                // 将删除操作转换为归档操作
                OptimizationAction::Archive { .. } => {
                    // 保留归档操作
                    filtered.push(action);
                }
                _ => {
                    // 保留其他操作
                    filtered.push(action);
                }
            }
        }
        
        filtered
    }
    
    /// 分析优化效果预测
    pub fn analyze_optimization_impact(
        &self,
        plan: &OptimizationPlan,
    ) -> Result<OptimizationImpact> {
        let stats = plan.action_statistics();
        let issue_stats = plan.issue_statistics();
        
        let mut predictions = HashMap::new();
        
        // 预测去重效果
        if stats.merge_count > 0 {
            predictions.insert("deduplication".to_string(), format!("预计合并 {} 个重复记忆", stats.merge_count));
        }
        
        // 预测空间节省
        if stats.delete_count > 0 {
            predictions.insert("space_saving".to_string(), format!("预计删除 {} 个记忆", stats.delete_count));
        }
        
        // 预测质量改善
        if stats.update_count > 0 {
            predictions.insert("quality_improvement".to_string(), format!("预计更新 {} 个记忆", stats.update_count));
        }
        
        // 预测性能提升
        let critical_issues = issue_stats.critical_or_high();
        if critical_issues > 0 {
            predictions.insert("performance_boost".to_string(), format!("预计解决 {} 个严重问题", critical_issues));
        }
        
        Ok(OptimizationImpact {
            estimated_duration_minutes: plan.estimated_duration_minutes,
            risk_level: self.calculate_risk_level(&stats),
            predictions,
            statistics: stats,
        })
    }
    
    /// 计算风险等级
    fn calculate_risk_level(&self, stats: &ActionStatistics) -> RiskLevel {
        let total_actions = stats.total();
        
        if total_actions == 0 {
            return RiskLevel::VeryLow;
        }
        
        let deletion_ratio = stats.delete_count as f64 / total_actions as f64;
        let merge_ratio = stats.merge_count as f64 / total_actions as f64;
        
        if deletion_ratio > 0.3 || merge_ratio > 0.5 {
            RiskLevel::High
        } else if deletion_ratio > 0.1 || merge_ratio > 0.3 {
            RiskLevel::Medium
        } else if deletion_ratio > 0.05 || merge_ratio > 0.1 {
            RiskLevel::Low
        } else {
            RiskLevel::VeryLow
        }
    }
}

/// 优化影响分析
#[derive(Debug, Clone)]
pub struct OptimizationImpact {
    pub estimated_duration_minutes: u64,
    pub risk_level: RiskLevel,
    pub predictions: HashMap<String, String>,
    pub statistics: ActionStatistics,
}

/// 风险等级
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::VeryLow => write!(f, "极低"),
            RiskLevel::Low => write!(f, "低"),
            RiskLevel::Medium => write!(f, "中等"),
            RiskLevel::High => write!(f, "高"),
        }
    }
}

impl Default for OptimizationAnalyzer {
    fn default() -> Self {
        panic!("OptimizationAnalyzer requires MemoryManager. Use with_memory_manager() instead.");
    }
}