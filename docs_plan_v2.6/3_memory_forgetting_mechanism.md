# 记忆遗忘机制设计技术方案

## 1. 问题分析

### 1.1 现有元数据未被充分利用

**位置**: `cortex-mem-core/src/memory_index.rs`

当前 `MemoryMetadata` 已定义了访问统计字段，但未实现基于此的衰减或遗忘逻辑：

```rust
pub struct MemoryMetadata {
    pub last_accessed: DateTime<Utc>,  // 有字段
    pub access_count: u32,              // 有字段
    pub confidence: f32,                // 有字段
    // 缺少：
    // - importance_decay (重要性衰减)
    // - ttl (存活时间)
    // - last_updated (内容更新时间)
    // - memory_phase (记忆阶段)
}
```

### 1.2 长期运行的问题

对于持续数月的 Coding Agent（如 Cursor、Claude Code），会积累大量记忆：

| 时间跨度 | 预估记忆量 | 问题 |
|----------|------------|------|
| 1 个月 | ~500 条 | 检索效率下降 |
| 3 个月 | ~1500 条 | 召回精度下降，噪声增加 |
| 6 个月 | ~3000 条 | 存储膨胀，过时信息干扰 |
| 1 年 | ~6000 条 | 系统性能显著下降 |

### 1.3 过时信息的危害

1. **召回噪声**：已废弃的项目配置、过时的技术决策干扰当前查询
2. **存储浪费**：无价值信息占用向量索引空间
3. **上下文污染**：Agent 可能基于过时信息做出错误决策

## 2. 理论基础

### 2.1 艾宾浩斯遗忘曲线

记忆强度随时间自然衰减，呈现指数曲线：

```
记忆强度 = 初始强度 × e^(-t/τ)

其中：
- t: 距上次访问的时间
- τ: 时间常数（半衰期）
```

### 2.2 记忆巩固理论

频繁访问的记忆会被"巩固"，衰减速度减慢：

```
衰减速度 ∝ 1 / log(1 + access_count)
```

### 2.3 工作记忆与长期记忆

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        记忆生命周期模型                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   新记忆 ──► 工作记忆 ──► 短期记忆 ──► 长期记忆 ──► 归档      │
│     │          │            │            │            │                    │
│     │          │            │            │            │                    │
│     │     高频访问       中频访问      低频访问     极少访问                  │
│     │     快速衰减       中速衰减      慢速衰减     仅查询可见                │
│     │     TTL: 7天       TTL: 30天     TTL: 90天   永久保留                  │
│     │          │            │            │            │                    │
│     └──────────┴────────────┴────────────┴────────────┘                    │
│                              │                                              │
│                              ▼                                              │
│                        自动遗忘清理                                          │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 3. 解决方案

### 3.1 增强的记忆元数据

```rust
// 文件: cortex-mem-core/src/memory_index.rs (修改)

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 记忆阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPhase {
    /// 工作记忆：当前会话频繁使用
    Working,
    /// 短期记忆：近期使用，可能过渡到长期
    ShortTerm,
    /// 长期记忆：已巩固，衰减慢
    LongTerm,
    /// 归档记忆：极少访问，仅特定查询可见
    Archived,
}

impl Default for MemoryPhase {
    fn default() -> Self {
        MemoryPhase::Working
    }
}

/// 遗忘策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingConfig {
    /// 工作记忆半衰期（天）
    pub working_half_life: f32,
    /// 短期记忆半衰期（天）
    pub short_term_half_life: f32,
    /// 长期记忆半衰期（天）
    pub long_term_half_life: f32,
    /// 巩固阈值：访问次数达到此值后进入下一阶段
    pub consolidation_threshold: u32,
    /// 遗忘阈值：强度低于此值的记忆将被清理
    pub forgetting_threshold: f32,
    /// 归档阈值：强度低于此值进入归档
    pub archive_threshold: f32,
    /// 是否启用自动清理
    pub auto_cleanup_enabled: bool,
    /// 清理周期（小时）
    pub cleanup_interval_hours: u64,
}

impl Default for ForgettingConfig {
    fn default() -> Self {
        Self {
            working_half_life: 7.0,      // 7 天半衰期
            short_term_half_life: 30.0,  // 30 天半衰期
            long_term_half_life: 180.0,  // 180 天半衰期
            consolidation_threshold: 5,  // 访问 5 次后巩固
            forgetting_threshold: 0.1,   // 强度低于 0.1 删除
            archive_threshold: 0.3,      // 强度低于 0.3 归档
            auto_cleanup_enabled: true,
            cleanup_interval_hours: 24,
        }
    }
}

/// 增强的记忆元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMemoryMetadata {
    // === 原有字段 ===
    pub id: String,
    pub file: String,
    pub memory_type: MemoryType,
    pub key: String,
    pub content_hash: String,
    pub source_sessions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub confidence: f32,
    pub content_summary: String,
    
    // === 新增字段 ===
    /// 记忆阶段
    pub phase: MemoryPhase,
    /// 重要性评分（用户可手动调整）
    pub importance: f32,
    /// 衰减后的当前强度
    pub current_strength: f32,
    /// 最后强度计算时间
    pub strength_updated_at: DateTime<Utc>,
    /// 是否标记为重要（永不遗忘）
    pub is_pinned: bool,
    /// 自定义 TTL（秒），None 表示使用默认
    pub custom_ttl: Option<u64>,
    /// 关联的其他记忆 ID
    pub related_memories: Vec<String>,
    /// 标签（用于分类遗忘）
    pub tags: Vec<String>,
    /// 内容过期时间（用于时序性信息）
    pub expires_at: Option<DateTime<Utc>>,
}

impl EnhancedMemoryMetadata {
    /// 创建新记忆
    pub fn new(
        id: String,
        file: String,
        memory_type: MemoryType,
        key: String,
        content_hash: String,
        source_session: &str,
        confidence: f32,
        content_summary: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            file,
            memory_type,
            key,
            content_hash,
            source_sessions: vec![source_session.to_string()],
            created_at: now,
            updated_at: now,
            last_accessed: now,
            access_count: 0,
            confidence,
            content_summary,
            
            // 新字段默认值
            phase: MemoryPhase::Working,
            importance: 0.5,
            current_strength: 1.0,
            strength_updated_at: now,
            is_pinned: false,
            custom_ttl: None,
            related_memories: Vec::new(),
            tags: Vec::new(),
            expires_at: None,
        }
    }
    
    /// 从旧元数据迁移
    pub fn from_legacy(legacy: MemoryMetadata) -> Self {
        Self {
            id: legacy.id,
            file: legacy.file,
            memory_type: legacy.memory_type,
            key: legacy.key,
            content_hash: legacy.content_hash,
            source_sessions: legacy.source_sessions,
            created_at: legacy.created_at,
            updated_at: legacy.updated_at,
            last_accessed: legacy.last_accessed,
            access_count: legacy.access_count,
            confidence: legacy.confidence,
            content_summary: legacy.content_summary,
            
            phase: Self::infer_phase_from_access(legacy.access_count),
            importance: legacy.confidence,
            current_strength: 1.0,
            strength_updated_at: Utc::now(),
            is_pinned: false,
            custom_ttl: None,
            related_memories: Vec::new(),
            tags: Vec::new(),
            expires_at: None,
        }
    }
    
    fn infer_phase_from_access(access_count: u32) -> MemoryPhase {
        match access_count {
            0..=2 => MemoryPhase::Working,
            3..=10 => MemoryPhase::ShortTerm,
            _ => MemoryPhase::LongTerm,
        }
    }
}
```

### 3.2 遗忘计算引擎

```rust
// 文件: cortex-mem-core/src/forgetting/engine.rs (新建)

use crate::memory_index::{EnhancedMemoryMetadata, MemoryPhase, ForgettingConfig};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// 遗忘计算引擎
pub struct ForgettingEngine {
    config: ForgettingConfig,
}

impl ForgettingEngine {
    pub fn new(config: ForgettingConfig) -> Self {
        Self { config }
    }
    
    /// 计算记忆的当前强度
    ///
    /// 强度 = 基础强度 × 衰减因子 × 巩固加成
    pub fn calculate_strength(&self, memory: &EnhancedMemoryMetadata) -> f32 {
        if memory.is_pinned {
            return 1.0;  // 固定的记忆永不衰减
        }
        
        // 1. 计算时间衰减
        let days_since_access = (Utc::now() - memory.last_accessed).num_days() as f32;
        let half_life = self.get_half_life(memory.phase);
        let decay_factor = (-days_since_access / half_life).exp();
        
        // 2. 计算巩固加成（访问次数越多，衰减越慢）
        let consolidation_bonus = 1.0 + (memory.access_count as f32).ln().max(0.0) * 0.1;
        
        // 3. 计算重要性加成
        let importance_factor = memory.importance * 0.5 + 0.5;  // 0.5 ~ 1.0
        
        // 4. 综合强度
        let strength = memory.confidence * decay_factor * consolidation_bonus * importance_factor;
        
        // 5. 检查是否过期
        if let Some(expires_at) = memory.expires_at {
            if Utc::now() > expires_at {
                return 0.0;  // 已过期
            }
        }
        
        strength.clamp(0.0, 1.0)
    }
    
    /// 获取对应阶段的半衰期
    fn get_half_life(&self, phase: MemoryPhase) -> f32 {
        match phase {
            MemoryPhase::Working => self.config.working_half_life,
            MemoryPhase::ShortTerm => self.config.short_term_half_life,
            MemoryPhase::LongTerm => self.config.long_term_half_life,
            MemoryPhase::Archived => self.config.long_term_half_life * 2.0,
        }
    }
    
    /// 判断记忆是否应该被遗忘
    pub fn should_forget(&self, memory: &EnhancedMemoryMetadata) -> bool {
        if memory.is_pinned {
            return false;
        }
        
        let strength = self.calculate_strength(memory);
        strength < self.config.forgetting_threshold
    }
    
    /// 判断记忆是否应该归档
    pub fn should_archive(&self, memory: &EnhancedMemoryMetadata) -> bool {
        if memory.is_pinned || memory.phase == MemoryPhase::Archived {
            return false;
        }
        
        let strength = self.calculate_strength(memory);
        strength < self.config.archive_threshold && strength >= self.config.forgetting_threshold
    }
    
    /// 判断记忆是否应该巩固（提升阶段）
    pub fn should_consolidate(&self, memory: &EnhancedMemoryMetadata) -> Option<MemoryPhase> {
        if memory.access_count >= self.config.consolidation_threshold {
            match memory.phase {
                MemoryPhase::Working => Some(MemoryPhase::ShortTerm),
                MemoryPhase::ShortTerm => Some(MemoryPhase::LongTerm),
                MemoryPhase::LongTerm | MemoryPhase::Archived => None,
            }
        } else {
            None
        }
    }
    
    /// 更新记忆的强度和阶段
    pub fn update_memory_state(&self, memory: &mut EnhancedMemoryMetadata) -> MemoryStateChange {
        let mut changes = MemoryStateChange::default();
        
        // 计算新强度
        let new_strength = self.calculate_strength(memory);
        let old_strength = memory.current_strength;
        memory.current_strength = new_strength;
        memory.strength_updated_at = Utc::now();
        
        if (new_strength - old_strength).abs() > 0.01 {
            changes.strength_changed = true;
        }
        
        // 检查巩固
        if let Some(new_phase) = self.should_consolidate(memory) {
            memory.phase = new_phase;
            changes.phase_changed = true;
            changes.new_phase = Some(new_phase);
        }
        
        // 检查归档
        if self.should_archive(memory) {
            memory.phase = MemoryPhase::Archived;
            changes.phase_changed = true;
            changes.new_phase = Some(MemoryPhase::Archived);
            changes.archived = true;
        }
        
        // 检查遗忘
        changes.should_forget = self.should_forget(memory);
        
        changes
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryStateChange {
    pub strength_changed: bool,
    pub phase_changed: bool,
    pub new_phase: Option<MemoryPhase>,
    pub archived: bool,
    pub should_forget: bool,
}

/// 遗忘操作类型
#[derive(Debug, Clone)]
pub enum ForgettingAction {
    /// 删除记忆
    Delete { memory_id: String },
    /// 归档记忆
    Archive { memory_id: String },
    /// 巩固记忆（提升阶段）
    Consolidate { memory_id: String, new_phase: MemoryPhase },
    /// 更新强度
    UpdateStrength { memory_id: String, new_strength: f32 },
    /// 无操作
    None,
}
```

### 3.3 记忆清理服务

```rust
// 文件: cortex-mem-core/src/forgetting/cleanup_service.rs (新建)

use crate::forgetting::{ForgettingEngine, ForgettingAction, ForgettingConfig};
use crate::memory_index_manager::MemoryIndexManager;
use crate::memory_events::{MemoryEvent, ChangeType, DeleteReason};
use crate::vector_store::QdrantVectorStore;
use crate::filesystem::{CortexFilesystem, FilesystemOperations};
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};

/// 记忆清理服务
pub struct MemoryCleanupService {
    engine: ForgettingEngine,
    index_manager: Arc<MemoryIndexManager>,
    vector_store: Arc<QdrantVectorStore>,
    filesystem: Arc<CortexFilesystem>,
    event_tx: mpsc::UnboundedSender<MemoryEvent>,
    config: ForgettingConfig,
}

/// 清理统计
#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
    pub total_scanned: usize,
    pub deleted: usize,
    pub archived: usize,
    pub consolidated: usize,
    pub strength_updated: usize,
    pub errors: usize,
}

impl MemoryCleanupService {
    pub fn new(
        config: ForgettingConfig,
        index_manager: Arc<MemoryIndexManager>,
        vector_store: Arc<QdrantVectorStore>,
        filesystem: Arc<CortexFilesystem>,
        event_tx: mpsc::UnboundedSender<MemoryEvent>,
    ) -> Self {
        Self {
            engine: ForgettingEngine::new(config.clone()),
            index_manager,
            vector_store,
            filesystem,
            event_tx,
            config,
        }
    }
    
    /// 启动定期清理任务
    pub fn start_background_cleanup(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(self.config.cleanup_interval_hours * 3600);
            
            loop {
                tokio::time::sleep(interval).await;
                
                if !self.config.auto_cleanup_enabled {
                    continue;
                }
                
                info!("Starting scheduled memory cleanup");
                
                match self.run_cleanup().await {
                    Ok(stats) => {
                        info!(
                            "Cleanup completed: {} scanned, {} deleted, {} archived, {} consolidated",
                            stats.total_scanned, stats.deleted, stats.archived, stats.consolidated
                        );
                    }
                    Err(e) => {
                        warn!("Cleanup failed: {}", e);
                    }
                }
            }
        })
    }
    
    /// 执行一次清理
    pub async fn run_cleanup(&self) -> Result<CleanupStats> {
        let mut stats = CleanupStats::default();
        
        // 1. 扫描所有维度的记忆
        let scopes = [
            (crate::memory_index::MemoryScope::User, "user"),
            (crate::memory_index::MemoryScope::Agent, "agent"),
        ];
        
        for (scope, owner_prefix) in scopes {
            let owners = self.index_manager.list_owners(&scope).await?;
            
            for owner_id in owners {
                let owner_stats = self.cleanup_owner_memories(&scope, &owner_id).await?;
                stats.total_scanned += owner_stats.total_scanned;
                stats.deleted += owner_stats.deleted;
                stats.archived += owner_stats.archived;
                stats.consolidated += owner_stats.consolidated;
                stats.strength_updated += owner_stats.strength_updated;
                stats.errors += owner_stats.errors;
            }
        }
        
        Ok(stats)
    }
    
    /// 清理特定所有者的记忆
    async fn cleanup_owner_memories(
        &self,
        scope: &crate::memory_index::MemoryScope,
        owner_id: &str,
    ) -> Result<CleanupStats> {
        let mut stats = CleanupStats::default();
        
        let index = self.index_manager.get_or_create_index(scope, owner_id).await?;
        let mut index = index.write().await;
        
        let memory_ids: Vec<String> = index.memories.keys().cloned().collect();
        stats.total_scanned = memory_ids.len();
        
        let mut to_delete = Vec::new();
        let mut to_archive = Vec::new();
        let mut to_consolidate = Vec::new();
        
        for memory_id in &memory_ids {
            if let Some(memory) = index.memories.get_mut(memory_id) {
                let change = self.engine.update_memory_state(memory);
                
                if change.should_forget {
                    to_delete.push(memory_id.clone());
                } else if change.archived {
                    to_archive.push(memory_id.clone());
                } else if let Some(new_phase) = change.new_phase {
                    to_consolidate.push((memory_id.clone(), new_phase));
                }
                
                if change.strength_changed {
                    stats.strength_updated += 1;
                }
            }
        }
        
        // 执行删除
        for memory_id in &to_delete {
            if let Some(memory) = index.memories.remove(memory_id) {
                self.delete_memory_files(&memory.file).await?;
                self.emit_delete_event(memory_id, &memory.file).await;
                stats.deleted += 1;
            }
        }
        
        // 归档已在 update_memory_state 中更新了 phase
        stats.archived = to_archive.len();
        
        // 巩固计数
        stats.consolidated = to_consolidate.len();
        
        // 保存更新后的索引
        self.index_manager.save_index(scope, owner_id, &index).await?;
        
        Ok(stats)
    }
    
    async fn delete_memory_files(&self, file_path: &str) -> Result<()> {
        // 从文件系统删除
        let uri = format!("cortex://user/{}", file_path);
        if self.filesystem.exists(&uri).await? {
            self.filesystem.delete(&uri).await?;
        }
        
        // 从向量存储删除
        self.vector_store.delete_by_uri(&uri).await?;
        
        Ok(())
    }
    
    async fn emit_delete_event(&self, memory_id: &str, file_path: &str) {
        let event = MemoryEvent::Deleted {
            memory_id: memory_id.to_string(),
            uri: format!("cortex://user/{}", file_path),
            reason: DeleteReason::Expired,
            timestamp: chrono::Utc::now(),
        };
        
        let _ = self.event_tx.send(event);
    }
}

/// 手动遗忘操作
impl MemoryCleanupService {
    /// 标记记忆为重要（永不遗忘）
    pub async fn pin_memory(&self, scope: &crate::memory_index::MemoryScope, owner_id: &str, memory_id: &str) -> Result<()> {
        let index = self.index_manager.get_or_create_index(scope, owner_id).await?;
        let mut index = index.write().await;
        
        if let Some(memory) = index.memories.get_mut(memory_id) {
            memory.is_pinned = true;
            memory.current_strength = 1.0;
        }
        
        self.index_manager.save_index(scope, owner_id, &index).await?;
        Ok(())
    }
    
    /// 取消固定
    pub async fn unpin_memory(&self, scope: &crate::memory_index::MemoryScope, owner_id: &str, memory_id: &str) -> Result<()> {
        let index = self.index_manager.get_or_create_index(scope, owner_id).await?;
        let mut index = index.write().await;
        
        if let Some(memory) = index.memories.get_mut(memory_id) {
            memory.is_pinned = false;
        }
        
        self.index_manager.save_index(scope, owner_id, &index).await?;
        Ok(())
    }
    
    /// 手动删除记忆
    pub async fn delete_memory(&self, scope: &crate::memory_index::MemoryScope, owner_id: &str, memory_id: &str) -> Result<()> {
        let index = self.index_manager.get_or_create_index(scope, owner_id).await?;
        let mut index = index.write().await;
        
        if let Some(memory) = index.memories.remove(memory_id) {
            self.delete_memory_files(&memory.file).await?;
            self.emit_delete_event(memory_id, &memory.file).await;
        }
        
        self.index_manager.save_index(scope, owner_id, &index).await?;
        Ok(())
    }
    
    /// 设置记忆过期时间
    pub async fn set_memory_expiry(
        &self,
        scope: &crate::memory_index::MemoryScope,
        owner_id: &str,
        memory_id: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let index = self.index_manager.get_or_create_index(scope, owner_id).await?;
        let mut index = index.write().await;
        
        if let Some(memory) = index.memories.get_mut(memory_id) {
            memory.expires_at = expires_at;
        }
        
        self.index_manager.save_index(scope, owner_id, &index).await?;
        Ok(())
    }
    
    /// 手动调整记忆重要性
    pub async fn set_memory_importance(
        &self,
        scope: &crate::memory_index::MemoryScope,
        owner_id: &str,
        memory_id: &str,
        importance: f32,
    ) -> Result<()> {
        let index = self.index_manager.get_or_create_index(scope, owner_id).await?;
        let mut index = index.write().await;
        
        if let Some(memory) = index.memories.get_mut(memory_id) {
            memory.importance = importance.clamp(0.0, 1.0);
        }
        
        self.index_manager.save_index(scope, owner_id, &index).await?;
        Ok(())
    }
}
```

### 3.4 检索时考虑记忆强度

```rust
// 文件: cortex-mem-core/src/search/vector_engine.rs (修改)

impl VectorSearchEngine {
    /// 在检索结果中加入记忆强度因子
    async fn apply_forgetting_factor(&self, results: &mut [SearchResult]) {
        for result in results.iter_mut() {
            // 从 metadata 获取记忆强度
            if let Some(strength) = self.get_memory_strength(&result.uri).await {
                // 调整分数：原始分数 × 强度因子
                // 使用幂函数使高强度记忆保持高分，低强度记忆显著降权
                let strength_factor = strength.powf(0.5);  // 平方根减缓衰减
                result.score *= strength_factor;
            }
        }
    }
    
    async fn get_memory_strength(&self, uri: &str) -> Option<f32> {
        // 从索引中获取记忆强度
        // 实现取决于具体的索引查询方式
        None
    }
}
```

### 3.5 配置支持

```toml
# config.toml 新增配置

[forgetting]
# 是否启用遗忘机制
enabled = true

# 是否启用自动清理
auto_cleanup = true

# 清理周期（小时）
cleanup_interval_hours = 24

# 半衰期配置（天）
working_half_life = 7
short_term_half_life = 30
long_term_half_life = 180

# 巩固阈值：访问次数达到此值后进入下一阶段
consolidation_threshold = 5

# 遗忘阈值：强度低于此值的记忆将被清理
forgetting_threshold = 0.1

# 归档阈值：强度低于此值进入归档
archive_threshold = 0.3

# 归档记忆是否参与检索
include_archived_in_search = false
```

### 3.6 CLI 命令扩展

```rust
// cortex-mem-cli 新增命令

// 记忆管理命令
cortex-mem-cli memory pin <memory_id>       // 标记为重要
cortex-mem-cli memory unpin <memory_id>     // 取消标记
cortex-mem-cli memory expire <memory_id> --days 30  // 设置过期时间
cortex-mem-cli memory importance <memory_id> --value 0.9  // 设置重要性

// 遗忘管理命令
cortex-mem-cli forgetting status            // 查看遗忘状态统计
cortex-mem-cli forgetting preview           // 预览将要被清理的记忆
cortex-mem-cli forgetting run               // 手动触发清理
cortex-mem-cli forgetting config            // 查看当前配置
```

## 4. 数据迁移方案

### 4.1 从现有元数据迁移

```rust
// 文件: cortex-mem-core/src/migration/memory_metadata.rs (新建)

/// 迁移现有的 MemoryMetadata 到 EnhancedMemoryMetadata
pub async fn migrate_memory_indices(data_dir: &Path) -> Result<MigrationStats> {
    let mut stats = MigrationStats::default();
    
    // 扫描所有 .memory_index.json 文件
    for entry in walkdir::WalkDir::new(data_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == ".memory_index.json")
    {
        let path = entry.path();
        match migrate_single_index(path).await {
            Ok(s) => stats.merge(s),
            Err(e) => {
                warn!("Failed to migrate {}: {}", path.display(), e);
                stats.errors += 1;
            }
        }
    }
    
    Ok(stats)
}

async fn migrate_single_index(path: &Path) -> Result<MigrationStats> {
    // 1. 读取旧格式
    let content = tokio::fs::read_to_string(path).await?;
    let old_index: MemoryIndex = serde_json::from_str(&content)?;
    
    // 2. 转换为新格式
    let new_index = EnhancedMemoryIndex {
        version: 2,
        scope: old_index.scope,
        owner_id: old_index.owner_id,
        last_updated: Utc::now(),
        memories: old_index.memories
            .into_iter()
            .map(|(k, v)| (k, EnhancedMemoryMetadata::from_legacy(v)))
            .collect(),
        session_summaries: old_index.session_summaries,
    };
    
    // 3. 备份旧文件
    let backup_path = path.with_extension("json.bak");
    tokio::fs::copy(path, &backup_path).await?;
    
    // 4. 写入新格式
    let new_content = serde_json::to_string_pretty(&new_index)?;
    tokio::fs::write(path, new_content).await?;
    
    Ok(MigrationStats {
        migrated: new_index.memories.len(),
        errors: 0,
    })
}
```

## 5. 实现计划

| 步骤 | 任务 | 依赖 |
|------|------|------|
| 1 | 定义 EnhancedMemoryMetadata 和 ForgettingConfig | 无 |
| 2 | 实现 ForgettingEngine 核心算法 | 步骤1 |
| 3 | 实现 MemoryCleanupService | 步骤2 |
| 4 | 修改检索逻辑，考虑记忆强度 | 步骤1 |
| 5 | 添加 CLI 命令 | 步骤3 |
| 6 | 实现数据迁移工具 | 步骤1 |
| 7 | 添加配置支持 | 全部 |

## 6. 预期收益

| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 6个月记忆量 | ~3000 条 | ~1500 条 | 50% 减少 |
| 召回噪声率 | ~25% | ~8% | 17pp 下降 |
| 存储空间 | 100% | 60% | 40% 节省 |
| 检索延迟 | 100% | 70% | 30% 提升 |

## 7. 注意事项

1. **默认保守**：首次部署时遗忘阈值设置较高，避免误删重要记忆
2. **用户可控**：提供 pin/unpin 功能让用户手动保护重要记忆
3. **审计日志**：记录所有遗忘操作，支持审计和恢复
4. **灰度发布**：先在测试环境验证，再逐步推广
