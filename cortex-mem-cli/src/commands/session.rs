use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn list(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Listing all sessions", "📋".bold());

    let sessions = operations.list_sessions().await?;

    if sessions.is_empty() {
        println!("\n{} No sessions found", "ℹ".yellow().bold());
        return Ok(());
    }

    println!("\n{} Found {} sessions:", "✓".green().bold(), sessions.len());
    println!();

    for session in sessions {
        println!("• {}", session.thread_id.bright_blue().bold());
        println!("  {}: {}", "Status".dimmed(), session.status);
        println!("  {}: {}", "Created".dimmed(), session.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("  {}: {}", "Updated".dimmed(), session.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!();
    }

    Ok(())
}

pub async fn create(
    operations: Arc<MemoryOperations>,
    thread: &str,
    title: Option<&str>,
) -> Result<()> {
    println!("{} Creating session: {}", "📝".bold(), thread.cyan());

    // Add a system message to create the session
    let message = if let Some(t) = title {
        format!("Session: {}", t)
    } else {
        "Session created".to_string()
    };
    
    operations.add_message(thread, "system", &message).await?;

    println!("{} Session created successfully", "✓".green().bold());
    println!("  {}: {}", "Thread ID".cyan(), thread);
    if let Some(t) = title {
        println!("  {}: {}", "Title".cyan(), t);
    }

    Ok(())
}