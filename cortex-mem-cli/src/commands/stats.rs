use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

pub async fn execute(fs: Arc<CortexFilesystem>) -> Result<()> {
    println!("{} Cortex-Mem Statistics", "📊".bold());
    println!();

    // Count threads
    let threads_uri = "cortex://threads";
    let thread_entries = fs.list(threads_uri).await.unwrap_or_default();
    let thread_count = thread_entries.iter().filter(|e| e.is_directory && !e.name.starts_with('.')).count();

    println!("{} Dimensions:", "📁".cyan().bold());
    println!("  {}: {}", "Threads".cyan(), thread_count);

    // Try to count agents, users, global
    for dimension in &["agents", "users", "global"] {
        let dim_uri = format!("cortex://{}", dimension);
        if let Ok(entries) = fs.list(&dim_uri).await {
            let count = entries.iter().filter(|e| e.is_directory && !e.name.starts_with('.')).count();
            println!("  {}: {}", dimension.to_string().cyan(), count);
        }
    }

    println!();

    // Count total messages (simplified - just counts files in threads)
    let mut total_messages = 0;
    for thread_entry in thread_entries.iter().filter(|e| e.is_directory && !e.name.starts_with('.')) {
        let timeline_uri = format!("{}/timeline", thread_entry.uri);
        if fs.exists(&timeline_uri).await.unwrap_or(false) {
            // This is simplified - would need recursive counting
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
