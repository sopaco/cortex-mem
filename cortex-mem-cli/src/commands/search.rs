use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::SearchOptions;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn execute(
    operations: Arc<MemoryOperations>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
    scope: &str,
) -> Result<()> {
    // Validate min_score parameter
    if min_score < 0.0 || min_score > 1.0 {
        return Err(anyhow::anyhow!(
            "min_score must be between 0.0 and 1.0, got {:.2}",
            min_score
        ));
    }
    
    // Validate limit parameter
    if limit == 0 {
        return Err(anyhow::anyhow!("limit must be greater than 0"));
    }

    println!("{} Searching for: {}", "🔍".bold(), query.yellow());

    // Build search scope URI
    let scope_uri = if let Some(t) = thread {
        format!("cortex://session/{}", t)
    } else {
        match scope {
            "session" => "cortex://session".to_string(),
            "user" => "cortex://user".to_string(),
            "agent" => "cortex://agent".to_string(),
            _ => "cortex://session".to_string(),
        }
    };

    println!("  {} Scope: {}", "📂".dimmed(), scope_uri.dimmed());
    println!("  {} Strategy: {}", "⚙".dimmed(), "Vector Search".cyan());

    // Configure search options
    let options = SearchOptions {
        limit,
        threshold: min_score,
        root_uri: Some(scope_uri.clone()),
        recursive: true,
    };

    // Perform vector search
    let results = operations.vector_engine()
        .semantic_search(query, &options)
        .await?;

    println!("\n{} Found {} results\n", "✓".green().bold(), results.len());

    for (i, result) in results.iter().enumerate() {
        println!("{}. {} (score: {:.2})", 
            (i + 1).to_string().cyan(), 
            result.uri.bold(),
            result.score
        );
        
        // Show snippet
        if !result.snippet.is_empty() {
            let display_snippet: String = result.snippet.chars().take(200).collect();
            println!("   {}\n", display_snippet.dimmed());
        }
    }

    Ok(())
}
