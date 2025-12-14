//! 记忆管理器模块
//! 
//! 提供MemoryManager实例的创建和配置

mod in_memory_vector_store;
mod mock_llm_client;

pub use in_memory_vector_store::InMemoryVectorStore;
pub use mock_llm_client::MockLLMClient;

use anyhow::Result;
use cortex_mem_core::{MemoryManager, MemoryConfig};
use cortex_mem_config::Config;
use std::sync::Arc;
use tracing::info;

/// 创建用于评估的MemoryManager实例
pub async fn create_memory_manager_for_evaluation() -> Result<Arc<MemoryManager>> {
    // 创建内存向量存储
    let vector_store = Box::new(InMemoryVectorStore::new());
    
    // 创建模拟LLM客户端
    let llm_client = Box::new(MockLLMClient::new(false)); // 不启用详细输出
    
    // 创建记忆配置
    let memory_config = MemoryConfig {
        max_memories: 10000,
        similarity_threshold: 0.65,
        max_search_results: 50,
        memory_ttl_hours: None,
        auto_summary_threshold: 32768,
        auto_enhance: true,
        deduplicate: true,
        merge_threshold: 0.75,
        search_similarity_threshold: Some(0.70),
    };
    
    // 创建MemoryManager
    let memory_manager = MemoryManager::new(vector_store, llm_client, memory_config);
    
    Ok(Arc::new(memory_manager))
}

/// 从配置文件创建MemoryManager实例
pub async fn create_memory_manager_from_config(config_path: &str) -> Result<Arc<MemoryManager>> {
    // 加载配置
    let config = Config::load(config_path)?;
    
                    // 简化：默认使用内存向量存储
                    info!("使用内存向量存储（评估模式）");
                    let vector_store: Box<dyn cortex_mem_core::VectorStore> = Box::new(InMemoryVectorStore::new());    // 创建模拟LLM客户端（评估模式下使用模拟客户端）
    let llm_client = Box::new(MockLLMClient::new(false));
    
    // 创建MemoryManager
    let memory_manager = MemoryManager::new(vector_store, llm_client, config.memory);
    
    Ok(Arc::new(memory_manager))
}

/// 创建用于测试的简单MemoryManager
pub async fn create_simple_memory_manager() -> Result<Arc<MemoryManager>> {
    create_memory_manager_for_evaluation().await
}

/// 创建用于真实评估的MemoryManager（根据评估配置）
pub async fn create_memory_manager_for_real_evaluation(evaluation_config: &crate::runner::ExperimentConfig) -> Result<Arc<MemoryManager>> {
    // 使用评估配置中的memory_config_path，如果未指定则使用默认路径
    let config_path = evaluation_config.memory_config_path
        .as_deref()
        .unwrap_or("config/evaluation_config.toml");
    
    info!("使用配置文件路径: {}", config_path);
    create_memory_manager_from_config(config_path).await
}