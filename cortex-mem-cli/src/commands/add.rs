use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn execute(
    operations: Arc<MemoryOperations>,
    thread: &str,
    role: &str,
    content: &str,
) -> Result<()> {
    println!("{} Adding message to session: {}", "📝".bold(), thread.cyan());

    // Add message using MemoryOperations
    // Note: add_message returns the full URI of the message file
    let message_uri = operations.add_message(thread, role, content).await?;

    println!("{} Message added successfully", "✓".green().bold());
    println!("  {}: {}", "Thread".cyan(), thread);
    println!("  {}: {}", "Role".cyan(), role);
    println!("  {}: {}", "URI".cyan(), message_uri.bright_blue());

    Ok(())
}
