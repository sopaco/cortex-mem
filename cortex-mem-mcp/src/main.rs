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
use service::{AutoTriggerConfig, MemoryMcpService};

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

    /// User identifier for memory storage (e.g. your username or unique user ID).
    /// If not specified, defaults to "default".
    /// Note: do NOT use the same value as --tenant. The tenant is an infrastructure
    /// isolation key (e.g. "local-XeStation_zed_agent") while user is the identity
    /// under which memories are stored (cortex://user/{user_id}/...).
    #[arg(long)]
    user: Option<String>,

    /// Message count threshold for auto-trigger (default: 10)
    #[arg(long, default_value = "10")]
    auto_trigger_threshold: usize,

    /// Minimum interval between auto-trigger in seconds (default: 300)
    #[arg(long, default_value = "300")]
    auto_trigger_interval: u64,

    /// Inactivity timeout for auto-trigger in seconds (default: 120)
    #[arg(long, default_value = "120")]
    auto_trigger_inactivity: u64,

    /// Disable auto-trigger feature
    #[arg(long, default_value = "false")]
    no_auto_trigger: bool,

    /// Path to log file. If specified, logs will be written to this file.
    /// If not specified, logging is disabled (MCP protocol uses stdio).
    #[arg(long)]
    log_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging only if --log-file is specified
    // MCP protocol uses stdio for JSON-RPC, so we avoid console output by default
    if let Some(ref log_file) = cli.log_file {
        // Create parent directory if needed
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        
        tracing_subscriber::fmt()
            .with_writer(std::sync::Arc::new(file))
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    info!("Starting Cortex Memory MCP Server");
    info!("Using configuration file: {:?}", cli.config);
    info!("Tenant ID: {}", cli.tenant);
    if let Some(ref uid) = cli.user {
        info!("User ID: {}", uid);
    }

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
        config.qdrant.api_key.as_deref(),
        &config.embedding.api_base_url,
        &config.embedding.api_key,
        &config.embedding.model_name,
        config.qdrant.embedding_dim,
        cli.user,  // explicit user_id; None → "default" (see MemoryOperations::new)
        config.cortex.enable_intent_analysis,
    ).await?;
    
    let operations = Arc::new(operations);
    info!("MemoryOperations initialized successfully");

    // Build auto-trigger configuration from CLI args
    let auto_trigger_config = AutoTriggerConfig {
        message_count_threshold: cli.auto_trigger_threshold,
        min_process_interval_secs: cli.auto_trigger_interval,
        inactivity_timeout_secs: cli.auto_trigger_inactivity,
        enable_auto_trigger: !cli.no_auto_trigger,
    };
    info!(
        "Auto-trigger config: threshold={}, interval={}s, inactivity={}s, enabled={}",
        auto_trigger_config.message_count_threshold,
        auto_trigger_config.min_process_interval_secs,
        auto_trigger_config.inactivity_timeout_secs,
        auto_trigger_config.enable_auto_trigger
    );

    // Create the MCP service with auto-trigger support
    let service = MemoryMcpService::with_config(operations, auto_trigger_config);

    // Start the inactivity checker for auto-triggering
    if auto_trigger_config.enable_auto_trigger {
        service.start_inactivity_checker();
    }

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
