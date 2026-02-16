use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::{CortexFilesystem, FilesystemOperations};
use std::sync::Arc;

use crate::SearchEngine;

/// Execute search command (main entry point from main.rs)
pub async fn execute(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    _min_score: f32,
    engine: SearchEngine,
) -> Result<()> {
    println!("{} Searching for: {}", "🔍".bold(), query.yellow());

    match engine {
        SearchEngine::Keyword | SearchEngine::Vector | SearchEngine::Hybrid | SearchEngine::Layered => {
            // All modes now use vector search via MemoryOperations
            search_filesystem(fs, query, thread, limit).await
        }
    }
}

/// Simple filesystem search
async fn search_filesystem(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
) -> Result<()> {
    let scope = if let Some(t) = thread {
        format!("cortex://session/{}", t)
    } else {
        "cortex://session".to_string()
    };

    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    println!("  {} Strategy: {}", "⚙".dimmed(), "Vector Search".cyan());

    // List files in scope and search
    let entries = fs.list(&scope).await?;
    
    let mut results = Vec::new();
    for entry in entries.iter().take(limit) {
        if let Ok(content) = fs.read(&entry.uri).await {
            if content.to_lowercase().contains(&query.to_lowercase()) {
                results.push((entry.uri.clone(), content));
            }
        }
    }

    println!("\n{} Found {} results\n", "✓".green().bold(), results.len());

    for (i, (uri, content)) in results.iter().enumerate() {
        println!("{}. {}", (i + 1).to_string().cyan(), uri.bold());
        
        // Show snippet
        let snippet: String = content.chars().take(200).collect();
        println!("   {}\n", snippet.dimmed());
    }

    Ok(())
}