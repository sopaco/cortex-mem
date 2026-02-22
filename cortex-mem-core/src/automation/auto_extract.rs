use crate::{
    Result,
    extraction::MemoryExtractor,
    filesystem::CortexFilesystem,
    llm::LLMClient,
    session::SessionManager,
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
}

impl Default for AutoExtractConfig {
    fn default() -> Self {
        Self {
            min_message_count: 5,
            extract_on_close: true,
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
/// 🔧 简化版本：移除了profile.json相关代码
/// 现在所有记忆统一由SessionManager的MemoryExtractor处理
pub struct AutoExtractor {
    #[allow(dead_code)]
    filesystem: Arc<CortexFilesystem>,
    #[allow(dead_code)]
    llm: Arc<dyn LLMClient>,
    #[allow(dead_code)]
    extractor: MemoryExtractor,
    #[allow(dead_code)]
    config: AutoExtractConfig,
    /// 用户ID（保留用于兼容性）
    user_id: String,
}

impl AutoExtractor {
    /// 创建新的自动提取器
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        llm: Arc<dyn LLMClient>,
        config: AutoExtractConfig,
    ) -> Self {
        let extraction_config = crate::extraction::ExtractionConfig::default();
        let extractor = MemoryExtractor::new(filesystem.clone(), llm.clone(), extraction_config);

        Self {
            filesystem,
            llm,
            extractor,
            config,
            user_id: "default".to_string(),
        }
    }

    /// 创建新的自动提取器,指定用户ID
    pub fn with_user_id(
        filesystem: Arc<CortexFilesystem>,
        llm: Arc<dyn LLMClient>,
        config: AutoExtractConfig,
        user_id: impl Into<String>,
    ) -> Self {
        let extraction_config = crate::extraction::ExtractionConfig::default();
        let extractor = MemoryExtractor::new(filesystem.clone(), llm.clone(), extraction_config);

        Self {
            filesystem,
            llm,
            extractor,
            config,
            user_id: user_id.into(),
        }
    }

    /// 设置用户ID
    pub fn set_user_id(&mut self, user_id: impl Into<String>) {
        self.user_id = user_id.into();
    }

    /// 🔧 简化:extract_session现在只需要直接使用SessionManager处理即可
    /// AutoExtractor不再负责用户记忆提取(由MemoryExtractor统一处理)
    pub async fn extract_session(&self, _thread_id: &str) -> Result<AutoExtractStats> {
        info!("AutoExtractor::extract_session is deprecated - all memory extraction is now handled by SessionManager::close_session");
        warn!("Use SessionManager::close_session instead. This method returns empty stats for compatibility.");
        
        Ok(AutoExtractStats::default())
    }
}

/// 增强SessionManager支持自动提取
pub struct AutoSessionManager {
    session_manager: SessionManager,
    #[allow(dead_code)]
    auto_extractor: AutoExtractor,
}

impl AutoSessionManager {
    /// 创建新的自动会话管理器
    pub fn new(
        session_manager: SessionManager,
        auto_extractor: AutoExtractor,
    ) -> Self {
        Self {
            session_manager,
            auto_extractor,
        }
    }

    /// 获取内部的 SessionManager
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// 获取可变的 SessionManager
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }

    /// 关闭会话并自动提取（增强版）
    pub async fn close_session(&mut self, thread_id: &str) -> Result<()> {
        // 先通过SessionManager关闭会话(触发timeline和记忆提取)
        self.session_manager.close_session(thread_id).await?;
        
        info!("Session {} closed with automatic memory extraction via SessionManager", thread_id);
        Ok(())
    }
}
