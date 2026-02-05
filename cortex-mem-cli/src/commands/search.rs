use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

#[cfg(feature = "vector-search")]
use std::path::Path;

#[cfg(feature = "vector-search")]
use cortex_mem_core::{EmbeddingClient, VectorSearchEngine, QdrantVectorStore, search::SearchOptions};

/// Search for content in the filesystem
pub async fn search_filesystem(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
) -> Result<()> {
    println!("{} Searching for: {}", "🔍".bold(), query.yellow());
    
    let scope = if let Some(t) = thread {
        format!("cortex://threads/{}", t)
    } else {
        "cortex://".to_string()
    };
    
    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    println!("  {} Strategy: {}", "⚙".dimmed(), "Filesystem".dimmed());
    
    // Simple filesystem search
    println!("  {} Searching filesystem...", "🔍".dimmed());
    
    let entries = fs.list(&scope).await?;
    let mut results = Vec::new();
    
    for entry in entries {
        if !entry.is_directory && entry.uri.ends_with(".md") {
            if let Ok(content) = fs.read(&entry.uri).await {
                if content.to_lowercase().contains(&query.to_lowercase()) {
                    results.push((entry.uri, content));
                }
            }
        }
    }
    
    // Display results
    if results.is_empty() {
        println!("\n{} No results found", "ℹ".yellow().bold());
        return Ok(());
    }
    
    println!("\n{} Found {} results:", "✓".green().bold(), results.len());
    println!();
    
    for (i, (uri, content)) in results.iter().enumerate() {
        let snippet = if content.len() > 100 {
            format!("{}...", &content[..100])
        } else {
            content.clone()
        };
        println!("{} {}", 
            format!("{}.", i + 1).dimmed(),
            uri.bright_blue()
        );
        println!("   {}", snippet.dimmed());
        println!();
    }
    
    Ok(())
}

/// Search using vector similarity (requires vector-search feature)
#[cfg(feature = "vector-search")]
pub async fn search_vector(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
) -> Result<()> {
    println!("{} Semantic search for: {}", "🔍".bold(), query.yellow());
    
    // Load configs
    if !Path::new("config.toml").exists() {
        anyhow::bail!("config.toml not found. Vector search requires Qdrant and embedding configurations.");
    }
    
    println!("  {} Loading config from config.toml", "⚙".dimmed());
    let (qdrant_config, embedding_config) = load_vector_configs("config.toml")?;
    
    // Initialize clients
    println!("  {} Initializing Qdrant and embedding clients", "🔌".dimmed());
    let embedding = Arc::new(EmbeddingClient::new(embedding_config)?);
    
    // Auto-detect dimension
    println!("  {} Detecting embedding dimension...", "📏".dimmed());
    let dim = embedding.dimension().await?;
    println!("  {} Dimension: {}", "✓".green(), dim);
    
    // Create Qdrant store with LLM client for dimension detection
    let llm_config = load_llm_config_from_toml("config.toml")?;
    let llm_client = cortex_mem_core::llm::LLMClientImpl::new(llm_config)?;
    let qdrant = Arc::new(
        QdrantVectorStore::new_with_llm_client(&qdrant_config, &llm_client).await?
    );
    
    // Create search engine
    let engine = VectorSearchEngine::new(qdrant, embedding, fs.clone());
    
    // Search
    let scope = if let Some(t) = thread {
        format!("cortex://threads/{}", t)
    } else {
        "cortex://".to_string()
    };
    
    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    println!("  {} Strategy: {}", "⚙".dimmed(), "Vector (Semantic)".bright_magenta());
    
    let options = SearchOptions {
        limit: 10,
        threshold: 0.5,
        root_uri: Some(scope.clone()),
        recursive: true,
    };
    
    println!("  {} Performing semantic search...", "🧠".dimmed());
    let results = engine.semantic_search(query, &options).await?;
    
    // Display results
    if results.is_empty() {
        println!("\n{} No results found", "ℹ".yellow().bold());
        return Ok(());
    }
    
    println!("\n{} Found {} semantic matches:", "✓".green().bold(), results.len());
    println!();
    
    for (i, res) in results.iter().enumerate() {
        println!("{} {} (similarity: {:.3})", 
            format!("{}.", i + 1).dimmed(),
            res.uri.bright_blue(),
            res.score
        );
        println!("   {}", res.snippet.dimmed());
        println!();
    }
    
    Ok(())
}

/// Hybrid search combining filesystem and vector search
#[cfg(feature = "vector-search")]
pub async fn search_hybrid(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
) -> Result<()> {
    println!("{} Hybrid search for: {}", "🔍".bold(), query.yellow());
    println!("  {} Strategy: {}", "⚙".dimmed(), "Hybrid (Filesystem + Vector)".bright_magenta());
    
    // Run both searches in parallel
    println!("  {} Running parallel searches...", "🚀".dimmed());
    
    let fs1 = fs.clone();
    let fs2 = fs.clone();
    let query1 = query.to_string();
    let query2 = query.to_string();
    let thread1 = thread.map(|s| s.to_string());
    let thread2 = thread.map(|s| s.to_string());
    
    let (file_result, vec_result) = tokio::join!(
        async move {
            search_filesystem(fs1, &query1, thread1.as_deref()).await
        },
        async move {
            search_vector(fs2, &query2, thread2.as_deref()).await
        }
    );
    
    // Both must succeed
    file_result?;
    vec_result?;
    
    println!("\n{} Hybrid search complete", "✓".green().bold());
    
    Ok(())
}

// Helper functions

fn load_llm_config_from_toml(path: &str) -> Result<cortex_mem_core::llm::client::LLMConfig> {
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    
    let llm_section = value.get("llm")
        .ok_or_else(|| anyhow::anyhow!("No [llm] section in config.toml"))?;
    
    let config = cortex_mem_core::llm::client::LLMConfig {
        api_base_url: llm_section.get("api_base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing llm.api_base_url"))?
            .to_string(),
        api_key: llm_section.get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing llm.api_key"))?
            .to_string(),
        model_efficient: llm_section.get("model_efficient")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing llm.model_efficient"))?
            .to_string(),
        temperature: llm_section.get("temperature")
            .and_then(|v| v.as_float())
            .unwrap_or(0.1) as f32,
        max_tokens: llm_section.get("max_tokens")
            .and_then(|v| v.as_integer())
            .unwrap_or(4096) as usize,
    };
    
    Ok(config)
}

#[cfg(feature = "vector-search")]
fn load_vector_configs(path: &str) -> Result<(cortex_mem_core::config::QdrantConfig, EmbeddingConfig)> {
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    
    // Parse Qdrant config
    let qdrant_section = value.get("qdrant")
        .ok_or_else(|| anyhow::anyhow!("No [qdrant] section in config.toml"))?;
    
    let qdrant_config = cortex_mem_core::config::QdrantConfig {
        url: qdrant_section.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing qdrant.url"))?
            .to_string(),
        collection_name: qdrant_section.get("collection_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing qdrant.collection_name"))?
            .to_string(),
        embedding_dim: qdrant_section.get("embedding_dim")
            .and_then(|v| v.as_integer())
            .map(|n| n as usize),
        timeout_secs: qdrant_section.get("timeout_secs")
            .and_then(|v| v.as_integer())
            .unwrap_or(30) as u64,
    };
    
    // Parse embedding config
    let embedding_section = value.get("embedding")
        .ok_or_else(|| anyhow::anyhow!("No [embedding] section in config.toml"))?;
    
    let embedding_config = EmbeddingConfig {
        api_base_url: embedding_section.get("api_base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing embedding.api_base_url"))?
            .to_string(),
        api_key: embedding_section.get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing embedding.api_key"))?
            .to_string(),
        model_name: embedding_section.get("model_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing embedding.model_name"))?
            .to_string(),
        batch_size: embedding_section.get("batch_size")
            .and_then(|v| v.as_integer())
            .unwrap_or(10) as usize,
        timeout_secs: embedding_section.get("timeout_secs")
            .and_then(|v| v.as_integer())
            .unwrap_or(30) as u64,
    };
    
    Ok((qdrant_config, embedding_config))
}

/// Execute search command (entry point from main.rs)
pub async fn execute(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    _limit: usize,
    _min_score: f32,
) -> Result<()> {
    // For now, use filesystem search by default
    // Future: add --semantic and --hybrid flags
    search_filesystem(fs, query, thread).await
}
