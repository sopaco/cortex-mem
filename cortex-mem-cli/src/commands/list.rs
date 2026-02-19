use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::FilesystemOperations;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn execute(
    operations: Arc<MemoryOperations>,
    uri: Option<&str>,
    include_abstracts: bool,
) -> Result<()> {
    let list_uri = uri.unwrap_or("cortex://session");
    
    println!("{} Listing memories from: {}", "📋".bold(), list_uri.cyan());

    // List entries using filesystem
    let entries = operations.filesystem().list(list_uri).await?;

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
            
            // Show abstract if requested
            if include_abstracts {
                if let Ok(abstract_result) = operations.get_abstract(&file.uri).await {
                    let snippet: String = abstract_result.abstract_text.chars().take(100).collect();
                    println!("    {} {}", "Abstract:".dimmed(), snippet.dimmed());
                }
            }
        }
    }

    Ok(())
}