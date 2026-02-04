use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

pub async fn execute(
    fs: Arc<CortexFilesystem>,
    thread: Option<&str>,
    dimension: Option<&str>,
) -> Result<()> {
    // Determine list scope
    let uri = match (dimension, thread) {
        (Some(dim), Some(thread_id)) => format!("cortex://{}/{}", dim, thread_id),
        (Some(dim), None) => format!("cortex://{}", dim),
        (None, Some(thread_id)) => format!("cortex://threads/{}", thread_id),
        (None, None) => "cortex://".to_string(),
    };

    println!("{} Listing memories from: {}", "📋".bold(), uri.cyan());

    // List entries
    let entries = fs.list(&uri).await?;

    if entries.is_empty() {
        println!("\n{} No memories found", "ℹ".yellow().bold());
        return Ok(());
    }

    println!("\n{} Found {} items:", "✓".green().bold(), entries.len());
    println!();

    // Group by type
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        if entry.name.starts_with('.') {
            continue; // Skip hidden files
        }
        if entry.is_directory {
            dirs.push(entry);
        } else {
            files.push(entry);
        }
    }

    // Display directories
    if !dirs.is_empty() {
        println!("{} Directories ({}):", "📁".bold(), dirs.len());
        for dir in dirs {
            println!("  • {}/", dir.name.bright_blue().bold());
        }
        println!();
    }

    // Display files
    if !files.is_empty() {
        println!("{} Files ({}):", "📄".bold(), files.len());
        for file in files {
            println!("  • {}", file.name);
            println!("    {} bytes", file.size.to_string().dimmed());
        }
    }

    Ok(())
}
