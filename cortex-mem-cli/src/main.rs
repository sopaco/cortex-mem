use anyhow::Result;
use clap::{Parser, Subcommand};
use cortex_mem_core::*;
use std::path::PathBuf;
use std::sync::Arc;

mod commands;
use commands::{add, automation, delete, get, list, search, session, stats};

/// Cortex-Mem CLI - File-based memory management for AI Agents
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Data directory path
    #[arg(short, long, default_value = "./cortex-data")]
    data_dir: PathBuf,

    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a memory to the system
    Add {
        /// Thread ID
        #[arg(short, long)]
        thread: String,

        /// Message role (user/assistant/system)
        #[arg(short, long, default_value = "user")]
        role: String,

        /// Message content
        content: String,
    },

    /// Search for memories
    Search {
        /// Search query
        query: String,

        /// Thread ID to search in
        #[arg(short, long)]
        thread: Option<String>,

        /// Maximum results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Minimum relevance score (0.0-1.0)
        #[arg(short = 's', long, default_value = "0.3")]
        min_score: f32,
    },

    /// List memories
    List {
        /// Thread ID
        #[arg(short, long)]
        thread: Option<String>,

        /// Dimension (agents/users/threads/global)
        #[arg(short, long)]
        dimension: Option<String>,
    },

    /// Get a specific memory
    Get {
        /// Memory URI
        uri: String,
    },

    /// Delete a memory
    Delete {
        /// Memory URI
        uri: String,
    },

    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    /// Automation features
    Automation {
        #[command(subcommand)]
        action: AutomationAction,
    },

    /// Show statistics
    Stats,
}

#[derive(Subcommand)]
enum SessionAction {
    /// Create a new session
    Create {
        /// Thread ID
        thread: String,

        /// Session title
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Close a session
    Close {
        /// Thread ID
        thread: String,
    },

    /// Extract memories from a session
    Extract {
        /// Thread ID
        thread: String,
    },

    /// List all sessions
    List,
}

#[derive(Subcommand)]
enum AutomationAction {
    /// Build vector index for a session
    Index {
        /// Thread ID
        thread: String,
    },

    /// Auto-extract memories from a session
    AutoExtract {
        /// Thread ID
        thread: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    // Initialize filesystem
    let fs = Arc::new(CortexFilesystem::new(&cli.data_dir));
    fs.initialize().await?;

    // Execute command
    match cli.command {
        Commands::Add {
            thread,
            role,
            content,
        } => {
            add::execute(fs, &thread, &role, &content).await?;
        }
        Commands::Search {
            query,
            thread,
            limit,
            min_score,
        } => {
            search::execute(fs, &query, thread.as_deref(), limit, min_score).await?;
        }
        Commands::List { thread, dimension } => {
            list::execute(fs, thread.as_deref(), dimension.as_deref()).await?;
        }
        Commands::Get { uri } => {
            get::execute(fs, &uri).await?;
        }
        Commands::Delete { uri } => {
            delete::execute(fs, &uri).await?;
        }
        Commands::Session { action } => match action {
            SessionAction::Create { thread, title } => {
                session::create(fs, &thread, title.as_deref()).await?;
            }
            SessionAction::Close { thread } => {
                session::close(fs, &thread).await?;
            }
            SessionAction::Extract { thread } => {
                session::extract(fs, &thread).await?;
            }
            SessionAction::List => {
                session::list(fs).await?;
            }
        },
        Commands::Automation { action } => match action {
            AutomationAction::Index { thread } => {
                automation::index_session(fs, &thread).await?;
            }
            AutomationAction::AutoExtract { thread } => {
                // Load LLM config
                let llm_config = if std::path::Path::new("config.toml").exists() {
                    match load_llm_config_from_toml("config.toml") {
                        Ok(config) => config,
                        Err(e) => {
                            eprintln!("Failed to load config.toml: {}", e);
                            LLMClient::default_config()
                        }
                    }
                } else {
                    LLMClient::default_config()
                };
                
                let llm = Arc::new(LLMClient::new(llm_config)?);
                automation::auto_extract_on_close(fs, &thread, llm).await?;
            }
        },
        Commands::Stats => {
            stats::execute(fs).await?;
        }
    }

    Ok(())
}

fn load_llm_config_from_toml(path: &str) -> Result<cortex_mem_core::llm::client::LLMConfig> {
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    
    let llm_section = value.get("llm")
        .ok_or_else(|| anyhow::anyhow!("No [llm] section in config.toml"))?;
    
    let config = cortex_mem_core::llm::client::LLMConfig {
        api_base_url: llm_section.get("api_base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing llm.api_base_url"))?
            .to_string(),
        api_key: llm_section.get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing llm.api_key"))?
            .to_string(),
        model_efficient: llm_section.get("model_efficient")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing llm.model_efficient"))?
            .to_string(),
        temperature: llm_section.get("temperature")
            .and_then(|v| v.as_float())
            .unwrap_or(0.1) as f32,
        max_tokens: llm_section.get("max_tokens")
            .and_then(|v| v.as_integer())
            .unwrap_or(4096) as usize,
    };
    
    Ok(config)
}
