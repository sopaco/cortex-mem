mod auto_extract;

#[cfg(feature = "vector-search")]
mod indexer;
#[cfg(feature = "vector-search")]
mod sync;
#[cfg(feature = "vector-search")]
mod watcher;

#[cfg(all(test, feature = "vector-search"))]
mod indexer_tests;

pub use auto_extract::{AutoExtractConfig, AutoExtractStats, AutoExtractor, AutoSessionManager};

#[cfg(feature = "vector-search")]
pub use indexer::{IndexerConfig, IndexStats, AutoIndexer};

#[cfg(feature = "vector-search")]
pub use sync::{SyncManager, SyncConfig, SyncStats};

#[cfg(feature = "vector-search")]
pub use watcher::{FsWatcher, WatcherConfig, FsEvent};
