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
    let qdrant = Arc::new(
        QdrantVectorStore::new(&qdrant_config).await?
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

    Ok(())
}

/// Extract memories from a session
pub async fn extract_session(
    fs: Arc<CortexFilesystem>,
    thread: &str,
) -> Result<()> {
    println!("{} Extracting memories from session: {}", "🧠".bold(), thread.cyan());

    // TODO: Implement using MemoryExtractor
    println!("  {} Memory extraction not yet implemented in CLI", "⚠".yellow());

    Ok(())
}

/// Load vector search configurations from config file
fn load_vector_configs(config_path: &str) -> Result<(QdrantConfig, embedding::EmbeddingConfig)> {
    let config = cortex_mem_config::Config::load(config_path)?;
    
    let qdrant_config = QdrantConfig {
        url: config.qdrant.url,
        collection_name: config.qdrant.collection_name,
        embedding_dim: config.qdrant.embedding_dim,
        timeout_secs: config.qdrant.timeout_secs,
    };
    
    let embedding_config = embedding::EmbeddingConfig {
        api_base_url: config.embedding.api_base_url,
        api_key: config.embedding.api_key,
        model_name: config.embedding.model_name,
        batch_size: config.embedding.batch_size,
        timeout_secs: config.embedding.timeout_secs,
    };
    
    Ok((qdrant_config, embedding_config))
}