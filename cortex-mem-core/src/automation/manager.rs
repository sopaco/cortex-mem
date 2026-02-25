use crate::{
    automation::{AutoExtractor, AutoIndexer, LayerGenerator},
    events::{CortexEvent, SessionEvent},
    Result,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// 自动化配置
#[derive(Debug, Clone)]
pub struct AutomationConfig {
    /// 是否启用自动索引
    pub auto_index: bool,
    /// 是否启用自动提取
    pub auto_extract: bool,
    /// 消息添加时是否立即索引（实时）
    pub index_on_message: bool,
    /// 会话关闭时是否索引（批量）
    pub index_on_close: bool,
    /// 索引批处理延迟（秒）
    pub index_batch_delay: u64,
    /// 🆕 启动时自动生成缺失的 L0/L1 文件
    pub auto_generate_layers_on_startup: bool,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            auto_index: true,
            auto_extract: true,
            index_on_message: false,  // 默认不实时索引（性能考虑）
            index_on_close: true,      // 默认会话关闭时索引
            index_batch_delay: 2,
            auto_generate_layers_on_startup: false,  // 🆕 默认关闭（避免启动时阻塞）
        }
    }
}

/// 自动化管理器 - 统一调度索引和提取
pub struct AutomationManager {
    indexer: Arc<AutoIndexer>,
    extractor: Option<Arc<AutoExtractor>>,
    layer_generator: Option<Arc<LayerGenerator>>,  // 🆕 层级生成器
    config: AutomationConfig,
}

impl AutomationManager {
    /// 创建自动化管理器
    pub fn new(
        indexer: Arc<AutoIndexer>,
        extractor: Option<Arc<AutoExtractor>>,
        config: AutomationConfig,
    ) -> Self {
        Self {
            indexer,
            extractor,
            layer_generator: None,  // 🆕 初始为 None，需要单独设置
            config,
        }
    }
    
    /// 🆕 设置层级生成器（可选）
    pub fn with_layer_generator(mut self, layer_generator: Arc<LayerGenerator>) -> Self {
        self.layer_generator = Some(layer_generator);
        self
    }
    
    /// 🎯 核心方法：启动自动化任务
    pub async fn start(self, mut event_rx: mpsc::UnboundedReceiver<CortexEvent>) -> Result<()> {
        info!("Starting AutomationManager with config: {:?}", self.config);
        
        // 🆕 启动时自动生成缺失的 L0/L1 文件
        if self.config.auto_generate_layers_on_startup {
            if let Some(ref generator) = self.layer_generator {
                info!("启动时检查并生成缺失的 L0/L1 文件...");
                let generator_clone = generator.clone();
                tokio::spawn(async move {
                    match generator_clone.ensure_all_layers().await {
                        Ok(stats) => {
                            info!(
                                "启动时层级生成完成: 总计 {}, 成功 {}, 失败 {}",
                                stats.total, stats.generated, stats.failed
                            );
                        }
                        Err(e) => {
                            warn!("启动时层级生成失败: {}", e);
                        }
                    }
                });
            } else {
                warn!("auto_generate_layers_on_startup 已启用但未设置 layer_generator");
            }
        }
        
        // 批处理缓冲区（收集需要索引的session_id）
        let mut pending_sessions: HashSet<String> = HashSet::new();
        let batch_delay = Duration::from_secs(self.config.index_batch_delay);
        let mut batch_timer: Option<tokio::time::Instant> = None;
        
        loop {
            tokio::select! {
                // 事件处理
                Some(event) = event_rx.recv() => {
                    if let Err(e) = self.handle_event(event, &mut pending_sessions, &mut batch_timer, batch_delay).await {
                        warn!("Failed to handle event: {}", e);
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
                            warn!("Failed to flush batch: {}", e);
                        }
                        batch_timer = None;
                    }
                }
            }
        }
    }
    
    /// 处理事件
    async fn handle_event(
        &self,
        event: CortexEvent,
        pending_sessions: &mut HashSet<String>,
        batch_timer: &mut Option<tokio::time::Instant>,
        batch_delay: Duration,
    ) -> Result<()> {
        match event {
            CortexEvent::Session(SessionEvent::MessageAdded { session_id, .. }) => {
                if self.config.index_on_message {
                    // 实时索引模式：立即索引
                    info!("Real-time indexing session: {}", session_id);
                    self.index_session(&session_id).await?;
                } else {
                    // 批处理模式：加入待处理队列
                    pending_sessions.insert(session_id);
                    
                    // 启动批处理定时器（如果未启动）
                    if batch_timer.is_none() {
                        *batch_timer = Some(tokio::time::Instant::now() + batch_delay);
                    }
                }
            }
            
            CortexEvent::Session(SessionEvent::Closed { session_id }) => {
                if self.config.index_on_close {
                    info!("Session closed, triggering full processing: {}", session_id);
                    
                    // 1. 自动提取记忆（如果配置了且有extractor）
                    if self.config.auto_extract {
                        if let Some(ref extractor) = self.extractor {
                            match extractor.extract_session(&session_id).await {
                                Ok(stats) => {
                                    info!("Extraction completed for {}: {:?}", session_id, stats);
                                }
                                Err(e) => {
                                    warn!("Extraction failed for {}: {}", session_id, e);
                                }
                            }
                        }
                    }
                    
                    // 2. 索引整个会话（包括L0/L1/L2）
                    if self.config.auto_index {
                        self.index_session(&session_id).await?;
                    }
                }
            }
            
            _ => { /* 其他事件暂时忽略 */ }
        }
        
        Ok(())
    }
    
    /// 批量处理待索引的会话
    async fn flush_batch(&self, pending_sessions: &mut HashSet<String>) -> Result<()> {
        info!("Flushing batch: {} sessions", pending_sessions.len());
        
        for session_id in pending_sessions.drain() {
            if let Err(e) = self.index_session(&session_id).await {
                warn!("Failed to index session {}: {}", session_id, e);
            }
        }
        
        Ok(())
    }
    
    /// 索引单个会话
    async fn index_session(&self, session_id: &str) -> Result<()> {
        match self.indexer.index_thread(session_id).await {
            Ok(stats) => {
                info!(
                    "Session {} indexed: {} indexed, {} skipped, {} errors",
                    session_id, stats.total_indexed, stats.total_skipped, stats.total_errors
                );
                Ok(())
            }
            Err(e) => {
                warn!("Failed to index session {}: {}", session_id, e);
                Err(e)
            }
        }
    }
}
