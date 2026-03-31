use axum::{Router, routing::get};
use clap::Parser;
use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod handlers;
mod models;
mod routes;
mod state;

use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "cortex-mem-service")]
#[command(about = "Cortex Memory HTTP REST API Service", long_about = None)]
#[command(version)]
struct Cli {
    /// Data directory for cortex filesystem
    #[arg(short, long, default_value = "./cortex-data")]
    data_dir: String,

    /// Path to configuration file (config.toml).
    /// If not specified, will look for config.toml in --data-dir first, then current directory.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Server host
    #[arg(long, default_value = "localhost")]
    host: String,

    /// Server port
    #[arg(short, long, default_value_t = 8085)]
    port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Log file path. When specified, logs will be written to both file and stdout
    #[arg(long, value_name = "PATH")]
    log_file: Option<PathBuf>,
}

impl Cli {
    /// Resolve the config file path with the following priority:
    /// 1. If --config is explicitly specified, use it
    /// 2. If --data-dir is specified (not default), look for config.toml in data_dir
    /// 3. Otherwise, use config.toml in current directory
    fn resolve_config_path(&self) -> PathBuf {
        // Case 1: Explicit --config specified
        if let Some(ref config) = self.config {
            return config.clone();
        }

        // Case 2: --data-dir is not the default, look for config.toml in data_dir
        // Default data_dir is "./cortex-data", but user might specify a different path
        let data_dir = PathBuf::from(&self.data_dir);
        let config_in_data_dir = data_dir.join("config.toml");
        if config_in_data_dir.exists() {
            return config_in_data_dir;
        }

        // Case 3: Fall back to config.toml in current directory
        PathBuf::from("config.toml")
    }
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

    // Setup logging layers
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_level(true);

    if let Some(ref log_path) = cli.log_file {
        // Ensure parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create log file
        let log_file = File::create(log_path)?;
        let file_writer = Mutex::new(log_file);

        // File layer (no colors, includes target for debugging)
        let file_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_level(true)
            .with_ansi(false)
            .with_writer(file_writer);

        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .with(log_level)
            .init();

        info!("Logging to file: {}", log_path.display());
    } else {
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(log_level)
            .init();
    }

    info!("Starting Cortex Memory Service");
    info!("Data directory: {}", cli.data_dir);
    
    // Resolve config path with priority: explicit --config > data_dir/config.toml > ./config.toml
    let config_path = cli.resolve_config_path();
    info!("Config file: {}", config_path.display());

    // Initialize application state with config path
    let state = AppState::new(&cli.data_dir, &config_path).await?;
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
