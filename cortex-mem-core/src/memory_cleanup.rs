//! Memory Cleanup Service
//!
//! 负责定期扫描记忆索引，根据记忆强度（Ebbinghaus 遗忘曲线）
//! 将低强度记忆归档或彻底删除，控制长程 Agent 的记忆空间膨胀。

use crate::{
    memory_index::{MemoryMetadata, MemoryScope},
    memory_index_manager::MemoryIndexManager,
    Result,
};
use std::sync::Arc;
use tracing::{info, warn};

/// 清理服务配置
#[derive(Debug, Clone)]
pub struct MemoryCleanupConfig {
    /// 清理间隔（小时）
    pub interval_hours: u64,
    /// 归档阈值：记忆强度低于此值时标记为归档（默认 0.1）
    pub archive_threshold: f32,
    /// 删除阈值：已归档且强度低于此值时彻底删除（默认 0.02）
    pub delete_threshold: f32,
}

impl Default for MemoryCleanupConfig {
    fn default() -> Self {
        Self {
            interval_hours: 24,
            archive_threshold: 0.1,
            delete_threshold: 0.02,
        }
    }
}

/// 单次清理结果统计
#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
    /// 已归档条目数
    pub archived: usize,
    /// 已删除条目数
    pub deleted: usize,
    /// 检查的总条目数
    pub total_scanned: usize,
}

/// 记忆清理服务
///
/// 使用方式：
/// ```rust,no_run
/// // 在 Agent 启动时创建，定期手动调用 run_cleanup，或用 tokio::spawn + interval 运行
/// let svc = MemoryCleanupService::new(index_manager, MemoryCleanupConfig::default());
/// let stats = svc.run_cleanup(&MemoryScope::User, "alice").await?;
/// println!("Archived: {}, Deleted: {}", stats.archived, stats.deleted);
/// ```
pub struct MemoryCleanupService {
    index_manager: Arc<MemoryIndexManager>,
    config: MemoryCleanupConfig,
}

impl MemoryCleanupService {
    pub fn new(index_manager: Arc<MemoryIndexManager>, config: MemoryCleanupConfig) -> Self {
        Self {
            index_manager,
            config,
        }
    }

    /// 对指定 scope/owner 执行一次记忆清理
    pub async fn run_cleanup(
        &self,
        scope: &MemoryScope,
        owner_id: &str,
    ) -> Result<CleanupStats> {
        let mut stats = CleanupStats::default();

        let index = self
            .index_manager
            .load_index(scope.clone(), owner_id.to_string())
            .await?;
        let memory_ids: Vec<String> = index.memories.keys().cloned().collect();
        stats.total_scanned = memory_ids.len();

        let mut to_archive: Vec<String> = Vec::new();
        let mut to_delete: Vec<String> = Vec::new();

        for (id, metadata) in &index.memories {
            let strength = metadata.compute_strength();

            if metadata.archived && strength < self.config.delete_threshold {
                to_delete.push(id.clone());
            } else if !metadata.archived && strength < self.config.archive_threshold {
                to_archive.push(id.clone());
            }
        }

        // 先归档
        if !to_archive.is_empty() {
            let mut index = self
                .index_manager
                .load_index(scope.clone(), owner_id.to_string())
                .await?;
            for id in &to_archive {
                if let Some(meta) = index.memories.get_mut(id) {
                    let strength = meta.compute_strength();
                    info!(
                        "Archiving memory '{}' (strength={:.3}, key='{}')",
                        id, strength, meta.key
                    );
                    meta.archived = true;
                }
            }
            self.index_manager.save_index(&index).await?;
            stats.archived = to_archive.len();
        }

        // 再删除已归档且强度极低的记忆
        if !to_delete.is_empty() {
            let mut index = self
                .index_manager
                .load_index(scope.clone(), owner_id.to_string())
                .await?;
            for id in &to_delete {
                if let Some(meta) = index.memories.remove(id) {
                    warn!(
                        "Deleting archived memory '{}' (strength < {:.3}, key='{}')",
                        id, self.config.delete_threshold, meta.key
                    );
                }
            }
            self.index_manager.save_index(&index).await?;
            stats.deleted = to_delete.len();
        }

        info!(
            "Cleanup complete for {}/{}: scanned={}, archived={}, deleted={}",
            scope, owner_id, stats.total_scanned, stats.archived, stats.deleted
        );

        Ok(stats)
    }

    /// 批量对多个 owner 执行清理（按 scope 分组）
    pub async fn run_cleanup_batch(
        &self,
        entries: &[(MemoryScope, String)],
    ) -> Result<CleanupStats> {
        let mut total = CleanupStats::default();
        for (scope, owner_id) in entries {
            match self.run_cleanup(scope, owner_id).await {
                Ok(stats) => {
                    total.total_scanned += stats.total_scanned;
                    total.archived += stats.archived;
                    total.deleted += stats.deleted;
                }
                Err(e) => {
                    warn!("Cleanup failed for {}/{}: {}", scope, owner_id, e);
                }
            }
        }
        Ok(total)
    }
}

/// 公共工具函数：直接计算某个 MemoryMetadata 的当前强度（供检索时惩罚分数使用）
pub fn compute_memory_strength(metadata: &MemoryMetadata) -> f32 {
    metadata.compute_strength()
}
