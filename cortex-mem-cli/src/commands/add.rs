use anyhow::Result;
use colored::Colorize;
use cortex_mem_core::*;
use std::sync::Arc;

pub async fn execute(
    fs: Arc<CortexFilesystem>,
    thread: &str,
    role: &str,
    content: &str,
) -> Result<()> {
    let storage = MessageStorage::new(fs);

    // Parse role
    let message_role = match role.to_lowercase().as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => {
            eprintln!("{} Invalid role: {}", "Error:".red().bold(), role);
            eprintln!("Valid roles: user, assistant, system");
            return Ok(());
        }
    };

    // Create message
    let message = Message::new(message_role, content);

    // Save message
    let uri = storage.save_message(thread, &message).await?;

    println!("{} Message added successfully", "✓".green().bold());
    println!("  {}: {}", "Thread".cyan(), thread);
    println!("  {}: {:?}", "Role".cyan(), message.role);
    println!("  {}: {}", "URI".cyan(), uri);
    println!("  {}: {}", "ID".cyan(), message.id);

    Ok(())
}
