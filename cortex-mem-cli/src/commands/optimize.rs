use clap::Parser;
use cortex_mem_core::{
    memory::{MemoryManager, DefaultMemoryOptimizer},
    config::Config,
};
use std::sync::Arc;


/// 优化命令
#[derive(Parser)]
pub struct OptimizeCommand {
    /// 优化策略
    #[arg(long, default_value = "full")]
    pub strategy: String,
    
    /// 用户ID过滤
    #[arg(long)]
    pub user_id: Option<String>,
    
    /// Agent ID过滤
    #[arg(long)]
    pub agent_id: Option<String>,
    
    /// 记忆类型过滤
    #[arg(long)]
    pub memory_type: Option<String>,
    
    /// 预览模式（不执行）
    #[arg(long)]
    pub preview: bool,
    
    /// 激进模式（更深层优化）
    #[arg(long)]
    pub aggressive: bool,
    
    /// 跳过确认
    #[arg(long)]
    pub no_confirm: bool,
    
    /// 超时时间（分钟）
    #[arg(long, default_value = "30")]
    pub timeout: u64,
}

/// 优化状态命令
#[derive(Parser)]
pub struct OptimizationStatusCommand {
    /// 显示详细指标
    #[arg(long)]
    pub detailed: bool,
    
    /// 显示历史记录
    #[arg(long)]
    pub history: bool,
}

/// 优化配置命令
#[derive(Parser)]
pub struct OptimizationConfigCommand {
    /// 显示当前配置
    #[arg(long)]
    pub show: bool,
    
    /// 更新配置
    #[arg(long)]
    pub update: bool,
    
    /// 配置文件路径
    #[arg(conflicts_with = "show")]
    pub config_file: Option<String>,
}

/// 优化命令执行器
pub struct OptimizeCommandRunner {
    memory_manager: Arc<MemoryManager>,
    config: Config,
}

impl OptimizeCommandRunner {
    pub fn new(memory_manager: Arc<MemoryManager>, config: Config) -> Self {
        Self {
            memory_manager,
            config,
        }
    }
    
    pub async fn run_optimize(&self, cmd: &OptimizeCommand) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 构建优化请求
        let request = self.build_optimization_request(cmd)?;
        
        // 2. 创建优化器
        let optimizer = self.create_optimizer().await?;
        
        // 3. 执行优化
        if cmd.preview {
            self.run_preview(optimizer.as_ref(), &request).await?;
        } else {
            self.run_optimization(optimizer.as_ref(), &request, cmd.no_confirm).await?;
        }
        
        Ok(())
    }
    
    async fn create_optimizer(&self) -> Result<Arc<dyn cortex_mem_core::memory::MemoryOptimizer>, Box<dyn std::error::Error>> {
        // 使用默认的优化配置
        let optimization_config = cortex_mem_core::types::OptimizationConfig::default();
        
        let optimizer = DefaultMemoryOptimizer::new(
            self.memory_manager.clone(),
            optimization_config,
        );
        
        Ok(Arc::new(optimizer))
    }
    
    async fn run_preview(&self, optimizer: &dyn cortex_mem_core::memory::MemoryOptimizer, request: &cortex_mem_core::types::OptimizationRequest) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 优化计划预览");
        println!("策略: {:?}", request.strategy);
        println!("过滤器: {:?}", request.filters);
        println!();
        
        let plan = optimizer.create_optimization_plan(request.strategy.clone()).await?;
        
        println!("📋 检测到的问题:");
        for (i, issue) in plan.issues.iter().enumerate() {
            println!("  {}. {:?} - {}", i + 1, issue.severity, issue.description);
        }
        
        println!();
        println!("🎯 建议的操作:");
        for (i, action) in plan.actions.iter().enumerate() {
            println!("  {}. {:?}", i + 1, action);
        }
        
        Ok(())
    }
    
    async fn run_optimization(&self, optimizer: &dyn cortex_mem_core::memory::MemoryOptimizer, request: &cortex_mem_core::types::OptimizationRequest, no_confirm: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !no_confirm {
            println!("⚠️  此操作将修改您的memory数据库");
            let input = prompt_for_confirmation("是否继续? (y/N): ");
            if !input {
                println!("❌ 操作已取消");
                return Ok(());
            }
        }
        
        println!("🚀 开始执行优化...");
        
        let result = optimizer.optimize(request).await?;
        
        if result.success {
            println!("✅ 优化完成!");
            println!("📊 优化统计:");
            println!("  - 执行时间: {:?}", result.end_time - result.start_time);
            println!("  - 发现问题: {} 个", result.issues_found.len());
            println!("  - 执行操作: {} 个", result.actions_performed.len());
            
            if let Some(metrics) = result.metrics {
                println!("  - 节省空间: {:.2} MB", metrics.saved_space_mb);
                println!("  - 改善质量: {:.2}%", metrics.quality_improvement * 100.0);
            }
        } else {
            println!("❌ 优化失败: {}", result.error_message.unwrap_or_else(|| "未知错误".to_string()));
        }
        
        Ok(())
    }
    
    pub async fn run_status(&self, cmd: &OptimizationStatusCommand) -> Result<(), Box<dyn std::error::Error>> {
        println!("📈 优化状态");
        
        if cmd.detailed {
            println!("详细指标功能开发中...");
        }
        
        if cmd.history {
            println!("历史记录功能开发中...");
        }
        
        Ok(())
    }
    
    pub async fn run_config(&self, cmd: &OptimizationConfigCommand) -> Result<(), Box<dyn std::error::Error>> {
        if cmd.show {
            println!("优化配置:");
            println!("当前配置功能开发中...");
        } else if cmd.update {
            println!("更新配置功能开发中...");
        }
        
        Ok(())
    }
    
    fn build_optimization_request(&self, cmd: &OptimizeCommand) -> Result<cortex_mem_core::types::OptimizationRequest, Box<dyn std::error::Error>> {
        let memory_type = cmd.memory_type.as_ref()
            .map(|s| cortex_mem_core::types::MemoryType::parse(s));
            
        let strategy = match cmd.strategy.to_lowercase().as_str() {
            "full" => cortex_mem_core::types::OptimizationStrategy::Full,
            "incremental" => cortex_mem_core::types::OptimizationStrategy::Incremental,
            "batch" => cortex_mem_core::types::OptimizationStrategy::Batch,
            "deduplication" => cortex_mem_core::types::OptimizationStrategy::Deduplication,
            "relevance" => cortex_mem_core::types::OptimizationStrategy::Relevance,
            "quality" => cortex_mem_core::types::OptimizationStrategy::Quality,
            "space" => cortex_mem_core::types::OptimizationStrategy::Space,
            _ => cortex_mem_core::types::OptimizationStrategy::Full,
        };
            
        let filters = cortex_mem_core::types::OptimizationFilters {
            user_id: cmd.user_id.clone(),
            agent_id: cmd.agent_id.clone(),
            memory_type,
            date_range: None,
            importance_range: None,
            custom_filters: std::collections::HashMap::new(),
        };
        
        Ok(cortex_mem_core::types::OptimizationRequest {
            optimization_id: None,
            strategy,
            filters,
            aggressive: cmd.aggressive,
            dry_run: cmd.preview,
            timeout_minutes: Some(cmd.timeout),
        })
    }
}

fn prompt_for_confirmation(prompt: &str) -> bool {
    use std::io::{self, Write};
    
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or_default();
    
    input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes"
}