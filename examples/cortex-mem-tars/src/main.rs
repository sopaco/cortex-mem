mod agent;
mod api_models;
mod api_server;
mod app;
mod config;
mod infrastructure;
mod logger;
mod ui;

use anyhow::{Context, Result};
use app::{App, create_default_bots};
use clap::Parser;
use config::ConfigManager;
use infrastructure::Infrastructure;
use logger::init_logger;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "cortex-mem-tars")]
#[command(about = "TARS, An Interactive Demonstration Program Based on Cortex Memory V2")]
#[command(author = "Cortex Memory Team")]
#[command(version)]
struct Args {
    /// Enable enhanced memory saver - save conversations to memory on exit
    #[arg(long, action)]
    enhance_memory_saver: bool,

    /// Enable audio connect - start API server for voice recognition
    #[arg(long, action)]
    enable_audio_connect: bool,

    /// Audio connect mode: store (save to memory) or chat (simulate user input)
    #[arg(long, default_value = "store")]
    audio_connect_mode: String,

    /// Data directory for cortex filesystem
    #[arg(short, long, default_value = "./cortex-data")]
    data_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    if args.enhance_memory_saver {
        log::info!("Enhanced memory saver enabled");
    }

    if args.enable_audio_connect {
        log::info!("Audio connect enabled");
        log::info!("Audio connect mode: {}", args.audio_connect_mode);
    }

    // Initialize configuration manager
    let config_manager = ConfigManager::new().context("Failed to initialize config manager")?;
    log::info!("Config manager initialized");

    // Initialize logger
    let log_manager = init_logger(&config_manager.config_dir()).context("Failed to initialize logger")?;
    log::info!("Logger initialized");

    // Create default bots
    create_default_bots(&config_manager).context("Failed to create default bots")?;

    // Initialize infrastructure (MemoryOperations)
    let infrastructure = match Infrastructure::new(&args.data_dir).await {
        Ok(inf) => {
            log::info!("Infrastructure initialized successfully");
            Some(Arc::new(inf))
        }
        Err(e) => {
            log::warn!("Infrastructure initialization failed: {}", e);
            None
        }
    };

    // Create and run application
    let mut app = App::new(
        config_manager,
        log_manager,
        infrastructure.clone(),
        args.enable_audio_connect,
        args.audio_connect_mode.clone(),
        args.data_dir.clone(),
    )
    .context("Failed to create application")?;

    log::info!("Application created successfully");

    // Check service status
    app.check_service_status().await.context("Failed to check service status")?;

    // Run application
    app.run().await.context("Application run failed")?;

    // On exit, save conversations if enhanced memory saver is enabled
    if args.enhance_memory_saver {
        if let Some(inf) = infrastructure {
            println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
            println!("║                            🧠 Cortex Memory - Exit Process                     ║");
            println!("╚══════════════════════════════════════════════════════════════════════════════╝");

            log::info!("Starting exit process, saving conversations to memory...");

            let conversations = app.get_conversations();
            let thread_id = app.get_thread_id();

            println!("📋 Session Summary:");
            println!("   • Conversations: {} turns", conversations.len());
            println!("   • Thread ID: {}", thread_id);

            if conversations.is_empty() {
                println!("⚠️  No conversations to save");
                println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
                println!("║                                    ✅ Exit Complete                          ║");
                println!("╚══════════════════════════════════════════════════════════════════════════════╝");
                println!("👋 Cortex TARS powering down. Goodbye!");
                return Ok(());
            }

            println!("\n🧠 Saving conversations to memory...");
            println!("📝 Storing {} conversation pairs...", conversations.len());

            match app.save_conversations_to_memory().await {
                Ok(_) => {
                    println!("✨ Memory saved successfully!");
                    println!("✅ All conversations stored in memory system");
                    println!("🔍 Storage details:");
                    println!("   • Conversation pairs: {}", conversations.len());
                }
                Err(e) => {
                    println!("❌ Failed to save memory: {}", e);
                    println!("⚠️  Continuing with normal exit");
                }
            }

            println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
            println!("║                                  🎉 Exit Complete                            ║");
            println!("╚══════════════════════════════════════════════════════════════════════════════╝");
            println!("👋 Cortex TARS powering down. Goodbye!");
        } else {
            println!("\n⚠️  Infrastructure not initialized, cannot save conversations");
            println!("👋 Cortex TARS powering down. Goodbye!");
        }
    } else {
        log::info!("Enhanced memory saver not enabled, skipping conversation save");
        println!("\n👋 Cortex TARS powering down. Goodbye!");
    }

    Ok(())
}
