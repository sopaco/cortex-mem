use anyhow::anyhow;
use cortex_mem_mcp::MemoryMcpService;
use rmcp::{transport::stdio, ServiceExt};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Cortex Memo MCP Server");

    // Create the service
    let service = MemoryMcpService::new()
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
