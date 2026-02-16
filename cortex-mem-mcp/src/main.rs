use anyhow::{anyhow, Result};
use clap::Parser;
use cortex_mem_config::Config;
use cortex_mem_core::llm::LLMClientImpl;
use cortex_mem_tools::MemoryOperations;
use rmcp::{transport::stdio, ServiceExt};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

mod service;
use service::MemoryMcpService;

#[derive(Parser)]
#[command(name = "cortex-mem-mcp")]
#[command(about = "MCP server for Cortex Memory to enhance agent's memory layer")]
#[command(author = "Cortex-Mem Contributors")]
#[command(version)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Tenant identifier for memory operations
    #[arg(long, default_value = "default")]
    tenant: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Cortex Memory MCP Server");
    info!("Using configuration file: {:?}", cli.config);
    info!("Tenant ID: {}", cli.tenant);

    // Load configuration
    let config = Config::load(&cli.config)?;
    
    // Determine data directory
    let data_dir = config.cortex.data_dir();
    info!("Data directory: {}", data_dir);

    // Initialize LLM client
    let model_name = config.llm.model_efficient.clone();
    let llm_config = cortex_mem_core::llm::LLMConfig {
        api_base_url: config.llm.api_base_url,
        api_key: config.llm.api_key,
        model_efficient: config.llm.model_efficient,
        temperature: config.llm.temperature,
        max_tokens: config.llm.max_tokens as usize,
    };
    let llm_client = Arc::new(LLMClientImpl::new(llm_config)?);
    info!("LLM client initialized with model: {}", model_name);

    // Initialize MemoryOperations with vector search
    let operations = MemoryOperations::new(
        &data_dir,
        &cli.tenant,
        llm_client,
        &config.qdrant.url,
        &config.qdrant.collection_name,
        &config.embedding.api_base_url,
        &config.embedding.api_key,
        &config.embedding.model_name,
        config.qdrant.embedding_dim,
    ).await?;
    
    let operations = Arc::new(operations);
    info!("MemoryOperations initialized successfully");

    // Create the MCP service
    let service = MemoryMcpService::new(operations);

    // Serve the MCP service
    let running_service = service
        .serve(stdio())
        .await
        .map_err(|e| anyhow!("Failed to start MCP server: {}", e))?;

    info!("MCP server initialized successfully");

    // Wait for the server to finish
    match running_service.waiting().await {
        Ok(reason) => info!("Server shutdown: {:?}", reason),
        Err(e) => error!("Server error: {:?}", e),
    }

    Ok(())
}