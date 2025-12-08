use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::Result,
    memory::MemoryManager,
    types::{
        OptimizationConfig, OptimizationRequest, OptimizationResult, OptimizationStrategy,
        OptimizationStatus, OptimizationStatusType,
    },
};

use super::{
    optimization_analyzer::OptimizationAnalyzer,
    optimization_detector::OptimizationDetector,
    execution_engine::ExecutionEngine,
    result_reporter::ResultReporter,
};

/// 主动内存优化器 - 核心协调组件
#[async_trait]
pub trait MemoryOptimizer: Send + Sync {
    /// 执行优化操作
    async fn optimize(&self, request: &OptimizationRequest) -> Result<OptimizationResult>;
    
    /// 创建优化计划（预览模式）
    async fn create_optimization_plan(&self, strategy: OptimizationStrategy) -> Result<super::optimization_plan::OptimizationPlan>;
    
    /// 获取优化状态
    async fn get_optimization_status(&self) -> Result<Vec<OptimizationStatus>>;
    
    /// 取消正在进行的优化
    async fn cancel_optimization(&self, optimization_id: &str) -> Result<()>;
}

/// MemoryOptimizer 实现
pub struct DefaultMemoryOptimizer {
    #[allow(dead_code)]
    memory_manager: Arc<MemoryManager>,
    #[allow(dead_code)]
    config: OptimizationConfig,
    detector: Arc<OptimizationDetector>,
    analyzer: Arc<OptimizationAnalyzer>,
    executor: Arc<ExecutionEngine>,
    reporter: Arc<ResultReporter>,
    running_optimizations: tokio::sync::RwLock<std::collections::HashMap<String, OptimizationStatus>>,
}

impl DefaultMemoryOptimizer {
    pub fn new(
        memory_manager: Arc<MemoryManager>,
        config: OptimizationConfig,
    ) -> Self {
        let memory_manager_detector = memory_manager.clone();
        let memory_manager_analyzer = memory_manager.clone();
        let memory_manager_executor = memory_manager.clone();
        
        Self {
            memory_manager,
            config,
            detector: Arc::new(OptimizationDetector::with_memory_manager(memory_manager_detector)),
            analyzer: Arc::new(OptimizationAnalyzer::with_memory_manager(memory_manager_analyzer)),
            executor: Arc::new(ExecutionEngine::with_memory_manager(memory_manager_executor)),
            reporter: Arc::new(ResultReporter::new()),
            running_optimizations: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl MemoryOptimizer for DefaultMemoryOptimizer {
    async fn optimize(&self, request: &OptimizationRequest) -> Result<OptimizationResult> {
        let optimization_id = request.optimization_id.clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // 初始化优化状态
        let mut status = OptimizationStatus {
            optimization_id: optimization_id.clone(),
            status: OptimizationStatusType::Running,
            progress: 0,
            current_phase: "初始化".to_string(),
            started_at: Some(Utc::now()),
            estimated_completion: None,
        };
        
        // 记录正在运行的优化
        {
            let mut running = self.running_optimizations.write().await;
            running.insert(optimization_id.clone(), status.clone());
        }
        
        let start_time = Utc::now();
        
        tracing::info!(optimization_id = optimization_id, "开始执行内存优化");
        
        // 1. 检测问题 (20%)
        {
            status.progress = 20;
            status.current_phase = "检测问题".to_string();
            self.update_optimization_status(&optimization_id, &status).await;
            tracing::info!("开始检测内存优化问题");
        }
        
        let issues = self.detector.detect_issues(&request.filters).await?;
        
        // 2. 分析制定计划 (40%)
        {
            status.progress = 40;
            status.current_phase = "制定优化计划".to_string();
            self.update_optimization_status(&optimization_id, &status).await;
            tracing::info!("制定优化计划");
        }
        
        let plan = self.analyzer.create_optimization_plan(&issues, &request.strategy, &request.filters).await?;
        
        // 3. 执行优化 (80%)
        {
            status.progress = 80;
            status.current_phase = "执行优化".to_string();
            self.update_optimization_status(&optimization_id, &status).await;
            tracing::info!("执行优化计划");
        }
        
        let result = if request.dry_run {
            // 干运行模式 - 不实际执行优化
            self.create_dry_run_result(&optimization_id, request, start_time, plan)
        } else {
            self.executor.execute_plan(&optimization_id, plan).await?
        };
        
        // 4. 报告结果 (100%)
        {
            status.progress = 100;
            status.current_phase = "完成".to_string();
            status.status = OptimizationStatusType::Completed;
            self.update_optimization_status(&optimization_id, &status).await;
            
            self.reporter.report_optimization_result(&result).await?;
        }
        
        // 从运行中优化列表中移除
        {
            let mut running = self.running_optimizations.write().await;
            running.remove(&optimization_id);
        }
        
        tracing::info!(optimization_id = optimization_id, "优化完成: {} 项操作", result.actions_performed.len());
        Ok(result)
    }
    
    async fn create_optimization_plan(&self, strategy: OptimizationStrategy) -> Result<super::optimization_plan::OptimizationPlan> {
        let issues = self.detector.detect_issues(&Default::default()).await?;
        self.analyzer.create_optimization_plan(&issues, &strategy, &Default::default()).await
    }
    
    async fn get_optimization_status(&self) -> Result<Vec<OptimizationStatus>> {
        let running = self.running_optimizations.read().await;
        let statuses = running.values().cloned().collect::<Vec<_>>();
        
        // 这里可以从历史记录中读取已完成的优化状态
        // 暂时只返回正在运行的优化状态
        
        Ok(statuses)
    }
    
    async fn cancel_optimization(&self, optimization_id: &str) -> Result<()> {
        let mut running = self.running_optimizations.write().await;
        
        if let Some(status) = running.get_mut(optimization_id) {
            status.status = OptimizationStatusType::Cancelled;
        }
        
        // 这里应该发送取消信号给执行引擎
        // 暂时只是更新状态
        
        tracing::info!("优化任务已取消: {}", optimization_id);
        Ok(())
    }
}

impl DefaultMemoryOptimizer {
    /// 创建干运行结果
    fn create_dry_run_result(
        &self,
        optimization_id: &str,
        request: &OptimizationRequest,
        start_time: chrono::DateTime<Utc>,
        plan: super::optimization_plan::OptimizationPlan,
    ) -> OptimizationResult {
        let end_time = Utc::now();
        
        OptimizationResult {
            optimization_id: optimization_id.to_string(),
            strategy: request.strategy.clone(),
            start_time,
            end_time,
            issues_found: plan.issues,
            actions_performed: plan.actions,
            metrics: None,
            success: true,
            error_message: None,
        }
    }
    
    /// 更新优化状态
    async fn update_optimization_status(
        &self,
        optimization_id: &str,
        status: &OptimizationStatus,
    ) {
        let mut running = self.running_optimizations.write().await;
        running.insert(optimization_id.to_string(), status.clone());
    }
}

impl DefaultMemoryOptimizer {
    /// 创建新的MemoryOptimizer实例
    pub async fn create(
        memory_manager: Arc<MemoryManager>,
        config: OptimizationConfig,
    ) -> Result<Box<dyn MemoryOptimizer>> {
        Ok(Box::new(Self::new(memory_manager, config)))
    }
}