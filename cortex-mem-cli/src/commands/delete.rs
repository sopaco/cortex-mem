use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn execute(
    operations: Arc<MemoryOperations>,
    uri: &str,
) -> Result<()> {
    println!("{} Deleting memory: {}", "🗑️".bold(), uri.cyan());

    operations.delete(uri).await?;

    println!("{} Memory deleted successfully", "✓".green().bold());

    Ok(())
}