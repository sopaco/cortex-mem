use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

pub async fn execute(fs: Arc<CortexFilesystem>, uri: &str) -> Result<()> {
    println!("{} Deleting memory: {}", "🗑️".bold(), uri.cyan());

    // Check if exists
    if !fs.exists(uri).await? {
        eprintln!("{} Memory not found: {}", "Error:".red().bold(), uri);
        return Ok(());
    }

    // Delete
    fs.delete(uri).await?;

    println!("{} Memory deleted successfully", "✓".green().bold());

    Ok(())
}
