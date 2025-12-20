use anyhow::anyhow;
use clap::Parser;
use cortex_mem_mcp::MemoryMcpService;
use rmcp::{transport::stdio, ServiceExt};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "cortex-mem-mcp")]
#[command(about = "MCP server of Cortex Memory to enhance agent's memory layer")]
#[command(author = "Sopaco")]
#[command(version)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Agent identifier for memory operations
    #[arg(long)]
    agent: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Cortex Memo MCP Server");
    info!("Using configuration file: {:?}", cli.config);

    // Create the service
    let service = MemoryMcpService::with_config_path_and_agent(cli.config, cli.agent)
        .await
        .map_err(|e| anyhow!("Failed to initialize memory management service: {}", e))?;

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
