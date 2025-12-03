//! Minimal test example for cortex-mem-mcp
//!
//! This example demonstrates the basic functionality of the MemoryMcpService
//! by creating the service and retrieving its info.

use cortex_mem_mcp::MemoryMcpService;
use rmcp::handler::server::ServerHandler;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Cortex Mem MCP Minimal Test");

    // Create the service
    let service = MemoryMcpService::new().await?;

    // Get server info
    println!("\n=== Server Info ===");
    let server_info = service.get_info();
    println!("Protocol: {:?}", server_info.protocol_version);
    println!("Server: {:?}", server_info.server_info.name);

    if let Some(instructions) = &server_info.instructions {
        println!("Instructions: {}", instructions);
    }

    println!("\n=== Test completed successfully! ===");

    Ok(())
}
