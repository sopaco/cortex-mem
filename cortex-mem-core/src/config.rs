use serde::{Deserialize, Serialize};

/// Qdrant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    pub embedding_dim: Option<usize>,
    pub timeout_secs: u64,
    pub api_key: Option<String>,
    /// 🆕 Optional tenant ID for collection isolation
    /// If set, collection_name will be suffixed with "_<tenant_id>"
    pub tenant_id: Option<String>,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6334".to_string(),
            collection_name: "cortex-mem".to_string(),
            embedding_dim: None,
            timeout_secs: 30,
            api_key: std::env::var("QDRANT_API_KEY").ok(),
            tenant_id: None,  // 🆕 默认不使用租户隔离
        }
    }
}

impl QdrantConfig {
    /// 🆕 Get the actual collection name with tenant isolation
    pub fn get_collection_name(&self) -> String {
        if let Some(tenant_id) = &self.tenant_id {
            format!("{}_{}", self.collection_name, tenant_id)
        } else {
            self.collection_name.clone()
        }
    }
    
    /// 🆕 Create a new config with tenant ID
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }
}
