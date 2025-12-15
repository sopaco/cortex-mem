//! 记忆管理器模块
//! 
//! 提供MemoryManager实例的创建和配置

use anyhow::Result;
use cortex_mem_core::MemoryManager;
use cortex_mem_config::Config;
use std::sync::Arc;
use tracing::info;

/// 从配置文件创建MemoryManager实例
pub async fn create_memory_manager_from_config(config_path: &str) -> Result<Arc<MemoryManager>> {
    // 加载配置
    info!("正在加载配置文件: {}", config_path);
    let config = Config::load(config_path)?;
    
    // 检查Qdrant配置
    info!("配置中的Qdrant URL: {}", config.qdrant.url);
    info!("配置中的集合名称: {}", config.qdrant.collection_name);
    
    // 创建真实LLM客户端
    info!("创建真实LLM客户端...");
    let llm_client = cortex_mem_core::llm::create_llm_client(&config.llm, &config.embedding)
        .map_err(|e| anyhow::anyhow!("创建LLM客户端失败: {}", e))?;
    
    info!("成功创建真实LLM客户端");
    
    // 添加延迟，避免立即调用API导致频率过高
    info!("等待1秒以避免API调用频率过高...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 创建Qdrant向量存储
    info!("正在创建Qdrant向量存储...");
    let qdrant_store = cortex_mem_core::vector_store::qdrant::QdrantVectorStore::new_with_llm_client(
        &config.qdrant,
        llm_client.as_ref()
    ).await
        .map_err(|e| anyhow::anyhow!("创建Qdrant向量存储失败: {}", e))?;
    
    let vector_store = Box::new(qdrant_store);
    
    // 创建MemoryManager
    let memory_manager = MemoryManager::new(vector_store, llm_client, config.memory);
    
    info!("MemoryManager 实例创建成功");
    info!("向量存储类型: Qdrant");
    info!("LLM客户端: 真实客户端");
    
    Ok(Arc::new(memory_manager))
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