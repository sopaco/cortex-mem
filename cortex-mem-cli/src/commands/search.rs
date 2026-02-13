use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

#[cfg(feature = "vector-search")]
use std::path::Path;

#[cfg(feature = "vector-search")]
use cortex_mem_core::{EmbeddingClient, VectorSearchEngine, QdrantVectorStore, search::SearchOptions};

use crate::SearchEngine;

/// Execute search command (main entry point from main.rs)
pub async fn execute(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
    engine: SearchEngine,
) -> Result<()> {
    println!("{} Searching for: {}", "🔍".bold(), query.yellow());
    
    match engine {
        SearchEngine::Keyword => {
            search_keyword(fs, query, thread, limit, min_score).await
        },
        #[cfg(feature = "vector-search")]
        SearchEngine::Vector => {
            search_vector_mode(fs, query, thread, limit).await
        },
        #[cfg(feature = "vector-search")]
        SearchEngine::Hybrid => {
            search_hybrid_mode(fs, query, thread, limit).await
        },
        #[cfg(feature = "vector-search")]
        SearchEngine::Layered => {
            search_layered_mode(fs, query, thread, limit).await
        },
    }
}

/// Keyword search using RetrievalEngine
async fn search_keyword(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
    min_score: f32,
) -> Result<()> {
    let scope = if let Some(t) = thread {
        format!("cortex://session/{}", t)
    } else {
        "cortex://session".to_string()
    };
    
    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    println!("  {} Strategy: {}", "⚙".dimmed(), "Keyword (BM25-like)".cyan());
    
    // Use RetrievalEngine
    let layer_manager = Arc::new(LayerManager::new(fs.clone()));
    let engine = RetrievalEngine::new(fs, layer_manager);
    
    let options = RetrievalOptions {
        top_k: limit,
        min_score,
        load_details: true,
        max_candidates: limit * 2,
    };
    
    println!("  {} Searching...", "🔍".dimmed());
    let result = engine.search(query, &scope, options).await?;
    
    // Display results
    if result.results.is_empty() {
        println!("\n{} No results found", "ℹ".yellow().bold());
        return Ok(());
    }
    
    println!("\n{} Found {} results:", "✓".green().bold(), result.results.len());
    println!();
    
    for (i, res) in result.results.iter().enumerate() {
        println!("{} {} (score: {:.3})", 
            format!("{}.", i + 1).dimmed(),
            res.uri.bright_blue(),
            res.score
        );
        println!("   {}", res.snippet.dimmed());
        println!();
    }
    
    Ok(())
}

/// Vector search using VectorSearchEngine
#[cfg(feature = "vector-search")]
async fn search_vector_mode(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
) -> Result<()> {
    println!("  {} Strategy: {}", "⚙".dimmed(), "Vector (Semantic)".bright_magenta());
    
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
    
    // Create Qdrant store
    let llm_config = load_llm_config_from_toml("config.toml")?;
    let llm_client = cortex_mem_core::llm::LLMClientImpl::new(llm_config)?;
    let qdrant = Arc::new(
        QdrantVectorStore::new_with_llm_client(&qdrant_config, &llm_client).await?
    );
    
    // Create search engine
    let engine = VectorSearchEngine::new(qdrant, embedding, fs.clone());
    
    // Search
    let scope = if let Some(t) = thread {
        format!("cortex://session/{}", t)
    } else {
        "cortex://session".to_string()
    };
    
    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    
    let options = SearchOptions {
        limit,
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

/// Hybrid search using MemoryOperations
#[cfg(feature = "vector-search")]
async fn search_hybrid_mode(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
) -> Result<()> {
    use cortex_mem_core::SessionManager;
    use tokio::sync::RwLock;
    
    println!("  {} Strategy: {}", "⚙".dimmed(), "Hybrid (Keyword + Vector)".bright_magenta());
    
    // Load configs and initialize
    if !Path::new("config.toml").exists() {
        anyhow::bail!("config.toml not found. Hybrid search requires Qdrant and embedding configurations.");
    }
    
    println!("  {} Loading config from config.toml", "⚙".dimmed());
    let (qdrant_config, embedding_config) = load_vector_configs("config.toml")?;
    
    // Initialize clients
    println!("  {} Initializing clients", "🔌".dimmed());
    let embedding = Arc::new(EmbeddingClient::new(embedding_config)?);
    let llm_config = load_llm_config_from_toml("config.toml")?;
    let llm_client = cortex_mem_core::llm::LLMClientImpl::new(llm_config)?;
    let qdrant = Arc::new(
        QdrantVectorStore::new_with_llm_client(&qdrant_config, &llm_client).await?
    );
    
    // Create session manager
    let session_config = cortex_mem_core::SessionConfig::default();
    let session_manager = SessionManager::new(fs.clone(), session_config);
    let _session_manager = Arc::new(RwLock::new(session_manager));
    
    // Create engines
    let layer_manager = Arc::new(LayerManager::new(fs.clone()));
    let vector_engine = Arc::new(VectorSearchEngine::new(qdrant, embedding, fs.clone()));
    use cortex_mem_core::RetrievalEngine;
    use cortex_mem_core::search::SearchOptions as CoreSearchOptions;
    
    let scope = if let Some(t) = thread {
        format!("cortex://session/{}", t)
    } else {
        "cortex://session".to_string()
    };
    
    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    println!("  {} Performing hybrid search...", "🚀".dimmed());
    
    // Execute keyword search
    let keyword_engine = RetrievalEngine::new(fs.clone(), layer_manager.clone());
    let keyword_options = cortex_mem_core::RetrievalOptions {
        top_k: limit,
        min_score: 0.3,
        load_details: true,
        max_candidates: limit * 2,
    };
    let keyword_result = keyword_engine.search(query, &scope, keyword_options).await?;
    
    // Execute vector search  
    let vector_options = CoreSearchOptions {
        limit,
        threshold: 0.5,
        root_uri: Some(scope.clone()),
        recursive: true,
    };
    let vector_results = vector_engine.semantic_search(query, &vector_options).await?;
    
    // Merge results with 0.5/0.5 weights (simple hybrid)
    use std::collections::HashMap;
    let mut combined: HashMap<String, f32> = HashMap::new();
    
    for res in keyword_result.results {
        combined.insert(res.uri.clone(), res.score * 0.5);
    }
    
    for res in vector_results {
        combined.entry(res.uri.clone())
            .and_modify(|score| *score += res.score * 0.5)
            .or_insert(res.score * 0.5);
    }
    
    let mut results: Vec<(String, f32)> = combined.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results.truncate(limit);
    
    // Display results
    if results.is_empty() {
        println!("\n{} No results found", "ℹ".yellow().bold());
        return Ok(());
    }
    
    println!("\n{} Found {} hybrid matches:", "✓".green().bold(), results.len());
    println!();
    
    for (i, (uri, score)) in results.iter().enumerate() {
        println!("{} {} (combined score: {:.3})", 
            format!("{}.", i + 1).dimmed(),
            uri.bright_blue(),
            score
        );
        // Read and display snippet
        if let Ok(content) = fs.read(uri).await {
            let snippet = if content.len() > 100 {
                format!("{}...", &content[..100])
            } else {
                content
            };
            println!("   {}", snippet.dimmed());
        }
        println!();
    }
    
    Ok(())
}

/// Layered semantic search using VectorSearchEngine
#[cfg(feature = "vector-search")]
async fn search_layered_mode(
    fs: Arc<CortexFilesystem>,
    query: &str,
    thread: Option<&str>,
    limit: usize,
) -> Result<()> {
    println!("  {} Strategy: {}", "⚙".dimmed(), "Layered Vector (L0→L1→L2)".bright_magenta());
    
    // Load configs
    if !Path::new("config.toml").exists() {
        anyhow::bail!("config.toml not found. Layered search requires Qdrant and embedding configurations.");
    }
    
    println!("  {} Loading config from config.toml", "⚙".dimmed());
    let (qdrant_config, embedding_config) = load_vector_configs("config.toml")?;
    
    // Initialize clients
    println!("  {} Initializing clients", "🔌".dimmed());
    let embedding = Arc::new(EmbeddingClient::new(embedding_config)?);
    let llm_config = load_llm_config_from_toml("config.toml")?;
    let llm_client = cortex_mem_core::llm::LLMClientImpl::new(llm_config)?;
    let qdrant = Arc::new(
        QdrantVectorStore::new_with_llm_client(&qdrant_config, &llm_client).await?
    );
    
    // Create search engine
    let engine = VectorSearchEngine::new(qdrant, embedding, fs.clone());
    
    // Search
    let scope = if let Some(t) = thread {
        format!("cortex://session/{}", t)
    } else {
        "cortex://session".to_string()
    };
    
    println!("  {} Scope: {}", "📂".dimmed(), scope.dimmed());
    
    let options = SearchOptions {
        limit,
        threshold: 0.5,
        root_uri: Some(scope.clone()),
        recursive: true,
    };
    
    println!("  {} Performing layered search...", "🎯".dimmed());
    let results = engine.layered_semantic_search(query, &options).await?;
    
    // Display results
    if results.is_empty() {
        println!("\n{} No results found", "ℹ".yellow().bold());
        return Ok(());
    }
    
    println!("\n{} Found {} layered matches:", "✓".green().bold(), results.len());
    println!();
    
    for (i, res) in results.iter().enumerate() {
        println!("{} {} (combined score: {:.3})", 
            format!("{}.", i + 1).dimmed(),
            res.uri.bright_blue(),
            res.score
        );
        println!("   {}", res.snippet.dimmed());
        println!();
    }
    
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
