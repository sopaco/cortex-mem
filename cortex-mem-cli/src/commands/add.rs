use cortex_mem_core::{
    memory::MemoryManager,
    types::{MemoryMetadata, MemoryType, Message},
};
use tracing::{error, info};

pub struct AddCommand {
    memory_manager: MemoryManager,
}

impl AddCommand {
    pub fn new(memory_manager: MemoryManager) -> Self {
        Self { memory_manager }
    }

    pub async fn execute(
        &self,
        content: String,
        user_id: Option<String>,
        agent_id: Option<String>,
        memory_type: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let memory_type = parse_memory_type(&memory_type);

        let mut metadata = MemoryMetadata::new(memory_type.to_owned());

        if let Some(ref user_id) = user_id {
            metadata = metadata.with_user_id(user_id.to_owned());
        }

        if let Some(ref agent_id) = agent_id {
            metadata = metadata.with_agent_id(agent_id.to_owned());
        }

        // Check if this should be handled as a conversation (for procedural memory or advanced fact extraction)
        let is_conversation = memory_type == MemoryType::Procedural
            || content.contains('\n')
            || content.contains("Assistant:")
            || content.contains("User:");

        if is_conversation {
            // Handle as conversation for advanced processing
            let messages = if content.contains('\n') || content.contains("User:") || content.contains("Assistant:") {
                // Parse conversation format
                parse_conversation_content(&content, &user_id, &agent_id)
            } else {
                // Single user message
                vec![Message {
                    role: "user".to_string(),
                    content: content.clone(),
                    name: user_id.clone(),
                }]
            };

            match self.memory_manager.add_memory(&messages, metadata).await {
                Ok(results) => {
                    info!("Memory added successfully with {} actions", results.len());
                    println!("✅ Memory added successfully!");
                    println!("Memory Type: {:?}", memory_type);
                    println!("Actions Performed: {}", results.len());

                    for (i, result) in results.iter().enumerate() {
                        println!(
                            "  {}. {:?} - {}",
                            i + 1,
                            result.event,
                            result.memory.chars().take(100).collect::<String>()
                        );
                        if result.memory.len() > 100 {
                            println!("     (truncated)");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to add memory: {}", e);
                    println!("❌ Failed to add memory: {}", e);
                    return Err(e.into());
                }
            }
        } else {
            // Handle as simple content storage
            match self.memory_manager.store(content.clone(), metadata).await {
                Ok(memory_id) => {
                    info!("Memory stored successfully with ID: {}", memory_id);
                    println!("✅ Memory added successfully!");
                    println!("ID: {}", memory_id);
                    println!("Content: {}", content.chars().take(100).collect::<String>());
                    if content.len() > 100 {
                        println!("(truncated)");
                    }
                }
                Err(e) => {
                    error!("Failed to store memory: {}", e);
                    println!("❌ Failed to add memory: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}

/// Parse conversation content from CLI input
fn parse_conversation_content(
    content: &str,
    user_id: &Option<String>,
    agent_id: &Option<String>,
) -> Vec<Message> {
    let mut messages = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("User:") || trimmed.starts_with("user:") {
            let user_content = trimmed[5..].trim();
            messages.push(Message {
                role: "user".to_string(),
                content: user_content.to_string(),
                name: user_id.clone(),
            });
        } else if trimmed.starts_with("Assistant:")
            || trimmed.starts_with("assistant:")
            || trimmed.starts_with("AI:")
        {
            let assistant_content = trimmed[10..].trim();
            messages.push(Message {
                role: "assistant".to_string(),
                content: assistant_content.to_string(),
                name: agent_id.clone(),
            });
        } else {
            // If no role prefix, treat as user message
            messages.push(Message {
                role: "user".to_string(),
                content: trimmed.to_string(),
                name: user_id.clone(),
            });
        }
    }

    // If no messages were parsed, treat entire content as user message
    if messages.is_empty() {
        messages.push(Message {
            role: "user".to_string(),
            content: content.to_string(),
            name: user_id.clone(),
        });
    }

    messages
}

fn parse_memory_type(type_str: &str) -> MemoryType {
    match type_str.to_lowercase().as_str() {
        "conversational" => MemoryType::Conversational,
        "procedural" => MemoryType::Procedural,
        "factual" => MemoryType::Factual,
        _ => MemoryType::Conversational,
    }
}
