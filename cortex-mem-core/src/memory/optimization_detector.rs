use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::{
    error::Result,
    types::{
        OptimizationFilters, OptimizationIssue, IssueKind, IssueSeverity,
    },
    memory::MemoryManager,
};

/// 优化问题检测器
pub struct OptimizationDetector {
    // 检测器配置
    config: OptimizationDetectorConfig,
    memory_manager: Arc<MemoryManager>,
}

#[derive(Debug, Clone)]
pub struct OptimizationDetectorConfig {
    pub duplicate_threshold: f32,
    pub quality_threshold: f32,
    pub time_decay_days: u32,
    pub max_issues_per_type: usize,
}

impl Default for OptimizationDetectorConfig {
    fn default() -> Self {
        Self {
            duplicate_threshold: 0.85,
            quality_threshold: 0.4,
            time_decay_days: 30,
            max_issues_per_type: 1000,
        }
    }
}

impl OptimizationDetector {
    pub fn new() -> Self {
        // 需要MemoryManager才能使用，需要使用with_memory_manager
        panic!("OptimizationDetector requires MemoryManager. Use with_memory_manager() instead.");
    }
    
    pub fn with_memory_manager(memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            config: OptimizationDetectorConfig::default(),
            memory_manager,
        }
    }
    
    pub fn with_config(config: OptimizationDetectorConfig, memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            config,
            memory_manager,
        }
    }
    
    /// 检测需要优化的内存问题
    pub async fn detect_issues(&self, filters: &OptimizationFilters) -> Result<Vec<OptimizationIssue>> {
        tracing::info!("开始检测内存优化问题");
        
        // 转换为MemoryManager使用的Filters
        let mm_filters = crate::types::Filters {
            user_id: filters.user_id.clone(),
            agent_id: filters.agent_id.clone(),
            run_id: None,
            memory_type: filters.memory_type.as_ref().map(|mt| mt.clone()),
            actor_id: None,
            min_importance: filters.importance_range.as_ref().and_then(|r| r.min),
            max_importance: filters.importance_range.as_ref().and_then(|r| r.max),
            created_after: filters.date_range.as_ref().and_then(|r| r.start),
            created_before: filters.date_range.as_ref().and_then(|r| r.end),
            updated_after: None,
            updated_before: None,
            entities: None,
            topics: None,
            custom: filters.custom_filters.clone(),
        };
        
        let mut all_issues = Vec::new();
        
        // 1. 检测重复问题
        let duplicates = self.detect_duplicates(&mm_filters).await?;
        all_issues.extend(duplicates);
        
        // 2. 检测质量问题
        let quality_issues = self.detect_quality_issues(&mm_filters).await?;
        all_issues.extend(quality_issues);
        
        // 3. 检测过时问题
        let outdated_issues = self.detect_outdated_issues(&mm_filters).await?;
        all_issues.extend(outdated_issues);
        
        // 4. 检测分类问题
        let classification_issues = self.detect_classification_issues(&mm_filters).await?;
        all_issues.extend(classification_issues);
        
        // 5. 检测空间效率问题
        let space_issues = self.detect_space_inefficiency(&mm_filters).await?;
        all_issues.extend(space_issues);
        
        // 限制每个类型的问题数量
        all_issues = self.limit_issues_per_type(all_issues);
        
        tracing::info!("检测完成，发现 {} 个问题", all_issues.len());
        Ok(all_issues)
    }
    
    /// 检测重复记忆
    async fn detect_duplicates(&self, filters: &crate::types::Filters) -> Result<Vec<OptimizationIssue>> {
        tracing::info!("检测重复记忆");
        
        let mut issues = Vec::new();
        
        // 获取所有记忆
        let memories = self.memory_manager.list(filters, None).await?;
        
        if memories.len() < 2 {
            tracing::debug!("记忆数量不足，跳过重复检测");
            return Ok(issues);
        }
        
        // 直接使用内存管理器进行重复检测
        // TODO: 实现真正的重复检测逻辑
        
        // 检测重复记忆组
        let mut processed_memories = std::collections::HashSet::new();
        
        for (i, memory_i) in memories.iter().enumerate() {
            if processed_memories.contains(&memory_i.id) {
                continue;
            }
            
            let mut similar_memories = Vec::new();
            
            // 与其他记忆进行比较
            for (j, memory_j) in memories.iter().enumerate() {
                if i >= j || processed_memories.contains(&memory_j.id) {
                    continue;
                }
                
                // 计算语义相似度
                let similarity = self.calculate_semantic_similarity(
                    &memory_i.content, 
                    &memory_j.content
                ).await?;
                
                if similarity >= self.config.duplicate_threshold {
                    similar_memories.push(memory_j.clone());
                    processed_memories.insert(memory_j.id.clone());
                }
            }
            
            if similar_memories.len() > 0 {
                // 发现重复记忆组
                let mut affected_memories = vec![memory_i.clone()];
                affected_memories.extend(similar_memories.clone());
                
                let duplicate_count = affected_memories.len();
                let severity = if similar_memories.len() > 2 { 
                    IssueSeverity::High 
                } else { 
                    IssueSeverity::Medium 
                };
                
                let issue = OptimizationIssue {
                    id: Uuid::new_v4().to_string(),
                    kind: IssueKind::Duplicate,
                    severity,
                    description: format!("检测到 {} 个高度相似的重复记忆", duplicate_count),
                    affected_memories: affected_memories.iter().map(|m| m.id.clone()).collect(),
                    recommendation: format!("建议合并这 {} 个重复记忆", duplicate_count),
                };
                issues.push(issue);
                processed_memories.insert(memory_i.id.clone());
            }
        }
        
        tracing::info!("重复检测完成，发现 {} 个重复问题", issues.len());
        Ok(issues)
    }
    
    /// 检测质量问题
    async fn detect_quality_issues(&self, filters: &crate::types::Filters) -> Result<Vec<OptimizationIssue>> {
        tracing::info!("检测质量问题");
        
        let mut issues = Vec::new();
        
        // 获取所有记忆
        let memories = self.memory_manager.list(filters, None).await?;
        
        for memory in memories {
            let quality_score = self.evaluate_memory_quality(&memory).await?;
            
            if quality_score < self.config.quality_threshold {
                let issue = OptimizationIssue {
                    id: Uuid::new_v4().to_string(),
                    kind: IssueKind::LowQuality,
                    severity: if quality_score < self.config.quality_threshold / 2.0 {
                        IssueSeverity::High
                    } else {
                        IssueSeverity::Low
                    },
                    description: format!("记忆质量评分过低: {:.2} (阈值: {:.2})", quality_score, self.config.quality_threshold),
                    affected_memories: vec![memory.id],
                    recommendation: "建议更新或删除低质量记忆".to_string(),
                };
                issues.push(issue);
            }
        }
        
        tracing::info!("质量检测完成，发现 {} 个质量问题", issues.len());
        Ok(issues)
    }
    
    /// 检测过时问题
    async fn detect_outdated_issues(&self, filters: &crate::types::Filters) -> Result<Vec<OptimizationIssue>> {
        tracing::info!("检测过时问题");
        
        let mut issues = Vec::new();
        
        // 获取所有记忆
        let memories = self.memory_manager.list(filters, None).await?;
        
        let _cutoff_date = Utc::now() - chrono::Duration::days(self.config.time_decay_days as i64);
        
        for memory in memories {
            let days_since_update = (Utc::now() - memory.updated_at).num_days();
            let is_outdated = days_since_update as u32 > self.config.time_decay_days;
            
            if is_outdated {
                let severity = if days_since_update as u32 > self.config.time_decay_days * 2 {
                    IssueSeverity::High
                } else if days_since_update as u32 > (self.config.time_decay_days as f32 * 1.5) as u32 {
                    IssueSeverity::Medium
                } else {
                    IssueSeverity::Low
                };
                
                let recommendation = if severity == IssueSeverity::High {
                    "建议删除过时记忆".to_string()
                } else {
                    "建议归档过时记忆".to_string()
                };
                
                let issue = OptimizationIssue {
                    id: Uuid::new_v4().to_string(),
                    kind: IssueKind::Outdated,
                    severity,
                    description: format!("记忆已 {} 天未更新，超过阈值 {} 天", days_since_update, self.config.time_decay_days),
                    affected_memories: vec![memory.id],
                    recommendation,
                };
                issues.push(issue);
            }
        }
        
        tracing::info!("过时检测完成，发现 {} 个过时问题", issues.len());
        Ok(issues)
    }
    
    /// 检测分类问题
    async fn detect_classification_issues(&self, filters: &crate::types::Filters) -> Result<Vec<OptimizationIssue>> {
        tracing::info!("检测分类问题");
        
        let mut issues = Vec::new();
        
        // 获取所有记忆
        let memories = self.memory_manager.list(filters, None).await?;
        
        for memory in memories {
            let classification_issues = self.check_classification_quality(&memory).await?;
            
            for issue_desc in classification_issues {
                let issue = OptimizationIssue {
                    id: Uuid::new_v4().to_string(),
                    kind: IssueKind::PoorClassification,
                    severity: IssueSeverity::Low,
                    description: format!("分类问题: {}", issue_desc),
                    affected_memories: vec![memory.id.clone()],
                    recommendation: "建议重新分类记忆".to_string(),
                };
                issues.push(issue);
            }
        }
        
        tracing::info!("分类检测完成，发现 {} 个分类问题", issues.len());
        Ok(issues)
    }
    
    /// 检测空间效率问题
    async fn detect_space_inefficiency(&self, filters: &crate::types::Filters) -> Result<Vec<OptimizationIssue>> {
        tracing::info!("检测空间效率问题");
        
        let mut issues = Vec::new();
        
        // 获取所有记忆
        let memories = self.memory_manager.list(filters, None).await?;
        
        // 获取统计数据
        let stats = self.memory_manager.get_stats(filters).await?;
        
        // 1. 检查单个记忆的大小问题
        for memory in &memories {
            let memory_size = memory.content.len() + memory.embedding.len() * 4; // 粗略估算
            
            // 如果记忆超过一定大小且重要性很低
            if memory_size > 10000 && memory.metadata.importance_score < 0.3 {
                let issue = OptimizationIssue {
                    id: Uuid::new_v4().to_string(),
                    kind: IssueKind::SpaceInefficient,
                    severity: IssueSeverity::Low,
                    description: format!("大记忆占用空间过多且重要性低，大小: {} 字节", memory_size),
                    affected_memories: vec![memory.id.clone()],
                    recommendation: "建议对大记忆进行摘要或归档".to_string(),
                };
                issues.push(issue);
            }
        }
        
        // 2. 检查总存储情况
        let total_memories = stats.total_count;
        if total_memories > 10000 {
            let issue = OptimizationIssue {
                id: Uuid::new_v4().to_string(),
                kind: IssueKind::SpaceInefficient,
                severity: IssueSeverity::Medium,
                description: format!("记忆数量过多: {}，可能影响查询性能", total_memories),
                affected_memories: Vec::new(), // 影响所有记忆
                recommendation: "建议进行深度优化和清理".to_string(),
            };
            issues.push(issue);
        }
        
        // 3. 检查低重要性记忆
        let low_importance_memories: Vec<_> = memories.iter()
            .filter(|m| m.metadata.importance_score < 0.2)
            .collect();
            
        if low_importance_memories.len() > total_memories / 4 {
            let issue = OptimizationIssue {
                id: Uuid::new_v4().to_string(),
                kind: IssueKind::SpaceInefficient,
                severity: IssueSeverity::Medium,
                description: format!("低重要性记忆过多: {} / {} ({:.1}%)", 
                    low_importance_memories.len(), 
                    total_memories,
                    low_importance_memories.len() as f64 / total_memories as f64 * 100.0),
                affected_memories: low_importance_memories.iter().map(|m| m.id.clone()).collect(),
                recommendation: "建议归档或删除低重要性记忆".to_string(),
            };
            issues.push(issue);
        }
        
        tracing::info!("空间效率检测完成，发现 {} 个空间问题", issues.len());
        Ok(issues)
    }
    
    /// 计算记忆的语义相似度
    async fn calculate_semantic_similarity(
        &self,
        content1: &str,
        content2: &str,
    ) -> Result<f32> {
        // 使用LLM客户端计算embedding并计算余弦相似度
        let llm_client = self.memory_manager.llm_client();
        
        // 获取两个内容的embedding
        let embedding1 = llm_client.embed(content1).await?;
        let embedding2 = llm_client.embed(content2).await?;
        
        // 计算余弦相似度
        let similarity = self.cosine_similarity(&embedding1, &embedding2);
        
        tracing::debug!("语义相似度计算: {} vs {} = {:.3}", 
            content1.chars().take(50).collect::<String>(),
            content2.chars().take(50).collect::<String>(),
            similarity);
        
        Ok(similarity)
    }
    
    /// 计算余弦相似度
    fn cosine_similarity(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        if vec1.len() != vec2.len() || vec1.is_empty() {
            return 0.0;
        }
        
        let mut dot_product = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;
        
        for i in 0..vec1.len() {
            dot_product += vec1[i] * vec2[i];
            norm1 += vec1[i] * vec1[i];
            norm2 += vec2[i] * vec2[i];
        }
        
        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm1.sqrt() * norm2.sqrt())
    }
    
    /// 评估记忆质量
    async fn evaluate_memory_quality(&self, memory: &crate::types::Memory) -> Result<f32> {
        let mut quality_score = 0.0;
        let max_score = 1.0;
        
        // 1. 内容长度评分 (30%)
        let content_length_score = if memory.content.len() < 10 {
            0.1
        } else if memory.content.len() < 50 {
            0.5
        } else if memory.content.len() < 200 {
            0.8
        } else {
            1.0
        };
        quality_score += content_length_score * 0.3;
        
        // 2. 结构化程度评分 (20%)
        let has_sentences = memory.content.contains('.') || memory.content.contains('!') || memory.content.contains('?');
        let has_paragraphs = memory.content.contains('\n');
        let structural_score = if has_sentences && has_paragraphs {
            1.0
        } else if has_sentences || has_paragraphs {
            0.7
        } else {
            0.3
        };
        quality_score += structural_score * 0.2;
        
        // 3. 重要性评分 (20%)
        quality_score += memory.metadata.importance_score * 0.2;
        
        // 4. 元数据完整性 (15%)
        let metadata_score = if !memory.metadata.entities.is_empty() && !memory.metadata.topics.is_empty() {
            1.0
        } else if !memory.metadata.entities.is_empty() || !memory.metadata.topics.is_empty() {
            0.6
        } else {
            0.2
        };
        quality_score += metadata_score * 0.15;
        
        // 5. 更新频率评分 (15%)
        let days_since_update = (chrono::Utc::now() - memory.updated_at).num_days();
        let update_score = if days_since_update < 7 {
            1.0
        } else if days_since_update < 30 {
            0.8
        } else if days_since_update < 90 {
            0.5
        } else {
            0.2
        };
        quality_score += update_score * 0.15;
        
        Ok(quality_score.min(max_score))
    }
    
    /// 检查分类质量
    async fn check_classification_quality(&self, memory: &crate::types::Memory) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // 只有当内容非常短且为默认类型时才检查类型是否合适
        if memory.metadata.memory_type == crate::types::MemoryType::Conversational && memory.content.len() < 20 {
            tracing::debug!("记忆 {} 太短且为默认类型，建议重新分类", memory.id);
        }
        
        // 2. 检查实体提取 - 只有内容很长时才检查
        if memory.metadata.entities.is_empty() && memory.content.len() > 200 {
            issues.push("缺少实体信息".to_string());
        }
        
        // 3. 检查主题提取 - 只有内容很长时才检查
        if memory.metadata.topics.is_empty() && memory.content.len() > 100 {
            issues.push("缺少主题信息".to_string());
        }
        
        // 4. 检查记忆类型与内容是否匹配 - 更宽松的逻辑
        let detected_type = self.detect_memory_type_from_content(&memory.content);
        
        // 如果检测到的类型与当前类型不同，且内容足够长，才认为是问题
        if detected_type != memory.metadata.memory_type && memory.content.len() > 50 {
            issues.push(format!("记忆类型与内容可能不匹配: 当前 {:?}, 检测到 {:?}", 
                memory.metadata.memory_type, detected_type));
        }
        
        Ok(issues)
    }
    
    /// 从内容检测记忆类型
    fn detect_memory_type_from_content(&self, content: &str) -> crate::types::MemoryType {
        let content_lower = content.to_lowercase();
        
        // 程序性关键词 (英文 + 中文)
        if content_lower.contains("how") || content_lower.contains("step") || 
           content_lower.contains("method") || content_lower.contains("process") || 
           content_lower.contains("操作") || content_lower.contains("如何") ||
           content_lower.contains("方法") || content_lower.contains("步骤") {
            return crate::types::MemoryType::Procedural;
        }
        
        // 事实性关键词 (英文 + 中文)
        if content_lower.contains("fact") || content_lower.contains("info") || 
           content_lower.contains("data") || content_lower.contains("knowledge") ||
           content_lower.contains("事实") || content_lower.contains("信息") ||
           content_lower.contains("数据") || content_lower.contains("关于") {
            return crate::types::MemoryType::Factual;
        }
        
        // 语义关键词 (英文 + 中文)
        if content_lower.contains("concept") || content_lower.contains("meaning") || 
           content_lower.contains("understand") || content_lower.contains("definition") ||
           content_lower.contains("概念") || content_lower.contains("含义") ||
           content_lower.contains("理解") || content_lower.contains("定义") {
            return crate::types::MemoryType::Semantic;
        }
        
        // 情节性关键词 (英文 + 中文)
        if content_lower.contains("happen") || content_lower.contains("experience") || 
           content_lower.contains("event") || content_lower.contains("when") ||
           content_lower.contains("发生") || content_lower.contains("经历") ||
           content_lower.contains("事件") || content_lower.contains("时间") {
            return crate::types::MemoryType::Episodic;
        }
        
        // 个人性关键词 (英文 + 中文)
        if content_lower.contains("like") || content_lower.contains("prefer") || 
           content_lower.contains("personality") || content_lower.contains("habit") ||
           content_lower.contains("喜欢") || content_lower.contains("偏好") ||
           content_lower.contains("个性") || content_lower.contains("习惯") {
            return crate::types::MemoryType::Personal;
        }
        
        // 默认是对话型
        crate::types::MemoryType::Conversational
    }
    
    /// 限制每个类型的问题数量
    fn limit_issues_per_type(&self, issues: Vec<OptimizationIssue>) -> Vec<OptimizationIssue> {
        let mut issues_by_type: std::collections::HashMap<IssueKind, Vec<OptimizationIssue>> = 
            std::collections::HashMap::new();
        
        for issue in &issues {
            issues_by_type
                .entry(issue.kind.clone())
                .or_insert_with(Vec::new)
                .push(issue.clone());
        }
        
        let mut limited_issues = Vec::new();
        
        for (kind, mut kind_issues) in issues_by_type {
            if kind_issues.len() > self.config.max_issues_per_type {
                kind_issues.truncate(self.config.max_issues_per_type);
                tracing::warn!("{:?} 类型的问题数量超过限制，截取到 {} 个", kind, self.config.max_issues_per_type);
            }
            limited_issues.extend(kind_issues);
        }
        
        limited_issues
    }
}

impl Default for OptimizationDetector {
    fn default() -> Self {
        panic!("OptimizationDetector requires MemoryManager. Use with_memory_manager() instead.");
    }
}