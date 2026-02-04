mod indexer;
mod auto_extract;

pub use indexer::{IndexerConfig, IndexStats};
pub use auto_extract::{AutoExtractConfig, AutoExtractStats, AutoExtractor, AutoSessionManager};

#[cfg(feature = "vector-search")]
pub use indexer::AutoIndexer;
