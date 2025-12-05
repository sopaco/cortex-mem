use clap::{Parser, Subcommand};
use cortex_mem_core::{
    config::Config,
    initialize_memory_system,
    memory::MemoryManager,
};
use std::path::PathBuf;
use tokio;
use tracing::info;
use tracing_subscriber;

mod commands;

use commands::{add::AddCommand, delete::DeleteCommand, list::ListCommand, search::SearchCommand};

#[derive(Parser)]
#[command(name = "cortex-mem-cli")]
#[command(about = "Rust Agent Memory System CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new memory
    Add {
        /// Content to store as memory
        #[arg(short, long)]
        content: String,
        /// User ID for the memory
        #[arg(short, long)]
        user_id: Option<String>,
        /// Agent ID for the memory
        #[arg(short, long)]
        agent_id: Option<String>,
        /// Memory type (conversational, procedural, factual)
        #[arg(short = 't', long, default_value = "conversational")]
        memory_type: String,
    },
    /// Search for memories
    Search {
        /// Search query (optional - if not provided, will use only metadata filters)
        #[arg(short, long)]
        query: Option<String>,
        /// User ID filter
        #[arg(short, long)]
        user_id: Option<String>,
        /// Agent ID filter
        #[arg(short, long)]
        agent_id: Option<String>,
        /// Topics filter (comma-separated)
        #[arg(long, value_delimiter = ',')]
        topics: Option<Vec<String>>,
        /// Keywords filter (comma-separated)
        #[arg(long, value_delimiter = ',')]
        keywords: Option<Vec<String>>,
        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// List memories
    List {
        /// User ID filter
        #[arg(short, long)]
        user_id: Option<String>,
        /// Agent ID filter
        #[arg(short, long)]
        agent_id: Option<String>,
        /// Memory type filter
        #[arg(short = 't', long)]
        memory_type: Option<String>,
        /// Topics filter (comma-separated)
        #[arg(long, value_delimiter = ',')]
        topics: Option<Vec<String>>,
        /// Keywords filter (comma-separated)
        #[arg(long, value_delimiter = ',')]
        keywords: Option<Vec<String>>,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Delete a memory by ID
    Delete {
        /// Memory ID to delete
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Load configuration from file
    let config = Config::load(&cli.config)?;

    // Create memory manager
    let memory_manager = create_memory_manager(&config).await?;

    // Execute command
    match cli.command {
        Commands::Add {
            content,
            user_id,
            agent_id,
            memory_type,
        } => {
            let cmd = AddCommand::new(memory_manager);
            cmd.execute(content, user_id, agent_id, memory_type).await?;
        }
        Commands::Search {
            query,
            user_id,
            agent_id,
            topics,
            keywords,
            limit,
        } => {
            let cmd = SearchCommand::new(memory_manager);
            cmd.execute(query, user_id, agent_id, topics, keywords, limit).await?;
        }
        Commands::List {
            user_id,
            agent_id,
            memory_type,
            topics,
            keywords,
            limit,
        } => {
            let cmd = ListCommand::new(memory_manager);
            cmd.execute(user_id, agent_id, memory_type, topics, keywords, limit).await?;
        }
        Commands::Delete { id } => {
            let cmd = DeleteCommand::new(memory_manager);
            cmd.execute(id).await?;
        }
    }

    Ok(())
}

async fn create_memory_manager(
    config: &Config,
) -> Result<MemoryManager, Box<dyn std::error::Error>> {
    // Use the new initialization system with auto-detection
    let (vector_store, llm_client) = initialize_memory_system(config).await?;

    // Create memory manager
    let memory_manager = MemoryManager::new(vector_store, llm_client, config.memory.clone());

    info!("Memory manager initialized successfully with auto-detected embedding dimensions");
    Ok(memory_manager)
}
