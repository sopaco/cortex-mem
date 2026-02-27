use anyhow::Result;
use cortex_mem_core::automation::{LayerGenerator, LayerGenerationConfig};
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// Ensure all directories have L0/L1 files
pub async fn ensure_all(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("🔍 Scanning filesystem for missing .abstract.md and .overview.md files...\n");
    
    // Get LLM client from session_manager
    let llm_client = {
        let sm = operations.session_manager().read().await;
        sm.llm_client()
            .ok_or_else(|| anyhow::anyhow!("LLM client not available"))?
            .clone()
    };
    
    // Create LayerGenerator
    let config = LayerGenerationConfig::default();
    let generator = LayerGenerator::new(
        operations.filesystem().clone(),
        llm_client,
        config,
    );
    
    // Execute scan and generation
    let stats = generator.ensure_all_layers().await?;
    
    // Display results
    println!("\n✅ Generation complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Statistics:");
    println!("   • Total missing:   {} directories", stats.total);
    println!("   • Generated:       {}", stats.generated);
    println!("   • Failed:          {}", stats.failed);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if stats.failed > 0 {
        println!("\n⚠️  Some directories failed to generate. Check logs for details.");
    }
    
    Ok(())
}

/// Display layer file status
pub async fn status(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("📊 Layer file status check\n");
    
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
    
    // Scan all directories
    let directories = generator.scan_all_directories().await?;
    println!("🗂️  Total directories: {}\n", directories.len());
    
    // Detect missing directories
    let missing = generator.filter_missing_layers(&directories).await?;
    
    let complete = directories.len() - missing.len();
    let complete_percent = if directories.len() > 0 {
        (complete as f64 / directories.len() as f64 * 100.0) as u32
    } else {
        100
    };
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Complete (has L0/L1): {} ({:.0}%)", complete, complete_percent);
    println!("❌ Missing (no L0/L1):   {} ({:.0}%)", missing.len(), 100 - complete_percent);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if missing.len() > 0 {
        println!("\n💡 Tip: Run `cortex-mem layers ensure-all` to generate missing files");
        
        if missing.len() <= 10 {
            println!("\nMissing directories:");
            for dir in &missing {
                println!("  • {}", dir);
            }
        } else {
            println!("\nMissing directories (showing first 10):");
            for dir in missing.iter().take(10) {
                println!("  • {}", dir);
            }
            println!("  ... and {} more", missing.len() - 10);
        }
    }
    
    Ok(())
}

/// Regenerate oversized .abstract files
pub async fn regenerate_oversized(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("🔍 Scanning for oversized .abstract.md files...\n");
    
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
    
    println!("\n✅ Regeneration complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Statistics:");
    println!("   • Oversized files found:    {}", stats.total);
    println!("   • Successfully regenerated: {}", stats.regenerated);
    println!("   • Failed:                   {}", stats.failed);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    if stats.total == 0 {
        println!("\n✨ All .abstract files are within size limits!");
    }
    
    Ok(())
}
