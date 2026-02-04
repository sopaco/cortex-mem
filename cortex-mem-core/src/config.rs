use serde::{Deserialize, Serialize};

/// Qdrant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    pub embedding_dim: Option<usize>,
    pub timeout_secs: u64,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6334".to_string(),
            collection_name: "cortex-mem".to_string(),
            embedding_dim: None,
            timeout_secs: 30,
        }
    }
}
