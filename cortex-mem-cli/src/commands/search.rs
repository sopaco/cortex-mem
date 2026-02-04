use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

pub async fn execute(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<()> {
    let layer_manager = Arc::new(LayerManager::new(fs.clone()));
    let engine = RetrievalEngine::new(fs.clone(), layer_manager);

    // Determine search scope
    let scope = if let Some(thread_id) = thread {
        format!("cortex://threads/{}", thread_id)
    } else {
        "cortex://".to_string()
    };

    println!("{} Searching for: {}", "🔍".bold(), query.cyan().bold());
    if let Some(thread_id) = thread {
        println!("  {}: {}", "Scope".cyan(), thread_id);
    } else {
        println!("  {}: All threads", "Scope".cyan());
    }

    // Execute search
    let options = RetrievalOptions {
        top_k: limit,
        min_score,
        load_details: false,
        max_candidates: 50,
    };

    let result = engine.search(query, &scope, options).await?;

    // Display results
    if result.results.is_empty() {
        println!("\n{} No results found", "ℹ".yellow().bold());
        return Ok(());
    }

    println!("\n{} Found {} results:", "✓".green().bold(), result.results.len());
    println!();

    for (i, search_result) in result.results.iter().enumerate() {
        println!("{}. {} (score: {:.2})", 
            (i + 1).to_string().bold(),
            search_result.uri.bright_blue(),
            search_result.score
        );
        
        // Show snippet
        let snippet = search_result.snippet.trim();
        if !snippet.is_empty() {
            let preview = if snippet.len() > 100 {
                format!("{}...", &snippet[..97])
            } else {
                snippet.to_string()
            };
            println!("   {}", preview.dimmed());
        }
        println!();
    }

    // Show trace summary
    println!("{} Retrieval trace:", "📊".bold());
    for step in &result.trace.steps {
        println!("  • {:?}: {} candidates ({}ms)",
            step.step_type,
            step.candidates_count,
            step.duration_ms
        );
    }
    println!("  {}: {}ms\n", "Total".cyan(), result.trace.total_duration_ms);

    Ok(())
}
