use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

/// Build index for a session
pub async fn index_session(
    fs: Arc<CortexFilesystem>,
    thread: &str,
) -> Result<()> {
    println!("{} Building vector index for session: {}", "🔍".bold(), thread.cyan());

    #[cfg(not(feature = "vector-search"))]
    {
        println!("{} Vector search feature is not enabled", "⚠".yellow().bold());
        println!("  Rebuild with: cargo build --features vector-search");
        return Ok(());
    }

    #[cfg(feature = "vector-search")]
    {
        use std::path::Path;

        // Load configs
        if !Path::new("config.toml").exists() {
            anyhow::bail!("config.toml not found. Vector index requires Qdrant and embedding configurations.");
        }

        println!("  {} Loading config from config.toml", "⚙".dimmed());
        let (qdrant_config, embedding_config) = load_vector_configs("config.toml")?;

        // Initialize clients
        println!("  {} Initializing Qdrant and embedding clients", "🔌".dimmed());
        let embedding = Arc::new(EmbeddingClient::new(embedding_config)?);

        // Create Qdrant store
        let llm_config = load_llm_config_from_toml("config.toml")?;
        let llm_client = LLMClient::new(llm_config)?;
        let qdrant = Arc::new(
            QdrantVectorStore::new_with_llm_client(&qdrant_config, &llm_client).await?
        );

        // Create indexer
        let indexer_config = IndexerConfig::default();
        let indexer = AutoIndexer::new(fs.clone(), embedding.clone(), qdrant, indexer_config);

        // Index the session
        println!("  {} Indexing messages...", "📝".dimmed());
        let stats = indexer.index_thread(thread).await?;

        println!("{} Indexing complete", "✓".green().bold());
        println!("  {}: {}", "Indexed".cyan(), stats.total_indexed);
        println!("  {}: {}", "Skipped".cyan(), stats.total_skipped);
        println!("  {}: {}", "Errors".cyan(), stats.total_errors);

        Ok(())
    }
}

/// Auto-extract memories when closing a session
pub async fn auto_extract_on_close(
    fs: Arc<CortexFilesystem>,
    thread: &str,
    llm: Arc<LLMClient>,
) -> Result<()> {
    println!("{} Auto-extracting memories on session close: {}", "🧠".bold(), thread.cyan());

    let config = AutoExtractConfig::default();
    let extractor = AutoExtractor::new(fs.clone(), llm, config);

    let stats = extractor.extract_session(thread).await?;

    println!("{} Auto-extraction complete", "✓".green().bold());
    println!("  {}: {}", "Facts".cyan(), stats.facts_extracted);
    println!("  {}: {}", "Decisions".cyan(), stats.decisions_extracted);
    println!("  {}: {}", "Entities".cyan(), stats.entities_extracted);
    println!("  {}: {}", "User memories saved".cyan(), stats.user_memories_saved);
    println!("  {}: {}", "Agent memories saved".cyan(), stats.agent_memories_saved);

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
