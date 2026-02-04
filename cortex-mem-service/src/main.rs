use axum::{
    Router,
    routing::get,
};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod state;
mod routes;
mod handlers;
mod models;
mod error;

use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "cortex-mem-service")]
#[command(about = "Cortex-Mem V2 HTTP REST API Service", long_about = None)]
#[command(version)]
struct Cli {
    /// Data directory for cortex filesystem
    #[arg(short, long, default_value = "./cortex-data")]
    data_dir: String,

    /// Server host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let log_level = if cli.verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true)
        )
        .with(log_level)
        .init();

    info!("Starting Cortex-Mem Service V2");
    info!("Data directory: {}", cli.data_dir);

    // Initialize application state
    let state = AppState::new(&cli.data_dir).await?;
    let state = Arc::new(state);

    // Build router
    let app = Router::new()
        .route("/health", get(handlers::health::health_check))
        .nest("/api/v2", routes::api_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
