mod auto_extract;
mod indexer;
mod layer_generator;  // 🆕 层级生成器
mod manager;  // 🆕 自动化管理器
mod sync;
mod watcher;

pub use auto_extract::{AutoExtractConfig, AutoExtractStats, AutoExtractor, AutoSessionManager};
pub use indexer::{AutoIndexer, IndexStats, IndexerConfig};
pub use layer_generator::{LayerGenerator, LayerGenerationConfig, GenerationStats, RegenerationStats, AbstractConfig, OverviewConfig};  // 🆕 导出
pub use manager::{AutomationConfig, AutomationManager};  // 🆕 导出
pub use sync::{SyncConfig, SyncManager, SyncStats};
pub use watcher::{FsEvent, FsWatcher, WatcherConfig};