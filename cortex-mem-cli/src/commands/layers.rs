use anyhow::Result;
use cortex_mem_core::automation::{LayerGenerator, LayerGenerationConfig};
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// 确保所有目录拥有 L0/L1 文件
pub async fn ensure_all(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("🔍 扫描文件系统，检查缺失的 .abstract.md 和 .overview.md 文件...\n");
    
    // 从 session_manager 中获取 LLM client
    let llm_client = {
        let sm = operations.session_manager().read().await;
        sm.llm_client()
            .ok_or_else(|| anyhow::anyhow!("LLM client not available"))?
            .clone()
    };
    
    // 创建 LayerGenerator
    let config = LayerGenerationConfig::default();
    let generator = LayerGenerator::new(
        operations.filesystem().clone(),
        llm_client,
        config,
    );
    
    // 执行扫描和生成
    let stats = generator.ensure_all_layers().await?;
    
    // 显示结果
    println!("\n✅ 生成完成！");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 统计信息:");
    println!("   • 总计发现缺失: {} 个目录", stats.total);
    println!("   • 成功生成:     {} 个", stats.generated);
    println!("   • 失败:         {} 个", stats.failed);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if stats.failed > 0 {
        println!("\n⚠️  部分目录生成失败，请检查日志获取详细信息");
    }
    
    Ok(())
}

/// 显示层级文件状态
pub async fn status(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("📊 层级文件状态检查\n");
    
    let llm_client = {
        let sm = operations.session_manager().read().await;
        sm.llm_client()
            .ok_or_else(|| anyhow::anyhow!("LLM client not available"))?
            .clone()
    };
    
    let config = LayerGenerationConfig::default();
    let generator = LayerGenerator::new(
        operations.filesystem().clone(),
        llm_client,
        config,
    );
    
    // 扫描所有目录
    let directories = generator.scan_all_directories().await?;
    println!("🗂️  总计目录数: {}\n", directories.len());
    
    // 检测缺失的目录
    let missing = generator.filter_missing_layers(&directories).await?;
    
    let complete = directories.len() - missing.len();
    let complete_percent = if directories.len() > 0 {
        (complete as f64 / directories.len() as f64 * 100.0) as u32
    } else {
        100
    };
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 完整 (有 L0/L1): {} ({:.0}%)", complete, complete_percent);
    println!("❌ 缺失 (无 L0/L1): {} ({:.0}%)", missing.len(), 100 - complete_percent);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if missing.len() > 0 {
        println!("\n💡 提示: 运行 `cortex-mem-cli layers ensure-all` 来生成缺失的文件");
        
        if missing.len() <= 10 {
            println!("\n缺失的目录:");
            for dir in &missing {
                println!("  • {}", dir);
            }
        } else {
            println!("\n缺失的目录 (显示前 10 个):");
            for dir in missing.iter().take(10) {
                println!("  • {}", dir);
            }
            println!("  ... 还有 {} 个", missing.len() - 10);
        }
    }
    
    Ok(())
}

/// 重新生成超大的 .abstract 文件
pub async fn regenerate_oversized(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("🔍 扫描超大的 .abstract.md 文件...\n");
    
    let llm_client = {
        let sm = operations.session_manager().read().await;
        sm.llm_client()
            .ok_or_else(|| anyhow::anyhow!("LLM client not available"))?
            .clone()
    };
    
    let config = LayerGenerationConfig::default();
    let generator = LayerGenerator::new(
        operations.filesystem().clone(),
        llm_client,
        config,
    );
    
    let stats = generator.regenerate_oversized_abstracts().await?;
    
    println!("\n✅ 重新生成完成！");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 统计信息:");
    println!("   • 发现超大文件: {} 个", stats.total);
    println!("   • 成功重新生成: {} 个", stats.regenerated);
    println!("   • 失败:         {} 个", stats.failed);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if stats.total == 0 {
        println!("\n✨ 所有 .abstract 文件大小都在限制范围内！");
    }
    
    Ok(())
}
