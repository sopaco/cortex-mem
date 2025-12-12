use cortex_mem_config::Config;
use cortex_mem_core::{
    init::initialize_memory_system,
    memory::MemoryManager,
};
use cortex_mem_tools::{MemoryOperations, MemoryOperationPayload};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting memory tools refactor example");

    // Load configuration
    let config = Config::load("config.toml")?;
    info!("Loaded configuration");

    // Initialize memory system
    let (vector_store, llm_client) = initialize_memory_system(&config).await?;
    info!("Initialized memory system");

    // Create memory manager
    let memory_manager = Arc::new(MemoryManager::new(
        vector_store,
        llm_client,
        config.memory.clone(),
    ));
    info!("Created memory manager");

    // Create shared operations
    let operations = MemoryOperations::new(
        memory_manager.clone(),
        Some("default_user".to_string()),
        Some("default_agent".to_string()),
        10,
    );
    info!("Created shared operations");

    // Test 1: Store a memory using the shared operations
    info!("Test 1: Storing a memory");
    let mut store_payload = MemoryOperationPayload::default();
    store_payload.content = Some("This is a test memory for the shared operations demo".to_string());
    store_payload.memory_type = Some("conversational".to_string());
    store_payload.topics = Some(vec!["testing".to_string(), "demo".to_string()]);

    let store_result = operations.store_memory(store_payload).await?;
    info!("Store result: {}", store_result.success);

    // Extract memory_id for use in later tests
    let memory_id = if let Some(data) = store_result.data {
        data.get("memory_id").and_then(|v| v.as_str()).unwrap()
    } else {
        ""
    };

    // Test 2: Query memories using the shared operations
    info!("Test 2: Querying memories");
    let mut query_payload = MemoryOperationPayload::default();
    query_payload.query = Some("test memory".to_string());
    query_payload.limit = Some(5);

    let query_result = operations.query_memory(query_payload).await?;
    info!("Query result: {} (found {} memories)",
          query_result.success,
          query_result.data
              .as_ref()
              .and_then(|d| d.get("count"))
              .and_then(|c| c.as_u64())
              .unwrap_or(0));

    // Test 3: List memories using the shared operations
    info!("Test 3: Listing memories");
    let mut list_payload = MemoryOperationPayload::default();
    list_payload.limit = Some(10);

    let list_result = operations.list_memories(list_payload).await?;
    info!("List result: {} (found {} memories)",
          list_result.success,
          list_result.data
              .as_ref()
              .and_then(|d| d.get("count"))
              .and_then(|c| c.as_u64())
              .unwrap_or(0));

    // Test 4: Get a specific memory using the shared operations
    if !memory_id.is_empty() {
        info!("Test 4: Getting specific memory");
        let mut get_payload = MemoryOperationPayload::default();
        get_payload.memory_id = Some(memory_id.to_string());

        let get_result = operations.get_memory(get_payload).await?;
        info!("Get result: {}", get_result.success);
    }

    info!("Memory tools refactor example completed successfully");
    Ok(())
}
