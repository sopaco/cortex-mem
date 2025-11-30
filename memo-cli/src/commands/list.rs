use memo_core::{
    memory::MemoryManager,
    types::{Filters, MemoryType},
};
use serde_json::Value;
use tracing::{error, info};

pub struct ListCommand {
    memory_manager: MemoryManager,
}

impl ListCommand {
    pub fn new(memory_manager: MemoryManager) -> Self {
        Self { memory_manager }
    }

    pub async fn execute(
        &self,
        user_id: Option<String>,
        agent_id: Option<String>,
        memory_type: Option<String>,
        topics: Option<Vec<String>>,
        keywords: Option<Vec<String>>,
        limit: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut filters = Filters::new();
        
        if let Some(user_id) = user_id {
            filters.user_id = Some(user_id);
        }
        
        if let Some(agent_id) = agent_id {
            filters.agent_id = Some(agent_id);
        }
        
        if let Some(memory_type_str) = memory_type {
            filters.memory_type = Some(parse_memory_type(&memory_type_str));
        }
        
        if let Some(topics) = topics {
            filters.topics = Some(topics);
        }
        
        if let Some(keywords) = keywords {
            filters.custom.insert("keywords".to_string(), Value::Array(
                keywords.into_iter().map(Value::String).collect()
            ));
        }

        match self.memory_manager.list(&filters, Some(limit)).await {
            Ok(memories) => {
                if memories.is_empty() {
                    println!("📝 No memories found with the specified filters");
                } else {
                    println!("📝 Found {} memories:", memories.len());
                    println!();
                    
                    for (i, memory) in memories.iter().enumerate() {
                        println!("{}. ID: {}", i + 1, memory.id);
                        println!("   Content: {}", memory.content);
                        println!("   Type: {:?}", memory.metadata.memory_type);
                        println!("   Created: {}", memory.created_at.format("%Y-%m-%d %H:%M:%S"));
                        println!("   Updated: {}", memory.updated_at.format("%Y-%m-%d %H:%M:%S"));
                        
                        if let Some(user_id) = &memory.metadata.user_id {
                            println!("   User: {}", user_id);
                        }
                        
                        if let Some(agent_id) = &memory.metadata.agent_id {
                            println!("   Agent: {}", agent_id);
                        }
                        
                        if let Some(role) = &memory.metadata.role {
                            println!("   Role: {}", role);
                        }
                        
                        // Display topics
                        if !memory.metadata.topics.is_empty() {
                            println!("   Topics: {}", memory.metadata.topics.join(", "));
                        }
                        
                        // Display keywords from custom metadata
                        if let Some(keywords) = memory.metadata.custom.get("keywords") {
                            if let Some(keywords_array) = keywords.as_array() {
                                let keyword_strings: Vec<String> = keywords_array
                                    .iter()
                                    .filter_map(|k| k.as_str())
                                    .map(|s| s.to_string())
                                    .collect();
                                if !keyword_strings.is_empty() {
                                    println!("   Keywords: {}", keyword_strings.join(", "));
                                }
                            }
                        }
                        
                        println!();
                    }
                }
                
                info!("List completed: {} memories found", memories.len());
            }
            Err(e) => {
                error!("Failed to list memories: {}", e);
                println!("❌ List failed: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }
}

fn parse_memory_type(type_str: &str) -> MemoryType {
    match type_str.to_lowercase().as_str() {
        "conversational" => MemoryType::Conversational,
        "procedural" => MemoryType::Procedural,
        "factual" => MemoryType::Factual,
        _ => MemoryType::Conversational,
    }
}