mod vector_engine;

pub use vector_engine::{SearchOptions, SearchResult};

#[cfg(feature = "vector-search")]
pub use vector_engine::VectorSearchEngine;
