use chrono::Utc;
use std::sync::Arc;

use crate::{
    error::Result,
    memory::MemoryManager,
    types::{OptimizationAction, OptimizationMetrics, OptimizationResult},
};

use super::optimization_plan::OptimizationPlan;

/// 优化执行引擎 - 负责执行具体的优化操作
pub struct ExecutionEngine {
    memory_manager: Arc<MemoryManager>,
    config: ExecutionEngineConfig,
    #[allow(dead_code)]
    initialized: bool,
}

#[derive(Debug, Clone)]
pub struct ExecutionEngineConfig {
    pub batch_size: usize,
    pub max_concurrent_tasks: usize,
    pub retry_attempts: u32,
}

impl Default for ExecutionEngineConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_concurrent_tasks: 4,
            retry_attempts: 3,
        }
    }
}

impl ExecutionEngine {
    pub fn new() -> Self {
        panic!(
            "ExecutionEngine cannot be constructed without a MemoryManager. Use with_memory_manager() instead."
        );
    }

    pub fn with_config(_config: ExecutionEngineConfig) -> Self {
        panic!(
            "ExecutionEngine cannot be constructed without a MemoryManager. Use with_memory_manager() instead."
        );
    }

    pub fn with_memory_manager(memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            memory_manager,
            config: ExecutionEngineConfig::default(),
            initialized: true,
        }
    }

    /// Get a reference to the LLM client through memory manager
    #[allow(dead_code)]
    fn llm_client(&self) -> &dyn crate::llm::client::LLMClient {
        self.memory_manager.llm_client()
    }

    /// 执行优化计划
    pub async fn execute_plan(
        &self,
        optimization_id: &str,
        plan: OptimizationPlan,
    ) -> Result<OptimizationResult> {
        let start_time = Utc::now();

        tracing::info!(
            optimization_id = optimization_id,
            "开始执行优化计划，{} 个操作",
            plan.actions.len()
        );

        let mut actions_performed = Vec::new();
        let memory_count_before = 0;
        let memory_count_after = 0;

        // 分批执行操作
        let action_batches = plan.actions.chunks(self.config.batch_size);
        let total_batches = action_batches.len();

        for (batch_index, batch) in action_batches.enumerate() {
            tracing::info!(
                optimization_id = optimization_id,
                "执行批次 {}/{}",
                batch_index + 1,
                total_batches
            );

            for action in batch {
                match self.execute_action(action).await {
                    Ok(performed_action) => {
                        actions_performed.push(performed_action);
                    }
                    Err(e) => {
                        tracing::error!(optimization_id = optimization_id, "执行操作失败: {}", e);
                        // 继续执行其他操作，记录错误但不中断整个优化过程
                    }
                }
            }

            // 短暂暂停以避免过度占用资源
            if batch_index < total_batches - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        let end_time = Utc::now();

        // 计算优化指标
        let saved_space_mb = self.calculate_saved_space(&actions_performed).await;
        let deduplication_rate = self.calculate_deduplication_rate(&actions_performed);

        let result = OptimizationResult {
            optimization_id: optimization_id.to_string(),
            strategy: plan.strategy,
            start_time,
            end_time,
            issues_found: plan.issues,
            actions_performed,
            metrics: Some(OptimizationMetrics {
                total_optimizations: 1,
                last_optimization: Some(end_time),
                memory_count_before,
                memory_count_after,
                saved_space_mb,
                deduplication_rate,
                quality_improvement: 0.1,      // 模拟数据
                performance_improvement: 0.15, // 模拟数据
            }),
            success: true,
            error_message: None,
        };

        tracing::info!(
            optimization_id = optimization_id,
            "优化执行完成，{} 个操作",
            result.actions_performed.len()
        );
        Ok(result)
    }

    /// 执行单个优化操作
    async fn execute_action(&self, action: &OptimizationAction) -> Result<OptimizationAction> {
        match action {
            OptimizationAction::Merge { memories } => {
                self.execute_merge(memories).await?;
                Ok(action.clone())
            }
            OptimizationAction::Delete { memory_id } => {
                self.execute_delete(memory_id).await?;
                Ok(action.clone())
            }
            OptimizationAction::Update { memory_id, updates } => {
                self.execute_update(memory_id, updates).await?;
                Ok(action.clone())
            }
            OptimizationAction::Reclassify { memory_id } => {
                self.execute_reclassify(memory_id).await?;
                Ok(action.clone())
            }
            OptimizationAction::Archive { memory_id } => {
                self.execute_archive(memory_id).await?;
                Ok(action.clone())
            }
        }
    }

    /// 执行记忆合并
    async fn execute_merge(&self, memory_ids: &[String]) -> Result<()> {
        if memory_ids.len() < 2 {
            tracing::warn!("合并操作需要至少2个记忆");
            return Ok(());
        }

        tracing::info!("开始合并 {} 个记忆", memory_ids.len());

        // 获取所有要合并的记忆
        let mut memories = Vec::new();
        for memory_id in memory_ids {
            if let Some(memory) = self.memory_manager.get(memory_id).await? {
                memories.push(memory);
            }
        }

        if memories.len() < 2 {
            tracing::warn!("可用的记忆少于2个，无法执行合并");
            return Ok(());
        }

        // 执行合并（使用现有的duplicate detector）
        // 这里需要使用实际的LLM客户端进行内容合并
        let base_memory = &memories[0];
        let merged_content = self.generate_merged_content(&memories).await?;

        let mut merged_memory = base_memory.clone();
        merged_memory.content = merged_content.clone();
        merged_memory.updated_at = Utc::now();

        // 更新合并后的记忆
        self.memory_manager
            .update_complete_memory(
                &merged_memory.id,
                Some(merged_content),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;

        // 删除其他被合并的记忆
        for memory in &memories[1..] {
            if memory.id != base_memory.id {
                let _ = self.memory_manager.delete(&memory.id).await;
            }
        }

        tracing::info!("记忆合并完成");
        Ok(())
    }

    /// 执行记忆删除
    async fn execute_delete(&self, memory_id: &str) -> Result<()> {
        tracing::info!("删除记忆: {}", memory_id);
        self.memory_manager.delete(memory_id).await?;
        Ok(())
    }

    /// 执行记忆更新
    async fn execute_update(
        &self,
        memory_id: &str,
        updates: &crate::types::MemoryUpdates,
    ) -> Result<()> {
        tracing::info!("更新记忆: {}", memory_id);

        // 检查记忆是否存在
        let memory = if let Some(existing) = self.memory_manager.get(memory_id).await? {
            existing
        } else {
            tracing::warn!("记忆不存在: {}", memory_id);
            return Ok(());
        };

        // 使用新的完整更新方法
        self.memory_manager
            .update_complete_memory(
                &memory.id,
                updates.content.clone(),
                updates.memory_type.clone(),
                updates.importance_score,
                updates.entities.clone(),
                updates.topics.clone(),
                updates.custom_metadata.clone(),
            )
            .await?;
        Ok(())
    }

    /// 执行记忆重新分类
    async fn execute_reclassify(&self, memory_id: &str) -> Result<()> {
        tracing::info!("重新分类记忆: {}", memory_id);

        // 获取当前记忆
        let memory = if let Some(existing) = self.memory_manager.get(memory_id).await? {
            existing
        } else {
            tracing::warn!("记忆不存在: {}", memory_id);
            return Ok(());
        };

        // 使用LLM进行精确分类
        let new_memory_type = self.detect_memory_type_from_content(&memory.content).await;

        if memory.metadata.memory_type != new_memory_type {
            // 使用新的update_metadata方法只更新元数据
            self.memory_manager
                .update_metadata(memory_id, new_memory_type.clone())
                .await?;

            tracing::info!("记忆重新分类完成: {} -> {:?}", memory_id, new_memory_type);
        } else {
            tracing::info!("记忆分类无需更改: {}", memory_id);
        }

        Ok(())
    }

    /// 执行记忆归档
    async fn execute_archive(&self, memory_id: &str) -> Result<()> {
        tracing::info!("归档记忆: {}", memory_id);

        // 获取当前记忆
        let mut memory = if let Some(existing) = self.memory_manager.get(memory_id).await? {
            existing
        } else {
            tracing::warn!("记忆不存在: {}", memory_id);
            return Ok(());
        };

        // 添加归档标记
        memory
            .metadata
            .custom
            .insert("archived".to_string(), serde_json::Value::Bool(true));
        memory.metadata.custom.insert(
            "archived_at".to_string(),
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );

        memory.updated_at = Utc::now();

        self.memory_manager
            .update(&memory.id, memory.content)
            .await?;
        Ok(())
    }

    /// 生成合并后的内容
    async fn generate_merged_content(&self, memories: &[crate::types::Memory]) -> Result<String> {
        if memories.is_empty() {
            return Ok(String::new());
        }

        if memories.len() == 1 {
            return Ok(memories[0].content.clone());
        }

        tracing::info!("使用LLM智能合并 {} 个记忆", memories.len());

        // 构建合并提示
        let mut prompt = String::new();
        prompt.push_str("请将以下多个相关记忆合并成一个连贯、完整、简洁的记忆。保留所有重要信息，去除冗余内容，确保逻辑连贯。\n\n");

        for (i, memory) in memories.iter().enumerate() {
            prompt.push_str(&format!("记忆 {}:\n{}\n\n", i + 1, memory.content));
        }

        prompt.push_str("请生成合并后的记忆内容：");

        // 使用LLM客户端生成合并内容
        let llm_client = self.memory_manager.llm_client();
        let merged_content = llm_client.complete(&prompt).await?;

        tracing::info!("LLM生成合并内容完成，长度: {}", merged_content.len());
        Ok(merged_content.trim().to_string())
    }

    /// 计算节省的空间
    async fn calculate_saved_space(&self, actions: &[OptimizationAction]) -> f64 {
        let mut saved_bytes = 0;

        for action in actions {
            match action {
                OptimizationAction::Merge { memories } => {
                    // 合并操作，节省n-1个记忆的空间
                    let saved_memories = memories.len().saturating_sub(1);
                    saved_bytes += saved_memories * 1024; // 假设每个记忆平均1KB
                }
                OptimizationAction::Delete { .. } => {
                    // 删除操作，节省1个记忆的空间
                    saved_bytes += 1024;
                }
                _ => {}
            }
        }

        saved_bytes as f64 / 1024.0 / 1024.0 // 转换为MB
    }

    /// 计算去重率
    fn calculate_deduplication_rate(&self, actions: &[OptimizationAction]) -> f32 {
        let total_merge_actions = actions
            .iter()
            .filter(|action| matches!(action, OptimizationAction::Merge { .. }))
            .count() as f32;

        if actions.is_empty() {
            0.0
        } else {
            total_merge_actions / actions.len() as f32
        }
    }

    /// 使用LLM从内容检测记忆类型
    async fn detect_memory_type_from_content(&self, content: &str) -> crate::types::MemoryType {
        let llm_client = self.memory_manager.llm_client();

        // 检查内容是否为空或过短
        if content.trim().is_empty() {
            tracing::warn!("记忆内容为空，默认分类为Conversational");
            return crate::types::MemoryType::Conversational;
        }

        if content.trim().len() < 5 {
            tracing::warn!("记忆内容过短: '{}'，默认分类为Conversational", content);
            return crate::types::MemoryType::Conversational;
        }

        // 记录调试信息
        tracing::debug!(
            "开始对记忆内容进行LLM分类: '{}...'",
            content.chars().take(50).collect::<String>()
        );

        // 创建分类提示
        let prompt = format!(
            r#"Classify the following memory content into one of these categories:

1. Conversational - Dialogue, conversations, or interactive exchanges
2. Procedural - Instructions, how-to information, or step-by-step processes
3. Factual - Objective facts, data, or verifiable information
4. Semantic - Concepts, meanings, definitions, or general knowledge
5. Episodic - Specific events, experiences, or temporal information
6. Personal - Personal preferences, characteristics, or individual-specific information

Content: "{}"

Respond with only the category name (e.g., "Conversational", "Procedural", etc.):"#,
            content
        );

        // 使用LLM分类器进行分类
        match llm_client.classify_memory(&prompt).await {
            Ok(classification) => {
                let memory_type = match classification.memory_type.as_str() {
                    "Conversational" => crate::types::MemoryType::Conversational,
                    "Procedural" => crate::types::MemoryType::Procedural,
                    "Factual" => crate::types::MemoryType::Factual,
                    "Semantic" => crate::types::MemoryType::Semantic,
                    "Episodic" => crate::types::MemoryType::Episodic,
                    "Personal" => crate::types::MemoryType::Personal,
                    _ => crate::types::MemoryType::Conversational, // 默认回退
                };

                tracing::info!(
                    "LLM分类成功: '{}' -> {:?} (置信度: {})",
                    content.chars().take(30).collect::<String>(),
                    memory_type,
                    classification.confidence
                );

                memory_type
            }
            Err(e) => {
                tracing::error!(
                    "LLM分类失败: '{}' -> 错误: {}, 使用默认分类Conversational",
                    content.chars().take(30).collect::<String>(),
                    e
                );
                crate::types::MemoryType::Conversational // 失败时的回退
            }
        }
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        panic!(
            "ExecutionEngine cannot be constructed without a MemoryManager. Use with_memory_manager() instead."
        );
    }
}
