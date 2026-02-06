use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure (V2 - simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub qdrant: QdrantConfig,
    pub llm: LLMConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}

/// Qdrant vector database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    pub embedding_dim: Option<usize>,
    pub timeout_secs: u64,
}

/// LLM configuration for rig framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model_efficient: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub log_directory: String,
    pub level: String,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            enabled: false,
            log_directory: "logs".to_string(),
            level: "info".to_string(),
        }
    }
}
