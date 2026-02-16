mod auto_extract;

#[cfg(feature = "vector-search")]
mod indexer;
#[cfg(feature = "vector-search")]
mod sync;
#[cfg(feature = "vector-search")]
mod watcher;

pub use auto_extract::{AutoExtractConfig, AutoExtractStats, AutoExtractor, AutoSessionManager};

#[cfg(feature = "vector-search")]
pub use indexer::{AutoIndexer, IndexStats, IndexerConfig};

#[cfg(feature = "vector-search")]
pub use sync::{SyncConfig, SyncManager, SyncStats};

#[cfg(feature = "vector-search")]
pub use watcher::{FsEvent, FsWatcher, WatcherConfig};
