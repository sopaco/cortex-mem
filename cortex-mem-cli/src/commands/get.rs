use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

pub async fn execute(fs: Arc<CortexFilesystem>, uri: &str) -> Result<()> {
    println!("{} Getting memory: {}", "🔍".bold(), uri.cyan());

    // Check if exists
    if !fs.exists(uri).await? {
        eprintln!("{} Memory not found: {}", "Error:".red().bold(), uri);
        return Ok(());
    }

    // Read content
    let content = fs.read(uri).await?;

    println!("\n{}", "─".repeat(80).dimmed());
    println!("{}", content);
    println!("{}\n", "─".repeat(80).dimmed());

    // Show metadata
    if let Ok(meta) = fs.metadata(uri).await {
        println!("{} Metadata:", "ℹ".cyan().bold());
        println!("  {}: {}", "Size".cyan(), format!("{} bytes", meta.size).dimmed());
    }

    Ok(())
}
