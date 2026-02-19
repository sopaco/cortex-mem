use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn execute(
    operations: Arc<MemoryOperations>,
    uri: &str,
    abstract_only: bool,
) -> Result<()> {
    println!("{} Getting memory: {}", "🔍".bold(), uri.cyan());

    if abstract_only {
        // Get abstract (L0 layer)
        let abstract_result = operations.get_abstract(uri).await?;
        
        println!("\n{}", "─".repeat(80).dimmed());
        println!("{} Abstract (L0)", "📝".bold());
        println!("{}\n", "─".repeat(80).dimmed());
        println!("{}", abstract_result.abstract_text);
        println!("{}\n", "─".repeat(80).dimmed());
    } else {
        // Get full content
        let content = operations.read_file(uri).await?;
        
        println!("\n{}", "─".repeat(80).dimmed());
        println!("{}", content);
        println!("{}\n", "─".repeat(80).dimmed());
    }

    Ok(())
}