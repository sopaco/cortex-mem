use memo_core::memory::MemoryManager;
use tracing::{error, info};

pub struct DeleteCommand {
    memory_manager: MemoryManager,
}

impl DeleteCommand {
    pub fn new(memory_manager: MemoryManager) -> Self {
        Self { memory_manager }
    }

    pub async fn execute(&self, id: String) -> Result<(), Box<dyn std::error::Error>> {
        // First, try to get the memory to confirm it exists
        match self.memory_manager.get(&id).await {
            Ok(Some(memory)) => {
                println!("Found memory to delete:");
                println!("ID: {}", memory.id);
                println!("Content: {}", memory.content);
                println!("Type: {:?}", memory.metadata.memory_type);
                println!();

                // Confirm deletion
                print!("Are you sure you want to delete this memory? (y/N): ");
                use std::io::{self, Write};
                io::stdout().flush().unwrap();
                
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                    match self.memory_manager.delete(&id).await {
                        Ok(()) => {
                            println!("✅ Memory deleted successfully!");
                            info!("Memory deleted: {}", id);
                        }
                        Err(e) => {
                            error!("Failed to delete memory: {}", e);
                            println!("❌ Failed to delete memory: {}", e);
                            return Err(e.into());
                        }
                    }
                } else {
                    println!("❌ Deletion cancelled");
                }
            }
            Ok(None) => {
                println!("❌ Memory with ID '{}' not found", id);
            }
            Err(e) => {
                error!("Failed to retrieve memory: {}", e);
                println!("❌ Failed to retrieve memory: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }
}