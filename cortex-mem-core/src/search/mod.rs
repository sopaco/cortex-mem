mod vector_engine;

#[cfg(all(test, feature = "vector-search"))]
mod vector_search_tests;

pub use vector_engine::{SearchOptions, SearchResult};

#[cfg(feature = "vector-search")]
pub use vector_engine::VectorSearchEngine;
