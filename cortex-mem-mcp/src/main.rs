use anyhow::{anyhow, Result};
use clap::Parser;
use cortex_mem_core::*;
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

    /// Agent identifier for memory operations
    #[arg(long)]
    agent: Option<String>,

    /// User identifier for memory operations
    #[arg(long)]
    user: Option<String>,
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
    
    if let Some(ref agent_id) = cli.agent {
        info!("Default agent ID: {}", agent_id);
    }
    if let Some(ref user_id) = cli.user {
        info!("Default user ID: {}", user_id);
    }

    // Load configuration
    let _config = load_config(&cli.config).await?;

    // Initialize filesystem
    let data_path = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Failed to get data directory"))?
        .join("cortex-mem");
    
    let filesystem = CortexFilesystem::new(&data_path);
    let filesystem = Arc::new(filesystem);

    // Create the service
    let service = MemoryMcpService::new(
        filesystem,
        cli.agent,
        cli.user,
    );

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

async fn load_config(_config_path: &PathBuf) -> Result<()> {
    // For now, just return OK
    // In the future, we can load LLM config from config.toml
    Ok(())
}
