use anyhow::Result;
use clap::{Parser, Subcommand};
use cortex_mem_config::Config;
use cortex_mem_core::llm::LLMClientImpl;
use cortex_mem_tools::MemoryOperations;
use std::path::PathBuf;
use std::sync::Arc;

mod commands;
use commands::{add, delete, get, layers, list, search, session, stats, tenant};

/// Cortex-Mem CLI - File-based memory management for AI Agents
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Tenant identifier (use 'cortex-mem tenant list' to see available tenants)
    #[arg(long, default_value = "default")]
    tenant: String,

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

    /// Search for memories using semantic vector search
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
        #[arg(short = 's', long, default_value = "0.4")]
        min_score: f32,

        /// Search scope: "session", "user", or "agent"
        #[arg(long, default_value = "session")]
        scope: String,
    },

    /// List memories
    List {
        /// URI path to list (e.g., "cortex://session" or "cortex://user/preferences")
        #[arg(short, long)]
        uri: Option<String>,

        /// Include abstracts in results
        #[arg(long)]
        include_abstracts: bool,
    },

    /// Get a specific memory
    Get {
        /// Memory URI
        uri: String,

        /// Show abstract (L0) instead of full content
        #[arg(short, long)]
        abstract_only: bool,
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

    /// Show statistics
    Stats,

    /// Layer management (L0/L1 files)
    Layers {
        #[command(subcommand)]
        action: LayersAction,
    },

    /// Tenant management
    Tenant {
        #[command(subcommand)]
        action: TenantAction,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List,

    /// Create a new session
    Create {
        /// Thread ID
        thread: String,

        /// Session title
        #[arg(short, long)]
        title: Option<String>,
    },
}

#[derive(Subcommand)]
enum LayersAction {
    /// Ensure all directories have L0/L1 files (.abstract.md and .overview.md)
    EnsureAll,

    /// Show status of L0/L1 file coverage
    Status,

    /// Regenerate oversized .abstract files (> 2K characters)
    RegenerateOversized,
}

#[derive(Subcommand)]
enum TenantAction {
    /// List all available tenants
    List,
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

    // Load configuration (required for vector search)
    let config = Config::load(&cli.config).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load config from {}: {}. \
             Please ensure config.toml exists with [llm], [qdrant], and [embedding] sections.",
            cli.config.display(),
            e
        )
    })?;

    // Determine data directory
    let data_dir = config.cortex.data_dir();

    // Handle tenant list command early (doesn't need MemoryOperations)
    if let Commands::Tenant { action } = cli.command {
        match action {
            TenantAction::List => {
                tenant::list(&data_dir).await?;
            }
        }
        return Ok(());
    }

    // Initialize LLM client
    let model_name = config.llm.model_efficient.clone();
    let llm_config = cortex_mem_core::llm::LLMConfig {
        api_base_url: config.llm.api_base_url,
        api_key: config.llm.api_key,
        model_efficient: config.llm.model_efficient,
        temperature: config.llm.temperature,
        max_tokens: config.llm.max_tokens as usize,
    };
    let llm_client = Arc::new(LLMClientImpl::new(llm_config)?);

    // Initialize MemoryOperations with vector search
    let operations = MemoryOperations::new(
        &data_dir,
        &cli.tenant,
        llm_client,
        &config.qdrant.url,
        &config.qdrant.collection_name,
        config.qdrant.api_key.as_deref(),
        &config.embedding.api_base_url,
        &config.embedding.api_key,
        &config.embedding.model_name,
        config.qdrant.embedding_dim,
        None,  // user_id parameter
    )
    .await?;

    if cli.verbose {
        eprintln!("LLM model: {}", model_name);
        eprintln!("Data directory: {}", data_dir);
        eprintln!("Tenant: {}", cli.tenant);
    }

    let operations = Arc::new(operations);

    // Execute command
    match cli.command {
        Commands::Add {
            thread,
            role,
            content,
        } => {
            add::execute(operations, &thread, &role, &content).await?;
        }
        Commands::Search {
            query,
            thread,
            limit,
            min_score,
            scope,
        } => {
            search::execute(
                operations,
                &query,
                thread.as_deref(),
                limit,
                min_score,
                &scope,
            )
            .await?;
        }
        Commands::List {
            uri,
            include_abstracts,
        } => {
            list::execute(operations, uri.as_deref(), include_abstracts).await?;
        }
        Commands::Get { uri, abstract_only } => {
            get::execute(operations, &uri, abstract_only).await?;
        }
        Commands::Delete { uri } => {
            delete::execute(operations, &uri).await?;
        }
        Commands::Session { action } => match action {
            SessionAction::List => {
                session::list(operations).await?;
            }
            SessionAction::Create { thread, title } => {
                session::create(operations, &thread, title.as_deref()).await?;
            }
        },
        Commands::Stats => {
            stats::execute(operations).await?;
        }
        Commands::Layers { action } => match action {
            LayersAction::EnsureAll => {
                layers::ensure_all(operations).await?;
            }
            LayersAction::Status => {
                layers::status(operations).await?;
            }
            LayersAction::RegenerateOversized => {
                layers::regenerate_oversized(operations).await?;
            }
        },
        Commands::Tenant { .. } => {
            // Already handled above
        }
    }

    Ok(())
}
