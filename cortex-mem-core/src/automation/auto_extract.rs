use crate::{
    extraction::{ExtractedMemories, MemoryExtractor},
    filesystem::CortexFilesystem,
    llm::LLMClient,
    session::{SessionManager, SessionMetadata},
    Result,
};
use std::sync::Arc;
use tracing::{info, warn};

/// 会话自动提取配置
#[derive(Debug, Clone)]
pub struct AutoExtractConfig {
    /// 触发自动提取的最小消息数
    pub min_message_count: usize,
    /// 是否在会话关闭时自动提取
    pub extract_on_close: bool,
    /// 是否保存用户记忆
    pub save_user_memories: bool,
    /// 是否保存Agent记忆
    pub save_agent_memories: bool,
}

impl Default for AutoExtractConfig {
    fn default() -> Self {
        Self {
            min_message_count: 5,
            extract_on_close: true,
            save_user_memories: true,
            save_agent_memories: true,
        }
    }
}

/// 自动提取统计
#[derive(Debug, Clone, Default)]
pub struct AutoExtractStats {
    pub facts_extracted: usize,
    pub decisions_extracted: usize,
    pub entities_extracted: usize,
    pub user_memories_saved: usize,
    pub agent_memories_saved: usize,
}

/// 会话自动提取器
/// 
/// 参考OpenViking的自迭代机制：
/// 1. 在会话关闭时自动触发LLM提取
/// 2. 将提取的记忆分类存储（用户记忆、Agent记忆）
/// 3. 支持增量更新
pub struct AutoExtractor {
    filesystem: Arc<CortexFilesystem>,
    llm: Arc<LLMClient>,
    extractor: MemoryExtractor,
    config: AutoExtractConfig,
}

impl AutoExtractor {
    /// 创建新的自动提取器
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        llm: Arc<LLMClient>,
        config: AutoExtractConfig,
    ) -> Self {
        let extraction_config = crate::extraction::ExtractionConfig::default();
        let extractor = MemoryExtractor::new(filesystem.clone(), llm.clone(), extraction_config);

        Self {
            filesystem,
            llm,
            extractor,
            config,
        }
    }

    /// 在会话关闭时自动提取
    pub async fn on_session_close(
        &self,
        session: &SessionMetadata,
    ) -> Result<Option<AutoExtractStats>> {
        if !self.config.extract_on_close {
            return Ok(None);
        }

        // 检查消息数是否达到阈值
        if session.message_count < self.config.min_message_count {
            info!(
                "Session {} has only {} messages, skipping auto-extraction (threshold: {})",
                session.thread_id, session.message_count, self.config.min_message_count
            );
            return Ok(None);
        }

        info!(
            "Auto-extracting memories from session: {} ({} messages)",
            session.thread_id, session.message_count
        );

        // 执行提取
        let extracted = self.extractor.extract_from_thread(&session.thread_id).await?;

        let mut stats = AutoExtractStats {
            facts_extracted: extracted.facts.len(),
            decisions_extracted: extracted.decisions.len(),
            entities_extracted: extracted.entities.len(),
            user_memories_saved: 0,
            agent_memories_saved: 0,
        };

        // 保存提取结果
        self.extractor.save_extraction(&session.thread_id, &extracted).await?;

        // 分类存储记忆
        if self.config.save_user_memories {
            stats.user_memories_saved = self.save_user_memories(&session.thread_id, &extracted).await?;
        }

        if self.config.save_agent_memories {
            stats.agent_memories_saved = self.save_agent_memories(&session.thread_id, &extracted).await?;
        }

        info!(
            "Auto-extraction complete: {} facts, {} decisions, {} entities",
            stats.facts_extracted, stats.decisions_extracted, stats.entities_extracted
        );

        Ok(Some(stats))
    }

    /// 保存用户记忆
    /// 
    /// 参考OpenViking的User Memory Update机制：
    /// - 用户的个人信息、偏好、习惯
    /// - 存储到 cortex://users/{user_id}/memories/
    async fn save_user_memories(
        &self,
        thread_id: &str,
        extracted: &ExtractedMemories,
    ) -> Result<usize> {
        use crate::filesystem::FilesystemOperations;

        // 目前简化实现：将所有facts保存为用户记忆
        // 未来可以通过LLM判断哪些是用户相关的

        let user_id = "default"; // 可以从session metadata获取
        let memories_dir = format!("cortex://users/{}/memories", user_id);

        let mut saved_count = 0;

        // 保存重要的facts作为用户记忆
        for fact in &extracted.facts {
            if fact.confidence >= 0.7 {
                let memory_id = uuid::Uuid::new_v4();
                let memory_uri = format!("{}/{}.md", memories_dir, memory_id);

                let content = format!(
                    "# User Memory\n\n\
                    **Source**: {}\n\
                    **Extracted**: {}\n\
                    **Confidence**: {}\n\n\
                    ## Content\n\n\
                    {}\n",
                    thread_id,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    fact.confidence,
                    fact.content
                );

                match self.filesystem.write(&memory_uri, &content).await {
                    Ok(_) => {
                        saved_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to save user memory: {}", e);
                    }
                }
            }
        }

        info!("Saved {} user memories", saved_count);
        Ok(saved_count)
    }

    /// 保存Agent记忆
    /// 
    /// 参考OpenViking的Agent Memory Update机制：
    /// - Agent学到的知识、经验、决策模式
    /// - 存储到 cortex://agents/{agent_id}/memories/
    async fn save_agent_memories(
        &self,
        thread_id: &str,
        extracted: &ExtractedMemories,
    ) -> Result<usize> {
        use crate::filesystem::FilesystemOperations;

        let agent_id = "default"; // 可以从session metadata获取
        let memories_dir = format!("cortex://agents/{}/memories", agent_id);

        let mut saved_count = 0;

        // 保存decisions作为Agent记忆
        for decision in &extracted.decisions {
            if decision.confidence >= 0.7 {
                let memory_id = uuid::Uuid::new_v4();
                let memory_uri = format!("{}/{}.md", memories_dir, memory_id);

                let content = format!(
                    "# Agent Memory (Decision)\n\n\
                    **Source**: {}\n\
                    **Extracted**: {}\n\
                    **Confidence**: {}\n\n\
                    ## Decision\n\n\
                    {}\n\n\
                    ## Context\n\n\
                    {}\n\n\
                    ## Rationale\n\n\
                    {}\n",
                    thread_id,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    decision.confidence,
                    decision.decision,
                    decision.context,
                    decision.rationale.as_deref().unwrap_or("N/A")
                );

                match self.filesystem.write(&memory_uri, &content).await {
                    Ok(_) => {
                        saved_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to save agent memory: {}", e);
                    }
                }
            }
        }

        info!("Saved {} agent memories", saved_count);
        Ok(saved_count)
    }

    /// 手动触发提取（可用于测试或手动操作）
    pub async fn extract_session(&self, thread_id: &str) -> Result<AutoExtractStats> {
        info!("Manually extracting memories from session: {}", thread_id);

        let extracted = self.extractor.extract_from_thread(thread_id).await?;

        let mut stats = AutoExtractStats {
            facts_extracted: extracted.facts.len(),
            decisions_extracted: extracted.decisions.len(),
            entities_extracted: extracted.entities.len(),
            user_memories_saved: 0,
            agent_memories_saved: 0,
        };

        // 保存提取结果
        self.extractor.save_extraction(thread_id, &extracted).await?;

        // 分类存储
        if self.config.save_user_memories {
            stats.user_memories_saved = self.save_user_memories(thread_id, &extracted).await?;
        }

        if self.config.save_agent_memories {
            stats.agent_memories_saved = self.save_agent_memories(thread_id, &extracted).await?;
        }

        Ok(stats)
    }
}

/// 增强SessionManager支持自动提取
pub struct AutoSessionManager {
    session_manager: SessionManager,
    auto_extractor: Option<Arc<AutoExtractor>>,
}

impl AutoSessionManager {
    /// 创建新的自动会话管理器
    pub fn new(
        session_manager: SessionManager,
        auto_extractor: Option<Arc<AutoExtractor>>,
    ) -> Self {
        Self {
            session_manager,
            auto_extractor,
        }
    }

    /// 关闭会话并自动提取
    pub async fn close_session(&mut self, thread_id: &str) -> Result<SessionMetadata> {
        // 先关闭会话
        let metadata = self.session_manager.close_session(thread_id).await?;

        // 如果配置了自动提取器，执行提取
        if let Some(extractor) = &self.auto_extractor {
            match extractor.on_session_close(&metadata).await {
                Ok(Some(stats)) => {
                    info!(
                        "Session {} auto-extraction: {} facts, {} user memories, {} agent memories",
                        thread_id,
                        stats.facts_extracted,
                        stats.user_memories_saved,
                        stats.agent_memories_saved
                    );
                }
                Ok(None) => {
                    info!("Session {} skipped auto-extraction", thread_id);
                }
                Err(e) => {
                    warn!("Session {} auto-extraction failed: {}", thread_id, e);
                }
            }
        }

        Ok(metadata)
    }

    /// 获取内部SessionManager的引用
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// 获取内部SessionManager的可变引用
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_extract_config_default() {
        let config = AutoExtractConfig::default();
        assert_eq!(config.min_message_count, 5);
        assert!(config.extract_on_close);
        assert!(config.save_user_memories);
        assert!(config.save_agent_memories);
    }
}
