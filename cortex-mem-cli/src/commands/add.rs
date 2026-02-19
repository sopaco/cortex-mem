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
    let message_id = operations.add_message(thread, role, content).await?;

    println!("{} Message added successfully", "✓".green().bold());
    println!("  {}: {}", "Thread".cyan(), thread);
    println!("  {}: {}", "Role".cyan(), role);
    println!("  {}: {}", "ID".cyan(), message_id);

    let uri = format!("cortex://session/{}/timeline/{}.md", thread, message_id);
    println!("  {}: {}", "URI".cyan(), uri.bright_blue());

    Ok(())
}