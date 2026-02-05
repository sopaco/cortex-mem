pub mod qdrant;

use crate::{
    error::Result,
    types::{Filters, Memory, ScoredMemory},
};
use async_trait::async_trait;

pub use qdrant::QdrantVectorStore;

/// Trait for vector store operations
#[async_trait]
pub trait VectorStore: Send + Sync + dyn_clone::DynClone {
    /// Insert a memory into the vector store
    async fn insert(&self, memory: &Memory) -> Result<()>;

    /// Search for similar memories
    async fn search(
        &self,
        query_vector: &[f32],
        filters: &Filters,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>>;

    /// Search for similar memories with similarity threshold
    async fn search_with_threshold(
        &self,
        query_vector: &[f32],
        filters: &Filters,
        limit: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<ScoredMemory>>;

    /// Update an existing memory
    async fn update(&self, memory: &Memory) -> Result<()>;

    /// Delete a memory by ID
    async fn delete(&self, id: &str) -> Result<()>;

    /// Get a memory by ID
    async fn get(&self, id: &str) -> Result<Option<Memory>>;

    /// List all memories with optional filters
    async fn list(&self, filters: &Filters, limit: Option<usize>) -> Result<Vec<Memory>>;

    /// Check if the vector store is healthy
    async fn health_check(&self) -> Result<bool>;
    
    /// Scroll through memory IDs (for incremental indexing)
    async fn scroll_ids(&self, filters: &Filters, limit: usize) -> Result<Vec<String>>;
}

dyn_clone::clone_trait_object!(VectorStore);
