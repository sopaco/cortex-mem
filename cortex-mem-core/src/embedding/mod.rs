mod client;
mod cache;  // 🆕 Embedding 缓存层

pub use client::{EmbeddingClient, EmbeddingConfig};
pub use cache::{EmbeddingCache, CacheConfig, CacheStats, EmbeddingProvider};  // 🆕
