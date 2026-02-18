#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[dependencies]
cortex-mem-core = { path = "./cortex-mem-core" }
tokio = { version = "1", features = ["full"] }
tracing-subscriber = "0.3"
anyhow = "1"
---

//! 临时脚本：直接索引timeline L0/L1层
//! 
//! 用法: cargo +nightly -Zscript scripts/index_timeline_layers.rs

use cortex_mem_core::*;
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    println!("🚀 开始索引timeline L0/L1层...\n");
    
    // 配置
    let data_dir = "/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars";
    let tenant_id = "bf323233-1f53-4337-a8e7-2ebe9b0080d0";
    let thread_id = "431d5be1-4a97-4c42-81ec-d17f413b04ab";
    
    // 初始化组件
    println!("📂 数据目录: {}", data_dir);
    println!("👤 租户ID: {}", tenant_id);
    println!("🧵 Thread ID: {}\n", thread_id);
    
    // 1. 创建filesystem（带租户隔离）
    let tenant_root = PathBuf::from(data_dir)
        .join("cortex/tenants")
        .join(tenant_id);
    let filesystem = Arc::new(CortexFilesystem::new(tenant_root));
    
    // 2. 创建embedding client
    let embedding_config = embedding::EmbeddingConfig {
        api_base_url: "https://api.deepseek.com/v1".to_string(),
        api_key: std::env::var("DEEPSEEK_API_KEY")
            .expect("需要设置 DEEPSEEK_API_KEY 环境变量"),
        model: "deepseek-chat".to_string(),
        embedding_dim: 1536,
        timeout_secs: 30,
    };
    let embedding = Arc::new(embedding::EmbeddingClient::new(embedding_config)?);
    
    // 3. 创建Qdrant vector store
    let qdrant_config = vector_store::QdrantConfig {
        url: "http://localhost:6334".to_string(),
        api_key: None,
        collection_name: "cortex_memories".to_string(),
        embedding_dim: 1536,
    };
    let vector_store = Arc::new(vector_store::QdrantVectorStore::new(qdrant_config).await?);
    
    // 4. 创建AutoIndexer
    let indexer_config = automation::IndexerConfig {
        auto_index: true,
        batch_size: 10,
        async_index: false,
    };
    let indexer = automation::AutoIndexer::new(
        filesystem.clone(),
        embedding.clone(),
        vector_store.clone(),
        indexer_config,
    );
    
    // 5. 索引L2消息（先确保L2都被索引）
    println!("📝 步骤1: 索引L2消息层...");
    match indexer.index_thread(&thread_id).await {
        Ok(stats) => {
            println!("✅ L2索引完成:");
            println!("   - 已索引: {}", stats.total_indexed);
            println!("   - 已跳过: {}", stats.total_skipped);
            println!("   - 错误数: {}\n", stats.total_errors);
        }
        Err(e) => {
            println!("⚠️  L2索引失败（可能timeline路径不存在）: {}\n", e);
            println!("继续尝试直接索引L0/L1...\n");
        }
    }
    
    // 6. 手动触发L0/L1索引（因为上面的index_thread可能失败）
    println!("📊 步骤2: 手动索引L0/L1层...");
    
    // 直接调用私有方法的替代方案：使用SyncManager
    use cortex_mem_core::automation::SyncManager;
    use cortex_mem_core::llm::{LLMClient, LLMConfig};
    
    // 创建LLM client（用于layer generation）
    let llm_config = LLMConfig {
        api_base_url: "https://api.deepseek.com/v1".to_string(),
        api_key: std::env::var("DEEPSEEK_API_KEY")
            .expect("需要设置 DEEPSEEK_API_KEY 环境变量"),
        model: "deepseek-chat".to_string(),
        temperature: 0.3,
        max_tokens: 2000,
    };
    let llm_client: Arc<dyn LLMClient> = Arc::new(
        cortex_mem_core::llm::LLMClientImpl::new(llm_config)?
    );
    
    let sync_manager = SyncManager::new(
        filesystem.clone(),
        embedding.clone(),
        vector_store.clone(),
        llm_client.clone(),
    );
    
    // 同步整个session目录（会自动生成和索引L0/L1）
    let timeline_uri = format!("cortex://session/{}", thread_id);
    match sync_manager.sync_uri(&timeline_uri).await {
        Ok(stats) => {
            println!("✅ L0/L1索引完成:");
            println!("   - 总文件数: {}", stats.total_files);
            println!("   - 已索引: {}", stats.indexed_files);
            println!("   - 已跳过: {}", stats.skipped_files);
            println!("   - 错误数: {}\n", stats.error_files);
        }
        Err(e) => {
            eprintln!("❌ L0/L1索引失败: {}", e);
            std::process::exit(1);
        }
    }
    
    println!("🎉 索引完成！现在可以搜索'杨雪'了。");
    
    Ok(())
}
