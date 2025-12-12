use cortex_mem_config::Config;
use cortex_mem_core::{init::initialize_memory_system, memory::MemoryManager};
use cortex_mem_rig::tool::MemoryToolConfig;
use cortex_mem_rig::{
    GetMemoryArgs, ListMemoriesArgs, MemoryToolOutput, QueryMemoryArgs, StoreMemoryArgs,
    create_memory_tools,
};
use rig::tool::Tool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    // Note: In a real application, you would initialize logging properly
    // tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::load("config.toml")?;
    println!("Loaded configuration");

    // Initialize memory system
    let (vector_store, llm_client) = initialize_memory_system(&config).await?;
    println!("Initialized vector store and LLM client");

    // Create memory manager
    let memory_manager = Arc::new(MemoryManager::new(
        vector_store,
        llm_client,
        config.memory.clone(),
    ));
    println!("Created memory manager");

    // Create memory tools
    let memory_tools = create_memory_tools(
        memory_manager,
        &config,
        Some(MemoryToolConfig {
            default_user_id: Some("example_user".to_string()),
            default_agent_id: Some("example_agent".to_string()),
            ..Default::default()
        }),
    );
    println!("Created memory tools");

    // Example 1: Store a memory
    println!("\n=== Store Memory Example ===");
    let store_args = StoreMemoryArgs {
        content: "The user prefers to work in the morning and is interested in Rust programming."
            .to_string(),
        user_id: Some("example_user".to_string()),
        agent_id: Some("example_agent".to_string()),
        memory_type: Some("personal".to_string()),
        topics: Some(vec!["preferences".to_string(), "programming".to_string()]),
    };

    match memory_tools.store_memory().call(store_args).await {
        Ok(MemoryToolOutput {
            success,
            message,
            data,
        }) => {
            println!(
                "Store memory result: success={}, message={}",
                success, message
            );
            if let Some(data) = data {
                println!("Data: {}", serde_json::to_string_pretty(&data)?);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Example 2: Query memories
    println!("\n=== Query Memory Example ===");
    let query_args = QueryMemoryArgs {
        query: "What are the user's programming preferences?".to_string(),
        k: Some(5),
        memory_type: None,
        min_salience: Some(0.5),
        topics: Some(vec!["programming".to_string(), "preferences".to_string()]),
        user_id: Some("example_user".to_string()),
        agent_id: Some("example_agent".to_string()),
    };

    match memory_tools.query_memory().call(query_args).await {
        Ok(MemoryToolOutput {
            success,
            message,
            data,
        }) => {
            println!(
                "Query memory result: success={}, message={}",
                success, message
            );
            if let Some(data) = data {
                println!("Data: {}", serde_json::to_string_pretty(&data)?);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Example 3: List memories
    println!("\n=== List Memories Example ===");
    let list_args = ListMemoriesArgs {
        limit: Some(10),
        memory_type: Some("personal".to_string()),
        user_id: Some("example_user".to_string()),
        agent_id: Some("example_agent".to_string()),
    };

    match memory_tools.list_memories().call(list_args).await {
        Ok(MemoryToolOutput {
            success,
            message,
            data,
        }) => {
            println!(
                "List memories result: success={}, message={}",
                success, message
            );
            if let Some(data) = data {
                println!("Data: {}", serde_json::to_string_pretty(&data)?);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Example 4: Get a specific memory
    println!("\n=== Get Memory Example ===");
    // First, we need to get a memory ID from the previous query or list
    // In a real application, you would store this ID somewhere

    // For this example, we'll try to get a memory with a placeholder ID
    let get_args = GetMemoryArgs {
        memory_id: "example_memory_id".to_string(),
    };

    match memory_tools.get_memory().call(get_args).await {
        Ok(MemoryToolOutput {
            success,
            message,
            data,
        }) => {
            println!(
                "Get memory result: success={}, message={}",
                success, message
            );
            if let Some(data) = data {
                println!("Data: {}", serde_json::to_string_pretty(&data)?);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    println!("\nAll examples completed successfully!");
    Ok(())
}
