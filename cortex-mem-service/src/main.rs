use axum::{
    Router,
    routing::{get, post},
};
use clap::Parser;
use cortex_mem_core::{
    config::Config, llm::create_llm_client, memory::MemoryManager,
    vector_store::qdrant::QdrantVectorStore,
};
use std::{path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber;

mod handlers;
mod models;

use handlers::{
    batch_delete_memories, batch_update_memories, create_memory, delete_memory, get_memory,
    health_check, list_memories, search_memories, update_memory,
};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub memory_manager: Arc<MemoryManager>,
}

#[derive(Parser)]
#[command(name = "cortex-mem-service")]
#[command(about = "Rust Agent Memory System HTTP Service")]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Load configuration
    let config = Config::load(&cli.config)?;

    // Create memory manager
    let memory_manager = create_memory_manager(&config).await?;

    // Create application state
    let app_state = AppState {
        memory_manager: Arc::new(memory_manager),
    };

    // Build the application router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/memories", post(create_memory).get(list_memories))
        .route("/memories/search", post(search_memories))
        .route(
            "/memories/{id}",
            get(get_memory).put(update_memory).delete(delete_memory),
        )
        .route("/memories/batch/delete", post(batch_delete_memories))
        .route("/memories/batch/update", post(batch_update_memories))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .into_inner(),
        )
        .with_state(app_state);

    // Start the server
    let addr = format!("{}:{}", config.server.host, config.server.port);

    info!("Starting cortex-mem-service on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_memory_manager(
    config: &Config,
) -> Result<MemoryManager, Box<dyn std::error::Error>> {
    // Create vector store
    let vector_store = QdrantVectorStore::new(&config.qdrant).await?;

    // Create LLM client
    let llm_client = create_llm_client(&config.llm, &config.embedding)?;

    // Create memory manager
    let memory_manager =
        MemoryManager::new(Box::new(vector_store), llm_client, config.memory.clone());

    info!("Memory manager initialized successfully");
    Ok(memory_manager)
}
