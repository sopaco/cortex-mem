use clap::Parser;
use cortex_mem_core::{
    config::Config,
    memory::{DefaultMemoryOptimizer, MemoryManager},
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

    /// 显示详细内容（预览时显示记忆摘要）
    #[arg(long)]
    pub verbose: bool,

    /// 限制显示的问题数量（默认10）
    #[arg(long, default_value = "10")]
    pub limit: usize,
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

    pub async fn run_optimize(
        &self,
        cmd: &OptimizeCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 构建优化请求
        let request = self.build_optimization_request(cmd)?;

        // 2. 创建优化器
        let optimizer = self.create_optimizer().await?;

        // 3. 执行优化
        if cmd.preview {
            self.run_preview(optimizer.as_ref(), &request).await?;
        } else {
            self.run_optimization(optimizer.as_ref(), &request, cmd.no_confirm)
                .await?;
        }

        Ok(())
    }

    async fn create_optimizer(
        &self,
    ) -> Result<Arc<dyn cortex_mem_core::memory::MemoryOptimizer>, Box<dyn std::error::Error>> {
        // 使用默认的优化配置
        let optimization_config = cortex_mem_core::types::OptimizationConfig::default();

        let optimizer =
            DefaultMemoryOptimizer::new(self.memory_manager.clone(), optimization_config);

        Ok(Arc::new(optimizer))
    }

    async fn run_preview(
        &self,
        optimizer: &dyn cortex_mem_core::memory::MemoryOptimizer,
        request: &cortex_mem_core::types::OptimizationRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 优化计划预览");
        println!("策略: {:?}", request.strategy);
        println!("过滤器: {:?}", request.filters);
        println!();

        // 创建优化计划，添加错误处理
        let plan = match optimizer
            .create_optimization_plan(request.strategy.clone())
            .await
        {
            Ok(plan) => plan,
            Err(e) => {
                // 检查是否是API限制错误
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("too many requests") || error_str.contains("429") {
                    println!("⚠️  API请求频率限制，无法生成优化计划");
                    println!("💡 请稍后再试，或使用 --limit 参数减少查询数量");
                    return Ok(());
                } else {
                    return Err(Box::new(e));
                }
            }
        };

        // 检查是否是详细模式
        let verbose = request
            .filters
            .custom_filters
            .get("verbose")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 显示问题统计
        println!("📊 问题统计:");
        let issue_stats = plan.issue_statistics();
        println!("  - 总问题数: {}", issue_stats.total());
        println!(
            "  - 严重: {} 个, 高: {} 个, 中: {} 个, 低: {} 个",
            issue_stats.critical_count,
            issue_stats.high_count,
            issue_stats.medium_count,
            issue_stats.low_count
        );

        if verbose {
            println!(
                "  - 重复: {} 个, 质量: {} 个, 相关性: {} 个, 分类: {} 个, 空间: {} 个",
                issue_stats.duplicate_issues,
                issue_stats.quality_issues,
                issue_stats.relevance_issues,
                issue_stats.classification_issues,
                issue_stats.space_issues
            );
        }

        println!();
        println!("📋 检测到的问题:");

        // 获取受影响的记忆详细信息（仅在详细模式下）
        // 添加错误处理，当遇到API限制时回退到非详细模式
        let memory_details = if verbose {
            match self.get_memory_details(&plan.issues).await {
                Ok(details) => Some(details),
                Err(e) => {
                    // 检查是否是API限制错误
                    let error_str = e.to_string().to_lowercase();
                    if error_str.contains("too many requests") || error_str.contains("429") {
                        println!("⚠️  API请求频率限制，回退到非详细模式");
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            None
        };

        // 如果原本请求详细信息但失败了，更新verbose标志
        let effective_verbose = verbose && memory_details.is_some();

        // 限制显示的问题数量
        let display_issues: Vec<_> = plan
            .issues
            .iter()
            .take(
                request
                    .filters
                    .custom_filters
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize,
            )
            .collect();

        for (i, issue) in display_issues.iter().enumerate() {
            println!(
                "  {}. [{}] {}",
                i + 1,
                self.format_severity(issue.severity.clone()),
                issue.description
            );

            // 在详细模式下显示受影响的记忆信息
            if effective_verbose {
                if let Some(ref details) = memory_details {
                    for memory_id in &issue.affected_memories {
                        if let Some(memory) = details.get(memory_id) {
                            println!(
                                "     📝 记忆ID: {}...",
                                &memory_id[..std::cmp::min(8, memory_id.len())]
                            );
                            println!(
                                "     📖 内容: \"{}\"",
                                self.truncate_content(&memory.content, 50)
                            );
                            println!(
                                "     🏷️  类型: {:?}, 重要性: {:.2}, 创建: {}",
                                memory.metadata.memory_type,
                                memory.metadata.importance_score,
                                memory.created_at.format("%Y-%m-%d")
                            );
                            if memory.metadata.user_id.is_some()
                                || memory.metadata.agent_id.is_some()
                            {
                                println!(
                                    "     👤 用户: {:?}, 代理: {:?}",
                                    memory.metadata.user_id, memory.metadata.agent_id
                                );
                            }
                        } else {
                            println!(
                                "     📝 记忆ID: {}... (无法获取详细信息)",
                                &memory_id[..std::cmp::min(8, memory_id.len())]
                            );
                        }
                    }
                } else {
                    // 详细模式回退到非详细模式
                    println!(
                        "     📝 影响记忆: {} 个 (详细查看受API限制)",
                        issue.affected_memories.len()
                    );
                }
            } else {
                // 非详细模式，只显示记忆ID数量
                println!("     📝 影响记忆: {} 个", issue.affected_memories.len());
            }

            println!("     💡 建议: {}", issue.recommendation);
            println!();
        }

        if plan.issues.len() > display_issues.len() {
            println!(
                "     ... 还有 {} 个问题未显示，使用 --limit 查看更多",
                plan.issues.len() - display_issues.len()
            );
        }

        println!("🎯 建议的操作:");

        // 获取操作统计
        let action_stats = plan.action_statistics();
        println!("📈 操作统计:");
        println!("  - 总操作数: {}", action_stats.total());
        println!(
            "  - 合并: {} 个, 删除: {} 个, 更新: {} 个, 重分类: {} 个, 归档: {} 个",
            action_stats.merge_count,
            action_stats.delete_count,
            action_stats.update_count,
            action_stats.reclassify_count,
            action_stats.archive_count
        );

        println!();
        let display_actions: Vec<_> = plan
            .actions
            .iter()
            .take(display_issues.len()) // 显示与问题相同数量的操作
            .collect();

        for (i, action) in display_actions.iter().enumerate() {
            println!("  {}. {:?}", i + 1, action);

            // 在详细模式下为每个操作添加解释
            if verbose {
                if let Some(ref details) = memory_details {
                    match action {
                        cortex_mem_core::types::OptimizationAction::Delete { memory_id } => {
                            if let Some(memory) = details.get(memory_id) {
                                println!(
                                    "     📖 将删除内容: \"{}\"",
                                    self.truncate_content(&memory.content, 30)
                                );
                            }
                        }
                        cortex_mem_core::types::OptimizationAction::Merge { memories } => {
                            println!("     🔗 将合并 {} 个记忆", memories.len());
                            if memories.len() > 0 && details.contains_key(&memories[0]) {
                                println!(
                                    "     📖 示例内容: \"{}\"",
                                    self.truncate_content(&details[&memories[0]].content, 30)
                                );
                            }
                        }
                        cortex_mem_core::types::OptimizationAction::Update {
                            memory_id,
                            updates,
                        } => {
                            if let Some(memory) = details.get(memory_id) {
                                println!(
                                    "     📖 更新内容: \"{}\"",
                                    self.truncate_content(&memory.content, 30)
                                );
                                if let Some(new_type) = &updates.memory_type {
                                    println!(
                                        "     🏷️  类型将从 {:?} 更改为 {:?}",
                                        memory.metadata.memory_type, new_type
                                    );
                                }
                            }
                        }
                        cortex_mem_core::types::OptimizationAction::Reclassify { memory_id } => {
                            if let Some(memory) = details.get(memory_id) {
                                println!(
                                    "     📖 重新分类内容: \"{}\"",
                                    self.truncate_content(&memory.content, 30)
                                );
                                println!("     🏷️  当前类型: {:?}", memory.metadata.memory_type);
                            }
                        }
                        cortex_mem_core::types::OptimizationAction::Archive { memory_id } => {
                            if let Some(memory) = details.get(memory_id) {
                                println!(
                                    "     📖 归档内容: \"{}\"",
                                    self.truncate_content(&memory.content, 30)
                                );
                                println!(
                                    "     ⏰ 创建时间: {}",
                                    memory.created_at.format("%Y-%m-%d %H:%M")
                                );
                            }
                        }
                    }
                }
            } else {
                // 非详细模式，显示简单操作描述
                match action {
                    cortex_mem_core::types::OptimizationAction::Delete { memory_id } => {
                        println!(
                            "     🗑️  删除记忆: {}...",
                            &memory_id[..std::cmp::min(8, memory_id.len())]
                        );
                    }
                    cortex_mem_core::types::OptimizationAction::Merge { memories } => {
                        println!("     🔗 合并 {} 个记忆", memories.len());
                    }
                    cortex_mem_core::types::OptimizationAction::Update { memory_id, updates } => {
                        println!(
                            "     ✏️  更新记忆: {}...",
                            &memory_id[..std::cmp::min(8, memory_id.len())]
                        );
                        if let Some(new_type) = &updates.memory_type {
                            println!("     🏷️  更新类型为 {:?}", new_type);
                        }
                    }
                    cortex_mem_core::types::OptimizationAction::Reclassify { memory_id } => {
                        println!(
                            "     🔄 重新分类记忆: {}...",
                            &memory_id[..std::cmp::min(8, memory_id.len())]
                        );
                    }
                    cortex_mem_core::types::OptimizationAction::Archive { memory_id } => {
                        println!(
                            "     📦 归档记忆: {}...",
                            &memory_id[..std::cmp::min(8, memory_id.len())]
                        );
                    }
                }
            }
            println!();
        }

        // 显示未处理的操作数量
        if plan.actions.len() > display_actions.len() {
            println!(
                "     ... 还有 {} 个操作未显示",
                plan.actions.len() - display_actions.len()
            );
        }

        println!(
            "✨ 预计优化后可节省空间 {:.2} MB，提升质量 {:.1}%",
            0.1 * plan.issues.len() as f64, // 简单估算
            5.0 * issue_stats.total() as f64
        ); // 简单估算

        Ok(())
    }

    async fn run_optimization(
        &self,
        optimizer: &dyn cortex_mem_core::memory::MemoryOptimizer,
        request: &cortex_mem_core::types::OptimizationRequest,
        no_confirm: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
            println!(
                "❌ 优化失败: {}",
                result
                    .error_message
                    .unwrap_or_else(|| "未知错误".to_string())
            );
        }

        Ok(())
    }

    pub async fn run_status(
        &self,
        cmd: &OptimizationStatusCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("📈 优化状态");

        if cmd.detailed {
            println!("详细指标功能开发中...");
        }

        if cmd.history {
            println!("历史记录功能开发中...");
        }

        Ok(())
    }

    pub async fn run_config(
        &self,
        cmd: &OptimizationConfigCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if cmd.show {
            println!("优化配置:");
            println!("当前配置功能开发中...");
        } else if cmd.update {
            println!("更新配置功能开发中...");
        }

        Ok(())
    }

    fn build_optimization_request(
        &self,
        cmd: &OptimizeCommand,
    ) -> Result<cortex_mem_core::types::OptimizationRequest, Box<dyn std::error::Error>> {
        let memory_type = cmd
            .memory_type
            .as_ref()
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

        let mut custom_filters = std::collections::HashMap::new();
        custom_filters.insert(
            "limit".to_string(),
            serde_json::Value::Number(serde_json::Number::from(cmd.limit)),
        );
        custom_filters.insert("verbose".to_string(), serde_json::Value::Bool(cmd.verbose));

        let filters = cortex_mem_core::types::OptimizationFilters {
            user_id: cmd.user_id.clone(),
            agent_id: cmd.agent_id.clone(),
            memory_type,
            date_range: None,
            importance_range: None,
            custom_filters,
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

impl OptimizeCommandRunner {
    /// 获取记忆详细信息
    async fn get_memory_details(
        &self,
        issues: &[cortex_mem_core::types::OptimizationIssue],
    ) -> Result<
        std::collections::HashMap<String, cortex_mem_core::types::Memory>,
        Box<dyn std::error::Error>,
    > {
        let mut memory_details = std::collections::HashMap::new();

        // 收集所有需要获取的记忆ID
        let mut all_memory_ids = std::collections::HashSet::new();
        for issue in issues {
            for memory_id in &issue.affected_memories {
                all_memory_ids.insert(memory_id.clone());
            }
        }

        // 批量获取记忆详情
        for memory_id in all_memory_ids {
            match self.memory_manager.get(&memory_id).await {
                Ok(Some(memory)) => {
                    // 记录记忆内容状态
                    if memory.content.trim().is_empty() {
                        tracing::warn!("记忆 {} 内容为空", memory_id);
                    } else {
                        tracing::debug!("记忆 {} 内容长度: {}", memory_id, memory.content.len());
                    }
                    memory_details.insert(memory_id, memory);
                }
                Ok(None) => {
                    tracing::warn!("记忆 {} 不存在", memory_id);
                }
                Err(e) => {
                    tracing::warn!("无法获取记忆 {} 的详细信息: {}", memory_id, e);
                }
            }
        }

        Ok(memory_details)
    }

    /// 格式化严重程度
    fn format_severity(&self, severity: cortex_mem_core::types::IssueSeverity) -> String {
        match severity {
            cortex_mem_core::types::IssueSeverity::Critical => "🔴 严重".to_string(),
            cortex_mem_core::types::IssueSeverity::High => "🟠 高".to_string(),
            cortex_mem_core::types::IssueSeverity::Medium => "🟡 中".to_string(),
            cortex_mem_core::types::IssueSeverity::Low => "🟢 低".to_string(),
        }
    }

    /// 截断内容（安全处理Unicode字符）
    fn truncate_content(&self, content: &str, max_length: usize) -> String {
        if content.len() <= max_length {
            content.to_string()
        } else {
            // 安全地找到字符边界
            let end = match content.char_indices().nth(max_length) {
                Some((idx, _)) => idx,
                None => content.len(),
            };
            format!("{}...", &content[..end])
        }
    }
}
