use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;
use std::path::Path;

pub async fn create(fs: Arc<CortexFilesystem>, thread: &str, title: Option<&str>) -> Result<()> {
    let config = SessionConfig::default();
    let manager = SessionManager::new(fs, config);

    println!("{} Creating session: {}", "📝".bold(), thread.cyan());

    let mut metadata = manager.create_session(thread).await?;

    if let Some(title_text) = title {
        metadata.set_title(title_text);
        manager.update_session(&metadata).await?;
        println!("  {}: {}", "Title".cyan(), title_text);
    }

    println!("{} Session created successfully", "✓".green().bold());
    println!("  {}: {}", "Thread ID".cyan(), metadata.thread_id);
    println!("  {}: {:?}", "Status".cyan(), metadata.status);
    println!("  {}: {}", "Created".cyan(), metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC"));

    Ok(())
}

pub async fn close(fs: Arc<CortexFilesystem>, thread: &str) -> Result<()> {
    let config = SessionConfig::default();
    let mut manager = SessionManager::new(fs, config);

    println!("{} Closing session: {}", "🔒".bold(), thread.cyan());

    let metadata = manager.close_session(thread).await?;

    println!("{} Session closed successfully", "✓".green().bold());
    println!("  {}: {}", "Thread ID".cyan(), metadata.thread_id);
    println!("  {}: {:?}", "Status".cyan(), metadata.status);
    if let Some(closed_at) = metadata.closed_at {
        println!("  {}: {}", "Closed".cyan(), closed_at.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    println!("  {}: {}", "Messages".cyan(), metadata.message_count);

    Ok(())
}

pub async fn extract(fs: Arc<CortexFilesystem>, thread: &str) -> Result<()> {
    println!("{} Extracting memories from session: {}", "🧠".bold(), thread.cyan());

    // Try to load LLM config from config.toml
    let llm_config = if Path::new("config.toml").exists() {
        println!("  {} Loading LLM config from config.toml", "⚙".dimmed());
        match load_llm_config_from_toml("config.toml") {
            Ok(config) => {
                println!("  {} Using model: {}", "✓".green(), config.model_efficient.bright_blue());
                config
            }
            Err(e) => {
                println!("  {} Failed to load config.toml: {}", "⚠".yellow(), e);
                println!("  {} Using default config (environment variables)", "ℹ".dimmed());
                cortex_mem_core::llm::client::LLMConfig::default()
            }
        }
    } else {
        println!("  {} No config.toml found, using environment variables", "ℹ".dimmed());
        cortex_mem_core::llm::client::LLMConfig::default()
    };

    // Initialize extractor
    let llm = Arc::new(LLMClient::new(llm_config)?);
    let extractor_config = ExtractionConfig::default();
    let extractor = MemoryExtractor::new(fs.clone(), llm, extractor_config);

    // Extract memories
    println!("  {} Analyzing conversation with LLM...", "🤖".dimmed());
    let extracted = extractor.extract_from_thread(thread).await?;

    println!("{} Extraction complete", "✓".green().bold());
    println!("  {}: {}", "Facts".cyan(), extracted.facts.len());
    println!("  {}: {}", "Decisions".cyan(), extracted.decisions.len());
    println!("  {}: {}", "Entities".cyan(), extracted.entities.len());
    println!("  {}: {}", "Total".cyan().bold(), extracted.total_count());

    // Save extraction
    let extraction_uri = extractor.save_extraction(thread, &extracted).await?;
    println!("  {}: {}", "Saved to".cyan(), extraction_uri.bright_blue());

    // Display preview
    if !extracted.facts.is_empty() {
        println!("\n{} Sample Facts:", "📌".bold());
        for (i, fact) in extracted.facts.iter().take(3).enumerate() {
            println!("  {}. {} (confidence: {:.2})", 
                i + 1, 
                fact.content.dimmed(),
                fact.confidence
            );
        }
        if extracted.facts.len() > 3 {
            println!("  ... and {} more", extracted.facts.len() - 3);
        }
    }

    if !extracted.decisions.is_empty() {
        println!("\n{} Sample Decisions:", "🎯".bold());
        for (i, decision) in extracted.decisions.iter().take(3).enumerate() {
            println!("  {}. {} (confidence: {:.2})", 
                i + 1, 
                decision.decision.dimmed(),
                decision.confidence
            );
        }
        if extracted.decisions.len() > 3 {
            println!("  ... and {} more", extracted.decisions.len() - 3);
        }
    }

    if !extracted.entities.is_empty() {
        println!("\n{} Sample Entities:", "👥".bold());
        for (i, entity) in extracted.entities.iter().take(3).enumerate() {
            println!("  {}. {} ({})", 
                i + 1, 
                entity.name.bright_yellow(),
                entity.entity_type.dimmed()
            );
        }
        if extracted.entities.len() > 3 {
            println!("  ... and {} more", extracted.entities.len() - 3);
        }
    }

    Ok(())
}

pub async fn list(fs: Arc<CortexFilesystem>) -> Result<()> {
    println!("{} Listing all sessions", "📋".bold());

    let threads_uri = "cortex://threads";
    let entries = fs.list(threads_uri).await?;

    if entries.is_empty() {
        println!("\n{} No sessions found", "ℹ".yellow().bold());
        return Ok(());
    }

    println!("\n{} Found {} sessions:", "✓".green().bold(), entries.len());
    println!();

    for entry in entries {
        if !entry.is_directory || entry.name.starts_with('.') {
            continue;
        }

        println!("• {}", entry.name.bright_blue().bold());

        // Try to load session metadata
        let metadata_uri = format!("{}/{}", entry.uri, ".session.json");
        if fs.exists(&metadata_uri).await.unwrap_or(false) {
            if let Ok(metadata_json) = fs.read(&metadata_uri).await {
                if let Ok(metadata) = serde_json::from_str::<SessionMetadata>(&metadata_json) {
                    println!("  {}: {:?}", "Status".dimmed(), metadata.status);
                    println!("  {}: {}", "Messages".dimmed(), metadata.message_count);
                    if let Some(ref title) = metadata.title {
                        println!("  {}: {}", "Title".dimmed(), title);
                    }
                }
            }
        }
        println!();
    }

    Ok(())
}

fn load_llm_config_from_toml(path: &str) -> Result<cortex_mem_core::llm::client::LLMConfig> {
    use std::fs;
    
    let content = fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    
    let llm_section = value.get("llm")
        .ok_or_else(|| anyhow::anyhow!("No [llm] section in config.toml"))?;
    
    let api_base_url = llm_section.get("api_base_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing api_base_url"))?
        .to_string();
    
    let api_key = llm_section.get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing api_key"))?
        .to_string();
    
    let model_efficient = llm_section.get("model_efficient")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing model_efficient"))?
        .to_string();
    
    let temperature = llm_section.get("temperature")
        .and_then(|v| v.as_float())
        .unwrap_or(0.1) as f32;
    
    let max_tokens = llm_section.get("max_tokens")
        .and_then(|v| v.as_integer())
        .unwrap_or(4096) as usize;
    
    Ok(cortex_mem_core::llm::client::LLMConfig {
        api_base_url,
        api_key,
        model_efficient,
        temperature,
        max_tokens,
    })
}
