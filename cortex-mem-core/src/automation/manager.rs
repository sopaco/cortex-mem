use crate::{
    Result,
    automation::AutoIndexer,
    events::{CortexEvent, SessionEvent},
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tracing::{info, warn};

/// 自动化配置
#[derive(Debug, Clone)]
pub struct AutomationConfig {
    /// 是否启用自动索引
    pub auto_index: bool,
    /// 消息添加时是否立即索引（实时）
    pub index_on_message: bool,
    /// 索引批处理延迟（秒）
    pub index_batch_delay: u64,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            auto_index: true,
            index_on_message: false, // 默认批处理模式（性能考虑）
            index_batch_delay: 2,
            max_concurrent_tasks: 3,
        }
    }
}

/// 自动化管理器
///
/// 职责
/// - 监听 `MessageAdded` 事件，将新消息内容（L2 级别）索引到向量数据库
///
/// 不再负责：
/// - 记忆提取（由 `MemoryEventCoordinator` 统一处理）
/// - L0/L1 层级文件生成（由 `CascadeLayerUpdater` 统一处理）
/// - Session 关闭时的全量索引（由 `VectorSyncManager` 统一处理）
pub struct AutomationManager {
    indexer: Arc<AutoIndexer>,
    config: AutomationConfig,
    /// 并发限制信号量
    semaphore: Arc<Semaphore>,
}

impl AutomationManager {
    /// 创建自动化管理器
    pub fn new(indexer: Arc<AutoIndexer>, config: AutomationConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_tasks));
        Self {
            indexer,
            config,
            semaphore,
        }
    }

    /// 获取并发限制信号量（供外部使用）
    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }

    /// 启动自动化任务，监听 EventBus 事件
    pub async fn start(self, mut event_rx: mpsc::UnboundedReceiver<CortexEvent>) -> Result<()> {
        info!("AutomationManager started (L2 message indexing only)");

        // 批处理缓冲区（收集需要索引的 session_id）
        let mut pending_sessions: HashSet<String> = HashSet::new();
        let batch_delay = Duration::from_secs(self.config.index_batch_delay);
        let mut batch_timer: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                // 事件处理
                Some(event) = event_rx.recv() => {
                    if let Err(e) = self.handle_event(
                        event,
                        &mut pending_sessions,
                        &mut batch_timer,
                        batch_delay,
                    ).await {
                        warn!("AutomationManager: failed to handle event: {}", e);
                    }
                }

                // 批处理定时器触发
                _ = async {
                    if let Some(deadline) = batch_timer {
                        tokio::time::sleep_until(deadline).await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    if !pending_sessions.is_empty() {
                        if let Err(e) = self.flush_batch(&mut pending_sessions).await {
                            warn!("AutomationManager: failed to flush batch: {}", e);
                        }
                        batch_timer = None;
                    }
                }
            }
        }
    }

    /// 处理事件 — 仅关心 MessageAdded（L2 索引）
    async fn handle_event(
        &self,
        event: CortexEvent,
        pending_sessions: &mut HashSet<String>,
        batch_timer: &mut Option<tokio::time::Instant>,
        batch_delay: Duration,
    ) -> Result<()> {
        match event {
            CortexEvent::Session(SessionEvent::MessageAdded { session_id, .. }) => {
                if !self.config.auto_index {
                    return Ok(());
                }

                if self.config.index_on_message {
                    // 实时索引模式：立即索引本 session 的 L2 消息
                    info!("AutomationManager: real-time L2 indexing for session {}", session_id);
                    self.index_session_l2(&session_id).await?;
                } else {
                    // 批处理模式：加入待处理队列
                    pending_sessions.insert(session_id);
                    if batch_timer.is_none() {
                        *batch_timer = Some(tokio::time::Instant::now() + batch_delay);
                    }
                }
            }

            // Session 关闭由 MemoryEventCoordinator 全权处理，此处忽略
            CortexEvent::Session(SessionEvent::Closed { .. }) => {}

            _ => {} // 其他事件忽略
        }

        Ok(())
    }

    /// 批量处理待索引的 session
    async fn flush_batch(&self, pending_sessions: &mut HashSet<String>) -> Result<()> {
        info!("AutomationManager: flushing {} sessions", pending_sessions.len());
        for session_id in pending_sessions.drain() {
            if let Err(e) = self.index_session_l2(&session_id).await {
                warn!("AutomationManager: failed to index session {}: {}", session_id, e);
            }
        }
        Ok(())
    }

    /// 索引单个 session 的 L2 消息内容
    async fn index_session_l2(&self, session_id: &str) -> Result<()> {
        let _permit = self.semaphore.acquire().await;
        match self.indexer.index_thread(session_id).await {
            Ok(stats) => {
                info!(
                    "AutomationManager: session {} L2 indexed ({} indexed, {} skipped, {} errors)",
                    session_id, stats.total_indexed, stats.total_skipped, stats.total_errors
                );
                Ok(())
            }
            Err(e) => {
                warn!("AutomationManager: failed to index session {}: {}", session_id, e);
                Err(e)
            }
        }
    }
}
