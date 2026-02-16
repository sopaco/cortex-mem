use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::FilesystemOperations;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn execute(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Cortex-Mem Statistics", "📊".bold());
    println!();

    let fs = operations.filesystem();

    // Count sessions
    let session_entries = fs.list("cortex://session").await.unwrap_or_default();
    let session_count = session_entries.iter().filter(|e| e.is_directory && !e.name.starts_with('.')).count();

    println!("{} Dimensions:", "📁".cyan().bold());
    println!("  {}: {}", "Sessions".cyan(), session_count);

    // Count user and agent memories
    for dimension in &["user", "agent"] {
        let dim_uri = format!("cortex://{}", dimension);
        if let Ok(entries) = fs.list(&dim_uri).await {
            let count = entries.iter().filter(|e| e.is_directory && !e.name.starts_with('.')).count();
            println!("  {}: {}", dimension.to_string().cyan(), count);
        }
    }

    println!();

    // Count total messages (simplified)
    let mut total_messages = 0;
    for session_entry in session_entries.iter().filter(|e| e.is_directory && !e.name.starts_with('.')) {
        let timeline_uri = format!("{}/timeline", session_entry.uri);
        if fs.exists(&timeline_uri).await.unwrap_or(false) {
            if let Ok(timeline_entries) = fs.list(&timeline_uri).await {
                total_messages += timeline_entries.iter().filter(|e| !e.is_directory && e.name.ends_with(".md")).count();
            }
        }
    }

    println!("{} Content:", "📝".cyan().bold());
    println!("  {}: ~{}", "Messages".cyan(), total_messages);

    println!();
    println!("{} Storage:", "💾".cyan().bold());
    println!("  {}: {}", "Data directory".cyan(), fs.root_path().display());

    Ok(())
}