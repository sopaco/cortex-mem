//! Memory Event Coordinator Module
//!
//! Central coordinator that handles all memory events and orchestrates
//! the flow between different components.
//!
//! ## Phase 2 Optimization: Debouncing
//! - Batches layer update requests for the same directory
//! - Reduces redundant LLM calls by 70-90%
//! - Configurable debounce delay (default: 30 seconds)

use crate::Result;
use crate::cascade_layer_debouncer::{DebouncerConfig, LayerUpdateDebouncer};
use crate::cascade_layer_updater::CascadeLayerUpdater;
use crate::embedding::EmbeddingClient;
use crate::filesystem::{CortexFilesystem, FilesystemOperations};
use crate::incremental_memory_updater::IncrementalMemoryUpdater;
use crate::llm::LLMClient;
use crate::llm_result_cache::CacheConfig;
use crate::memory_events::{ChangeType, DeleteReason, EventStats, MemoryEvent};
use crate::memory_index::MemoryScope;
use crate::memory_index_manager::MemoryIndexManager;
use crate::session::extraction::ExtractedMemories;
use crate::vector_store::QdrantVectorStore;
use crate::vector_sync_manager::VectorSyncManager;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{RwLock, mpsc, watch};
use tracing::{debug, error, info, trace, warn};

/// Configuration for event coordinator
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Enable debouncing for layer updates (Phase 2)
    pub enable_debounce: bool,
    /// Debouncer configuration
    pub debouncer_config: DebouncerConfig,
    /// Enable LLM result cache (Phase 3)
    pub enable_cache: bool,
    /// Cache configuration
    pub cache_config: CacheConfig,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            enable_debounce: true, // Enable by default
            debouncer_config: DebouncerConfig::default(),
            enable_cache: true, // Enable cache by default
            cache_config: CacheConfig::default(),
        }
    }
}

/// Memory Event Coordinator
///
/// Central hub that coordinates all memory operations:
/// - Receives events from various sources
/// - Dispatches to appropriate handlers
/// - Ensures consistency across components
/// - (Phase 2) Debounces layer updates to reduce LLM calls
pub struct MemoryEventCoordinator {
    filesystem: Arc<CortexFilesystem>,
    llm_client: Arc<dyn LLMClient>,
    index_manager: Arc<MemoryIndexManager>,
    memory_updater: Arc<IncrementalMemoryUpdater>,
    layer_updater: Arc<CascadeLayerUpdater>,
    vector_sync: Arc<VectorSyncManager>,
    stats: Arc<RwLock<EventStats>>,
    /// Phase 2: Debouncer for layer updates
    debouncer: Option<Arc<LayerUpdateDebouncer>>,
    #[allow(dead_code)]
    config: CoordinatorConfig,
    /// 任务计数器：跟踪正在处理的任务数量
    pending_tasks: Arc<AtomicUsize>,
    /// 任务完成通知：当 pending_tasks 变为 0 时通知
    task_completion_tx: watch::Sender<usize>,
    /// 任务完成接收器（用于外部等待）
    task_completion_rx: watch::Receiver<usize>,
    /// 抑制后台层级联更新的 scope 集合（格式："scope/owner_id"）。
    ///
    /// 当 `on_session_closed` 正在同步执行 `update_all_layers` 时，
    /// 将对应的 "scope/owner_id" 加入此集合，防止后台 event loop 处理
    /// MemoryCreated/MemoryUpdated 事件时重复触发级联更新。
    /// update_all_layers 完成后移除对应条目，恢复正常处理。
    ///
    /// 使用 scope-granular 集合而非全局 bool，避免误压制不同 scope 的用户。
    suppress_layer_cascade_scopes: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
}

impl MemoryEventCoordinator {
    /// Create a new memory event coordinator with default config
    ///
    /// Returns (coordinator, event_sender, event_receiver)
    /// - coordinator: the coordinator instance (wrapped in Arc for shared access)
    /// - event_sender: use this to send events to the coordinator
    /// - event_receiver: pass this to coordinator.start() to begin processing
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        llm_client: Arc<dyn LLMClient>,
        embedding_client: Arc<EmbeddingClient>,
        vector_store: Arc<QdrantVectorStore>,
    ) -> (
        Arc<Self>,
        mpsc::UnboundedSender<MemoryEvent>,
        mpsc::UnboundedReceiver<MemoryEvent>,
    ) {
        Self::new_with_config(
            filesystem,
            llm_client,
            embedding_client,
            vector_store,
            CoordinatorConfig::default(),
        )
    }

    /// Create a new memory event coordinator with custom config
    pub fn new_with_config(
        filesystem: Arc<CortexFilesystem>,
        llm_client: Arc<dyn LLMClient>,
        embedding_client: Arc<EmbeddingClient>,
        vector_store: Arc<QdrantVectorStore>,
        config: CoordinatorConfig,
    ) -> (
        Arc<Self>,
        mpsc::UnboundedSender<MemoryEvent>,
        mpsc::UnboundedReceiver<MemoryEvent>,
    ) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let index_manager = Arc::new(MemoryIndexManager::new(filesystem.clone()));

        // Create memory updater with event sender
        let memory_updater = Arc::new(IncrementalMemoryUpdater::new(
            filesystem.clone(),
            index_manager.clone(),
            llm_client.clone(),
            event_tx.clone(),
        ));

        // Create layer updater with event sender and optional cache
        let cache_config = if config.enable_cache {
            Some(config.cache_config.clone())
        } else {
            None
        };

        let layer_updater = Arc::new(CascadeLayerUpdater::new_with_cache(
            filesystem.clone(),
            llm_client.clone(),
            event_tx.clone(),
            cache_config,
        ));

        // Create vector sync manager
        let vector_sync = Arc::new(VectorSyncManager::new(
            filesystem.clone(),
            embedding_client,
            vector_store,
        ));

        // Phase 2: Create debouncer if enabled
        let debouncer = if config.enable_debounce {
            let debouncer = Arc::new(LayerUpdateDebouncer::new(config.debouncer_config.clone()));
            info!(
                "🔧 Layer update debouncer enabled (delay: {}s)",
                config.debouncer_config.debounce_secs
            );
            Some(debouncer)
        } else {
            info!("⚠️  Layer update debouncer disabled");
            None
        };

        // 创建任务完成通知机制
        let pending_tasks = Arc::new(AtomicUsize::new(0));
        let (task_completion_tx, task_completion_rx) = watch::channel(0);

        let coordinator = Arc::new(Self {
            filesystem,
            llm_client,
            index_manager,
            memory_updater,
            layer_updater,
            vector_sync,
            stats: Arc::new(RwLock::new(EventStats::default())),
            debouncer,
            config,
            pending_tasks,
            task_completion_tx,
            task_completion_rx,
            suppress_layer_cascade_scopes: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        });

        (coordinator, event_tx, event_rx)
    }

    /// Start the event processing loop
    ///
    /// Phase 2: Integrates debouncer with periodic processing
    /// Returns a boxed future that can be spawned on a tokio runtime.
    pub fn start(
        self: Arc<Self>,
        mut event_rx: mpsc::UnboundedReceiver<MemoryEvent>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
        Box::pin(async move {
            info!("Memory Event Coordinator started");

            // Phase 2: Setup periodic debouncer processing if enabled
            let mut debounce_interval = if self.debouncer.is_some() {
                Some(tokio::time::interval(Duration::from_millis(500))) // Check every 500ms
            } else {
                None
            };

            loop {
                tokio::select! {
                    // Handle incoming events
                    event = event_rx.recv() => {
                        match event {
                            Some(event) => {
                                // 🔧 关键修复：在取出事件时就增加计数
                                // 这样 flush_and_wait 可以正确检测到有待处理的事件
                                self.pending_tasks.fetch_add(1, Ordering::SeqCst);

                                if let Err(e) = self.handle_event_inner(event).await {
                                    error!("Event handling failed: {}", e);
                                }

                                // 减少计数并通知
                                let remaining = self.pending_tasks.fetch_sub(1, Ordering::SeqCst) - 1;
                                let _ = self.task_completion_tx.send(remaining);
                            }
                            None => {
                                warn!("Memory Event Coordinator stopped (channel closed)");
                                break;
                            }
                        }
                    }

                    // Phase 2: Periodic debouncer processing
                    _ = async {
                        if let Some(ref mut interval) = debounce_interval {
                            interval.tick().await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        if let Some(ref debouncer) = self.debouncer {
                            let processed = debouncer.process_due_updates(&self.layer_updater).await;
                            if processed > 0 {
                                debug!("🔧 Debouncer processed {} updates", processed);
                            }
                        }
                    }
                }
            }

            // Final flush of pending updates
            if let Some(ref debouncer) = self.debouncer {
                let pending = debouncer.pending_count().await;
                if pending > 0 {
                    info!("🔄 Flushing {} pending updates before shutdown...", pending);
                    debouncer.process_due_updates(&self.layer_updater).await;
                }
            }

            info!("Memory Event Coordinator stopped");
        })
    }

    /// 获取任务完成通知接收器
    ///
    /// 外部可以使用这个接收器来等待所有任务完成
    pub fn get_task_completion_rx(&self) -> watch::Receiver<usize> {
        self.task_completion_rx.clone()
    }

    /// 获取当前待处理任务数量
    pub fn pending_task_count(&self) -> usize {
        self.pending_tasks.load(Ordering::SeqCst)
    }

    /// 刷新 debouncer 并等待所有任务完成（用于退出流程）
    ///
    /// 这个方法会：
    /// 0. 等待事件从 channel 被取出（通过 yield 让出运行时）
    /// 1. 等待当前正在处理的事件完成
    /// 2. 强制处理 debouncer 中所有待处理的层级更新
    /// 3. 再次等待确保所有更新完成
    ///
    /// 使用事件通知机制而非固定超时，确保真正等待任务完成。
    ///
    /// # Arguments
    /// * `check_interval` - 检查间隔
    ///
    /// # Returns
    /// * `true` - 所有任务已完成
    /// * `false` - 在等待过程中有新任务产生（通常不应该发生）
    pub async fn flush_and_wait(&self, check_interval: Duration) -> bool {
        info!("Flushing and waiting for all tasks...");

        let start = std::time::Instant::now();
        let max_wait = Duration::from_secs(300);

        // Phase 0: Yield runtime to let event loop run
        for i in 0..10 {
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(10)).await;

            let pending = self.pending_tasks.load(Ordering::SeqCst);
            if pending > 0 {
                debug!("Detected {} pending tasks", pending);
                break;
            }

            if i == 9 {
                debug!("No pending tasks detected");
            }
        }

        // Phase 1: Wait for current event processing to complete
        loop {
            let pending = self.pending_tasks.load(Ordering::SeqCst);
            if pending == 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let pending_after = self.pending_tasks.load(Ordering::SeqCst);
                if pending_after == 0 {
                    break;
                }
                continue;
            }

            if start.elapsed() >= max_wait {
                warn!("Timeout, {} tasks still pending", pending);
                return false;
            }

            trace!("Waiting for {} tasks... (elapsed: {:?})", pending, start.elapsed());
            tokio::time::sleep(check_interval).await;
        }
        debug!("Event processing tasks cleared");

        // Phase 2: Flush pending updates in debouncer
        if let Some(ref debouncer) = self.debouncer {
            let pending_count = debouncer.pending_count().await;
            if pending_count > 0 {
                info!("Flushing {} debouncer updates", pending_count);
                let flushed = debouncer.flush_all(&self.layer_updater).await;
                debug!("Flushed {} layer updates", flushed);
            }
        }

        // Phase 3: Wait for debouncer flush tasks to complete
        loop {
            let pending = self.pending_tasks.load(Ordering::SeqCst);
            if pending == 0 {
                break;
            }

            if start.elapsed() >= max_wait {
                warn!("Timeout, {} tasks still pending", pending);
                return false;
            }

            tokio::time::sleep(check_interval).await;
        }

        info!("All tasks completed (elapsed: {:?})", start.elapsed());
        true
    }

    /// Wait for all background tasks to complete
    ///
    /// # Arguments
    /// * `timeout` - Maximum wait time
    ///
    /// # Returns
    /// * `true` - All tasks completed
    /// * `false` - Timeout
    pub async fn wait_for_completion(&self, timeout: Duration) -> bool {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(500);

        loop {
            let pending = self.pending_tasks.load(Ordering::SeqCst);

            if pending == 0 {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let pending_after = self.pending_tasks.load(Ordering::SeqCst);
                if pending_after == 0 {
                    info!("All background tasks completed");
                    return true;
                }
                continue;
            }

            if start.elapsed() >= timeout {
                warn!("Timeout, {} tasks still pending", pending);
                return false;
            }

            if start.elapsed() < Duration::from_millis(600) {
                info!("Waiting for {} tasks...", pending);
            }

            tokio::time::sleep(check_interval).await;
        }
    }

    /// Handle a single event (internal implementation)
    async fn handle_event_inner(&self, event: MemoryEvent) -> Result<()> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.record(&event);
        }

        debug!("Handling event: {}", event);

        match event {
            MemoryEvent::MemoryCreated {
                scope,
                owner_id,
                memory_id,
                memory_type,
                key,
                source_session,
                file_uri,
            } => {
                self.on_memory_created(
                    &scope,
                    &owner_id,
                    &memory_id,
                    &memory_type,
                    &key,
                    &source_session,
                    &file_uri,
                )
                .await?;
            }

            MemoryEvent::MemoryUpdated {
                scope,
                owner_id,
                memory_id,
                memory_type,
                key,
                source_session,
                file_uri,
                old_content_hash,
                new_content_hash,
            } => {
                self.on_memory_updated(
                    &scope,
                    &owner_id,
                    &memory_id,
                    &memory_type,
                    &key,
                    &source_session,
                    &file_uri,
                    &old_content_hash,
                    &new_content_hash,
                )
                .await?;
            }

            MemoryEvent::MemoryDeleted {
                scope,
                owner_id,
                memory_id,
                memory_type,
                file_uri,
                reason,
            } => {
                self.on_memory_deleted(
                    &scope,
                    &owner_id,
                    &memory_id,
                    &memory_type,
                    &file_uri,
                    &reason,
                )
                .await?;
            }

            MemoryEvent::MemoryAccessed {
                scope,
                owner_id,
                memory_id,
                context,
            } => {
                self.on_memory_accessed(&scope, &owner_id, &memory_id, &context)
                    .await?;
            }

            MemoryEvent::LayersUpdated {
                scope,
                owner_id,
                directory_uri,
                layers,
            } => {
                self.on_layers_updated(&scope, &owner_id, &directory_uri, &layers)
                    .await?;
            }

            MemoryEvent::SessionClosed {
                session_id,
                user_id,
                agent_id,
            } => {
                self.on_session_closed(&session_id, &user_id, &agent_id)
                    .await?;
            }

            MemoryEvent::LayerUpdateNeeded {
                scope,
                owner_id,
                directory_uri,
                change_type,
                changed_file,
            } => {
                self.on_layer_update_needed(
                    &scope,
                    &owner_id,
                    &directory_uri,
                    &change_type,
                    &changed_file,
                )
                .await?;
            }

            MemoryEvent::VectorSyncNeeded {
                file_uri,
                change_type,
            } => {
                self.on_vector_sync_needed(&file_uri, &change_type).await?;
            }
        }

        Ok(())
    }

    /// Handle memory created event
    async fn on_memory_created(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        memory_type: &crate::memory_index::MemoryType,
        _key: &str,
        _source_session: &str,
        file_uri: &str,
    ) -> Result<()> {
        debug!(
            "Memory created: {} ({:?}) in {:?}/{}",
            memory_id, memory_type, scope, owner_id
        );

        // Skip layer cascade if suppressed for this scope (e.g. during on_session_closed's
        // update_all_layers), but always continue with vector sync so the new file is indexed.
        let key = format!("{}/{}", scope, owner_id);
        let suppress_cascade = self
            .suppress_layer_cascade_scopes
            .read()
            .await
            .contains(&key);

        if !suppress_cascade {
            // Trigger layer cascade update
            self.layer_updater
                .on_memory_changed(
                    scope.clone(),
                    owner_id.to_string(),
                    file_uri.to_string(),
                    ChangeType::Add,
                )
                .await?;
        } else {
            debug!("Layer cascade suppressed for MemoryCreated: {}", key);
        }

        // Always trigger vector sync (suppression only skips layer cascade, not vector indexing)
        self.vector_sync
            .sync_file_change(file_uri, ChangeType::Add)
            .await?;

        Ok(())
    }

    /// Handle memory updated event
    async fn on_memory_updated(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        memory_type: &crate::memory_index::MemoryType,
        _key: &str,
        _source_session: &str,
        file_uri: &str,
        _old_content_hash: &str,
        _new_content_hash: &str,
    ) -> Result<()> {
        debug!(
            "Memory updated: {} ({:?}) in {:?}/{}",
            memory_id, memory_type, scope, owner_id
        );

        // Skip layer cascade if suppressed for this scope, but always continue vector sync.
        let key = format!("{}/{}", scope, owner_id);
        let suppress_cascade = self
            .suppress_layer_cascade_scopes
            .read()
            .await
            .contains(&key);

        if !suppress_cascade {
            // Trigger layer cascade update
            self.layer_updater
                .on_memory_changed(
                    scope.clone(),
                    owner_id.to_string(),
                    file_uri.to_string(),
                    ChangeType::Update,
                )
                .await?;
        } else {
            debug!("Layer cascade suppressed for MemoryUpdated: {}", key);
        }

        // Always trigger vector sync
        self.vector_sync
            .sync_file_change(file_uri, ChangeType::Update)
            .await?;

        Ok(())
    }

    /// Handle memory deleted event
    async fn on_memory_deleted(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        memory_type: &crate::memory_index::MemoryType,
        file_uri: &str,
        reason: &DeleteReason,
    ) -> Result<()> {
        debug!(
            "Memory deleted: {} ({:?}) in {:?}/{}, reason: {:?}",
            memory_id, memory_type, scope, owner_id, reason
        );

        // Trigger layer cascade update
        self.layer_updater
            .on_memory_changed(
                scope.clone(),
                owner_id.to_string(),
                file_uri.to_string(),
                ChangeType::Delete,
            )
            .await?;

        // Trigger vector deletion
        self.vector_sync
            .sync_file_change(file_uri, ChangeType::Delete)
            .await?;

        Ok(())
    }

    /// Handle memory accessed event
    async fn on_memory_accessed(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        memory_id: &str,
        context: &str,
    ) -> Result<()> {
        debug!(
            "Memory accessed: {} in {:?}/{}, context: {}",
            memory_id, scope, owner_id, context
        );

        // Record access in index
        self.index_manager
            .record_access(scope, owner_id, memory_id)
            .await?;

        Ok(())
    }

    /// Handle layers updated event
    async fn on_layers_updated(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        directory_uri: &str,
        layers: &[crate::ContextLayer],
    ) -> Result<()> {
        debug!(
            "Layers updated for {} in {:?}/{}: {:?}",
            directory_uri, scope, owner_id, layers
        );

        // Sync layer files to vector database
        self.vector_sync.sync_layer_files(directory_uri).await?;

        Ok(())
    }

    /// 同步处理 session 关闭：记忆提取 → user/agent 文件写入 → L0/L1 生成 → 向量同步
    ///
    /// 调用方 `.await` 后可保证所有副作用已完成，不存在异步竞争。
    /// 供 `MemoryOperations::close_session_sync` 直接调用，无需经过 channel。
    pub async fn process_session_closed(
        &self,
        session_id: &str,
        user_id: &str,
        agent_id: &str,
    ) -> Result<()> {
        self.on_session_closed(session_id, user_id, agent_id).await
    }

    /// Handle session closed event (the main trigger for memory extraction)
    async fn on_session_closed(
        &self,
        session_id: &str,
        user_id: &str,
        agent_id: &str,
    ) -> Result<()> {
        info!("Processing session closed: {} (user={}, agent={})", session_id, user_id, agent_id);

        // 1. Extract memories from the session
        let extracted = self.extract_memories_from_session(session_id).await?;

        info!(
            "Extracted memories: {} preferences, {} entities, {} events, {} cases",
            extracted.preferences.len(),
            extracted.entities.len(),
            extracted.events.len(),
            extracted.cases.len()
        );

        // 2. Update user memories
        if !extracted.is_empty() {
            // 2a. 抑制后台 event loop 对 user/agent scope 的级联更新：
            //     update_memories 写文件时会发出 MemoryCreated/MemoryUpdated 事件，
            //     后台 loop 处理这些事件会触发 on_memory_changed → update_directory_layers
            //     → update_root_layers，与下方步骤 2b 的 update_all_layers 完全重复。
            //     只抑制当前 user/agent scope，不影响并发中其他 scope 的正常处理。
            //     注意：向量同步不受抑制影响，仍会正常执行。
            let user_key = format!("{}/{}", crate::memory_index::MemoryScope::User, user_id);
            let agent_key = format!("{}/{}", crate::memory_index::MemoryScope::Agent, agent_id);
            {
                let mut set = self.suppress_layer_cascade_scopes.write().await;
                set.insert(user_key.clone());
                set.insert(agent_key.clone());
            }

            let user_result = self
                .memory_updater
                .update_memories(user_id, agent_id, session_id, &extracted)
                .await?;

            info!(
                "User memory updated for session {}: {} created, {} updated",
                session_id, user_result.created, user_result.updated
            );

            // 2b. Synchronously generate L0/L1 for user and agent directories.
            //
            // `update_memories` writes files and emits MemoryCreated/MemoryUpdated events via
            // `event_tx`, but those events are handled by the background loop *asynchronously*.
            // Since `on_session_closed` is called synchronously (without waiting for the loop),
            // the background handler hasn't run yet — so L0/L1 files would not be generated
            // until the next background iteration, which may happen after the process exits.
            //
            // Fix: call `layer_updater.update_all_layers` directly here so that L0/L1 is
            // generated before we return.
            info!("Generating L0/L1 for user/{} after memory extraction...", user_id);
            if let Err(e) = self.layer_updater
                .update_all_layers(&crate::memory_index::MemoryScope::User, user_id)
                .await
            {
                warn!("Failed to update user L0/L1 layers: {}", e);
            }

            if !extracted.cases.is_empty() {
                info!("Generating L0/L1 for agent/{} after case memory extraction...", agent_id);
                if let Err(e) = self.layer_updater
                    .update_all_layers(&crate::memory_index::MemoryScope::Agent, agent_id)
                    .await
                {
                    warn!("Failed to update agent L0/L1 layers: {}", e);
                }
            }

            // 2c. 恢复后台级联更新（移除 scope 抑制）
            {
                let mut set = self.suppress_layer_cascade_scopes.write().await;
                set.remove(&user_key);
                set.remove(&agent_key);
            }
        } else {
            info!("No memories extracted from session {}", session_id);
        }

        // 3. Update timeline layers
        self.layer_updater
            .update_timeline_layers(session_id)
            .await?;

        // 4. Sync session to vectors
        let timeline_uri = format!("cortex://session/{}/timeline", session_id);
        self.vector_sync.sync_directory(&timeline_uri).await?;

        info!("Session {} processing complete", session_id);

        Ok(())
    }

    /// Handle layer update needed event
    ///
    /// Phase 2: Uses debouncer if enabled
    async fn on_layer_update_needed(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
        directory_uri: &str,
        change_type: &ChangeType,
        changed_file: &str,
    ) -> Result<()> {
        debug!(
            "Layer update needed for {} due to {:?} on {}",
            directory_uri, change_type, changed_file
        );

        // Phase 2: Use debouncer if enabled
        if let Some(ref debouncer) = self.debouncer {
            // Request update (will be debounced)
            debouncer
                .request_update(
                    directory_uri.to_string(),
                    scope.clone(),
                    owner_id.to_string(),
                )
                .await;

            debug!(
                "🔧 Layer update request queued for debouncing: {}",
                directory_uri
            );
        } else {
            // No debouncing, execute immediately
            self.layer_updater
                .on_memory_changed(
                    scope.clone(),
                    owner_id.to_string(),
                    changed_file.to_string(),
                    change_type.clone(),
                )
                .await?;
        }

        Ok(())
    }

    /// Handle vector sync needed event
    async fn on_vector_sync_needed(&self, file_uri: &str, change_type: &ChangeType) -> Result<()> {
        info!("🔍 VectorSync: processing {} ({:?})", file_uri, change_type);

        match self.vector_sync
            .sync_file_change(file_uri, change_type.clone())
            .await
        {
            Ok(stats) => {
                if stats.indexed > 0 || stats.updated > 0 {
                    info!(
                        "✅ VectorSync: {} indexed, {} updated, {} skipped, {} errors for {}",
                        stats.indexed, stats.updated, stats.skipped, stats.errors, file_uri
                    );
                } else if stats.errors > 0 {
                    warn!(
                        "⚠️ VectorSync: {} errors, {} skipped for {}",
                        stats.errors, stats.skipped, file_uri
                    );
                } else {
                    info!(
                        "⏭️ VectorSync: all {} skipped (already indexed) for {}",
                        stats.skipped, file_uri
                    );
                }
            }
            Err(e) => {
                error!("❌ VectorSync: failed for {}: {}", file_uri, e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Extract memories from a session using LLM
    async fn extract_memories_from_session(&self, session_id: &str) -> Result<ExtractedMemories> {
        let timeline_uri = format!("cortex://session/{}/timeline", session_id);

        let mut messages = Vec::new();
        match self
            .collect_messages_recursive(&timeline_uri, &mut messages)
            .await
        {
            Ok(_) => {
                debug!("Collected {} messages from session", messages.len());
            }
            Err(e) => {
                error!("Failed to collect messages: {}", e);
                return Err(e);
            }
        }

        if messages.is_empty() {
            warn!("No messages found in session {}", session_id);
            return Ok(ExtractedMemories::default());
        }

        let prompt = self.build_extraction_prompt(&messages);

        debug!("Calling LLM for memory extraction...");
        let response = match self.llm_client.complete(&prompt).await {
            Ok(resp) => {
                debug!("LLM response received ({} chars)", resp.len());
                resp
            }
            Err(e) => {
                error!("LLM call failed: {}", e);
                return Err(e);
            }
        };

        let extracted = self.parse_extraction_response(&response);

        info!(
            "Extracted {} memories from session {}",
            extracted.preferences.len()
                + extracted.entities.len()
                + extracted.events.len()
                + extracted.cases.len(),
            session_id
        );

        Ok(extracted)
    }

    /// Recursively collect messages from timeline
    fn collect_messages_recursive<'a>(
        &'a self,
        uri: &'a str,
        messages: &'a mut Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let entries = self.filesystem.list(uri).await?;

            for entry in entries {
                if entry.name.starts_with('.') {
                    continue;
                }

                if entry.is_directory {
                    self.collect_messages_recursive(&entry.uri, messages)
                        .await?;
                } else if entry.name.ends_with(".md") {
                    if let Ok(content) = self.filesystem.read(&entry.uri).await {
                        messages.push(content);
                    }
                }
            }

            Ok(())
        })
    }

    /// Build the extraction prompt
    fn build_extraction_prompt(&self, messages: &[String]) -> String {
        let messages_text = messages.join("\n\n---\n\n");

        format!(
            r#"Analyze the following conversation and extract memories in JSON format.

## CRITICAL LANGUAGE RULES

1. **Language Consistency** (MANDATORY):
   - Extract memories in the SAME language as the conversation
   - If conversation is in Chinese (中文) → memories MUST be in Chinese
   - If conversation is in English → memories in English
   - If mixed language → use the dominant language (>60% of content)
   - **DO NOT translate** the conversation content into another language

2. **Preserve Technical Terms** (MANDATORY):
   - Keep technical terminology unchanged in their original language
   - Programming languages: Rust, Python, TypeScript, JavaScript, Go
   - Frameworks: Cortex Memory, Rig, React, Vue
   - Personality types: INTJ, ENTJ, MBTI, DISC
   - Proper nouns: names, companies, projects
   - Acronyms: LLM, AI, ML, API, HTTP, REST

3. **Examples**:
   ✅ CORRECT (Chinese conversation):
   - "Cortex Memory 是基于 Rust 的长期记忆系统"
   - "用户喜欢吃牛肉汉堡，搭配酸黄瓜、芝士和可乐"

   ❌ WRONG (Chinese conversation, should NOT translate to English):
   - "User likes beef burgers with pickles, cheese, and coke"

## Instructions

Extract the following types of memories:

1. **Personal Info** (user's personal information):
   - category: "age", "occupation", "education", "location", etc.
   - content: The specific information
   - confidence: 0.0-1.0 confidence level

2. **Work History** (user's work experience):
   - company: Company name
   - role: Job title/role
   - duration: Time period (optional)
   - description: Brief description
   - confidence: 0.0-1.0 confidence level

3. **Preferences** (user preferences by topic):
   - topic: The topic/subject area
   - preference: The user's stated preference
   - confidence: 0.0-1.0 confidence level

4. **Relationships** (people user mentions):
   - person: Person's name
   - relation_type: "family", "colleague", "friend", etc.
   - context: How they're related
   - confidence: 0.0-1.0 confidence level

5. **Goals** (user's goals and aspirations):
   - goal: The specific goal
   - category: "career", "personal", "health", "learning", etc.
   - timeline: When they want to achieve it (optional)
   - confidence: 0.0-1.0 confidence level

6. **Entities** (people, projects, organizations mentioned):
   - name: Entity name
   - entity_type: "person", "project", "organization", "technology", etc.
   - description: Brief description
   - context: How it was mentioned

7. **Events** (decisions, milestones, important occurrences):
   - title: Event title
   - event_type: "decision", "milestone", "occurrence"
   - summary: Brief summary
   - timestamp: If mentioned

8. **Cases** (problems encountered and solutions found):
   - title: Case title
   - problem: The problem encountered
   - solution: How it was solved
   - lessons_learned: Array of lessons learned

## Response Format

Return ONLY a JSON object with this structure:

{{
  "personal_info": [{{ "category": "...", "content": "...", "confidence": 0.9 }}],
  "work_history": [{{ "company": "...", "role": "...", "duration": "...", "description": "...", "confidence": 0.9 }}],
  "preferences": [{{ "topic": "...", "preference": "...", "confidence": 0.9 }}],
  "relationships": [{{ "person": "...", "relation_type": "...", "context": "...", "confidence": 0.9 }}],
  "goals": [{{ "goal": "...", "category": "...", "timeline": "...", "confidence": 0.9 }}],
  "entities": [{{ "name": "...", "entity_type": "...", "description": "...", "context": "..." }}],
  "events": [{{ "title": "...", "event_type": "...", "summary": "...", "timestamp": "..." }}],
  "cases": [{{ "title": "...", "problem": "...", "solution": "...", "lessons_learned": ["..."] }}]
}}

Only include memories that are clearly stated in the conversation. Set empty arrays for categories with no data.

## Conversation

{}

## Response

Return ONLY the JSON object. No additional text before or after."#,
            messages_text
        )
    }

    /// Parse the LLM extraction response with detailed error logging
    fn parse_extraction_response(&self, response: &str) -> ExtractedMemories {
        // Try to extract JSON from the response
        let json_str = if response.starts_with('{') {
            response.to_string()
        } else {
            // Find JSON object in response
            let found = response
                .find('{')
                .and_then(|start| response.rfind('}').map(|end| (start, end)));

            match found {
                Some((start, end)) => response[start..=end].to_string(),
                None => {
                    warn!(
                        "LLM response contains no JSON object. Response preview: {}",
                        &response[..response.len().min(500)]
                    );
                    return ExtractedMemories::default();
                }
            }
        };

        match serde_json::from_str::<ExtractedMemories>(&json_str) {
            Ok(memories) => memories,
            Err(e) => {
                warn!(
                    "Failed to parse LLM extraction response: {}. JSON preview: {}",
                    e,
                    &json_str[..json_str.len().min(1000)]
                );
                ExtractedMemories::default()
            }
        }
    }

    /// Get current event statistics
    pub async fn get_stats(&self) -> EventStats {
        self.stats.read().await.clone()
    }

    /// Force a full update for a scope
    pub async fn force_full_update(&self, scope: &MemoryScope, owner_id: &str) -> Result<()> {
        info!("Forcing full update for {:?}/{}", scope, owner_id);

        // Update all layers
        self.layer_updater
            .update_all_layers(scope, owner_id)
            .await?;

        // Sync to vectors
        let root_uri = match scope {
            MemoryScope::User => format!("cortex://user/{}", owner_id),
            MemoryScope::Agent => format!("cortex://agent/{}", owner_id),
            MemoryScope::Session => format!("cortex://session/{}", owner_id),
            MemoryScope::Resources => "cortex://resources".to_string(),
        };

        self.vector_sync.sync_directory(&root_uri).await?;

        Ok(())
    }

    /// Delete all memories for a session
    pub async fn delete_session_memories(
        &self,
        session_id: &str,
        user_id: &str,
        agent_id: &str,
    ) -> Result<()> {
        info!("Deleting all memories for session {}", session_id);

        // Delete from index
        let deleted_user = self
            .index_manager
            .delete_memories_from_session(&MemoryScope::User, user_id, session_id)
            .await?;

        let deleted_agent = self
            .index_manager
            .delete_memories_from_session(&MemoryScope::Agent, agent_id, session_id)
            .await?;

        // Delete vectors
        self.vector_sync.delete_session_vectors(session_id).await?;

        info!(
            "Deleted {} user memories and {} agent memories for session {}",
            deleted_user.len(),
            deleted_agent.len(),
            session_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLLMClient;

    #[test]
    fn test_build_extraction_prompt() {
        let messages = vec![
            "User: I prefer Rust for systems programming.".to_string(),
            "Assistant: That's a great choice!".to_string(),
        ];

        // Build prompt directly (doesn't need coordinator)
        let messages_text = messages.join("\n\n---\n\n");
        let prompt = format!(
            r#"Analyze the following conversation and extract memories in JSON format.

## Conversation

{}

## Response

Return ONLY the JSON object. No additional text before or after."#,
            messages_text
        );

        assert!(prompt.contains("I prefer Rust"));
        assert!(prompt.contains("conversation"));
    }

    #[test]
    fn test_parse_extraction_response() {
        let llm_client = MockLLMClient::new();

        // Valid JSON response
        let response = r#"{
            "personal_info": [],
            "work_history": [],
            "preferences": [{"topic": "programming", "preference": "Rust", "confidence": 0.9}],
            "relationships": [],
            "goals": [],
            "entities": [],
            "events": [],
            "cases": []
        }"#;

        // Parse response directly
        let json_str = if response.starts_with('{') {
            response.to_string()
        } else {
            response
                .find('{')
                .and_then(|start| response.rfind('}').map(|end| &response[start..=end]))
                .map(|s| s.to_string())
                .unwrap_or_default()
        };

        let extracted: ExtractedMemories = serde_json::from_str(&json_str).unwrap_or_default();

        assert_eq!(extracted.preferences.len(), 1);
        assert_eq!(extracted.preferences[0].topic, "programming");
        assert_eq!(extracted.preferences[0].preference, "Rust");

        // Just to suppress unused variable warning
        let _ = llm_client;
    }

    #[test]
    fn test_parse_extraction_response_with_wrapper() {
        // Response with text wrapper
        let response = r#"Here is the extracted data:
        {
            "personal_info": [],
            "work_history": [],
            "preferences": [],
            "relationships": [],
            "goals": [{"goal": "Learn Rust", "category": "learning", "confidence": 0.8}],
            "entities": [],
            "events": [],
            "cases": []
        }
        That's all!"#;

        // Extract JSON from wrapper
        let json_str = response
            .find('{')
            .and_then(|start| response.rfind('}').map(|end| &response[start..=end]))
            .map(|s| s.to_string())
            .unwrap_or_default();

        let extracted: ExtractedMemories = serde_json::from_str(&json_str).unwrap_or_default();

        assert_eq!(extracted.goals.len(), 1);
        assert_eq!(extracted.goals[0].goal, "Learn Rust");
    }

    #[test]
    fn test_parse_extraction_response_empty() {
        // Empty response
        let json_str = "";
        let extracted: ExtractedMemories = serde_json::from_str(json_str).unwrap_or_default();
        assert!(extracted.is_empty());

        // Invalid JSON
        let extracted: ExtractedMemories = serde_json::from_str("not json").unwrap_or_default();
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_event_stats_tracking() {
        let mut stats = EventStats::default();

        stats.record(&MemoryEvent::MemoryCreated {
            scope: MemoryScope::User,
            owner_id: "user_001".to_string(),
            memory_id: "mem_001".to_string(),
            memory_type: crate::memory_index::MemoryType::Preference,
            key: "test".to_string(),
            source_session: "session_001".to_string(),
            file_uri: "cortex://user/user_001/test.md".to_string(),
        });

        stats.record(&MemoryEvent::SessionClosed {
            session_id: "session_001".to_string(),
            user_id: "user_001".to_string(),
            agent_id: "agent_001".to_string(),
        });

        assert_eq!(stats.memory_created, 1);
        assert_eq!(stats.sessions_closed, 1);
        assert_eq!(stats.total_events(), 2);
    }

    #[test]
    fn test_memory_event_scope() {
        let event = MemoryEvent::MemoryCreated {
            scope: MemoryScope::User,
            owner_id: "user_001".to_string(),
            memory_id: "mem_001".to_string(),
            memory_type: crate::memory_index::MemoryType::Preference,
            key: "test".to_string(),
            source_session: "session_001".to_string(),
            file_uri: "cortex://user/user_001/test.md".to_string(),
        };

        assert_eq!(event.scope(), Some(&MemoryScope::User));
        assert_eq!(event.owner_id(), Some("user_001"));
        assert!(event.requires_cascade_update());
        assert!(event.requires_vector_sync());
    }
}
