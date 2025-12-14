use cortex_mem_evaluation::memory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("测试MemoryManager创建...");
    
    // 创建MemoryManager实例
    let memory_manager = memory::create_memory_manager_for_evaluation().await?;
    println!("✓ MemoryManager实例创建成功");
    
    // 测试添加记忆
    println!("测试添加记忆...");
    let memory = cortex_mem_core::Memory {
        id: "test_memory_001".to_string(),
        content: "这是一个测试记忆，用于验证MemoryManager功能。".to_string(),
        embedding: vec![], // 实际使用时需要生成嵌入
        metadata: cortex_mem_core::MemoryMetadata {
            user_id: Some("test_user".to_string()),
            agent_id: None,
            run_id: None,
            actor_id: None,
            role: None,
            memory_type: cortex_mem_core::MemoryType::Conversational,
            hash: "test_hash".to_string(),
            importance_score: 5.0,
            entities: vec!["测试".to_string()],
            topics: vec!["验证".to_string(), "功能".to_string()],
            custom: std::collections::HashMap::new(),
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let result = memory_manager.add_memory(memory).await;
    match result {
        Ok(_) => println!("✓ 记忆添加成功"),
        Err(e) => println!("✗ 记忆添加失败: {}", e),
    }
    
    // 测试搜索记忆
    println!("测试搜索记忆...");
    let search_result = memory_manager.search_memories("测试记忆", None, None, None).await;
    match search_result {
        Ok(results) => println!("✓ 搜索成功，找到 {} 个结果", results.len()),
        Err(e) => println!("✗ 搜索失败: {}", e),
    }
    
    println!("测试完成！");
    Ok(())
}