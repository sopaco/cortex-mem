use cortex_mem_core::{memory::MemoryManager, types::Filters};
use serde_json::Value;
use tracing::{error, info};

pub struct SearchCommand {
    memory_manager: MemoryManager,
}

impl SearchCommand {
    pub fn new(memory_manager: MemoryManager) -> Self {
        Self { memory_manager }
    }

    pub async fn execute(
        &self,
        query: Option<String>,
        user_id: Option<String>,
        agent_id: Option<String>,
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
        
        if let Some(topics) = topics {
            filters.topics = Some(topics);
        }
        
        if let Some(keywords) = keywords {
            filters.custom.insert("keywords".to_string(), Value::Array(
                keywords.into_iter().map(Value::String).collect()
            ));
        }

        // 如果没有查询字符串但有元数据过滤器，使用 list 方法
        let results = if let Some(query_str) = &query {
            self.memory_manager.search(query_str, &filters, limit).await?
        } else {
            // 将 list 结果转换为 ScoredMemory 格式
            let memories = self.memory_manager.list(&filters, Some(limit)).await?;
            memories.into_iter()
                .map(|memory| cortex_mem_core::types::ScoredMemory {
                    memory,
                    score: 0.0, // list 操作没有相似度分数
                })
                .collect()
        };

        if results.is_empty() {
            if let Some(query_str) = &query {
                println!("🔍 No memories found for query: '{}'", query_str);
            } else {
                println!("🔍 No memories found with the specified filters");
            }
        } else {
            if let Some(query_str) = &query {
                println!("🔍 Found {} memories for query: '{}'", results.len(), query_str);
            } else {
                println!("🔍 Found {} memories with the specified filters", results.len());
            }
            println!();

                    for (i, scored_memory) in results.iter().enumerate() {
                        println!(
                            "{}. [Score: {:.3}] ID: {}",
                            i + 1,
                            scored_memory.score,
                            scored_memory.memory.id
                        );
                        println!("   Content: {}", scored_memory.memory.content);
                        println!("   Type: {:?}", scored_memory.memory.metadata.memory_type);
                        println!(
                            "   Created: {}",
                            scored_memory.memory.created_at.format("%Y-%m-%d %H:%M:%S")
                        );

                        if let Some(user_id) = &scored_memory.memory.metadata.user_id {
                            println!("   User: {}", user_id);
                        }

                        if let Some(agent_id) = &scored_memory.memory.metadata.agent_id {
                            println!("   Agent: {}", agent_id);
                        }
                        
                        // Display topics
                        if !scored_memory.memory.metadata.topics.is_empty() {
                            println!("   Topics: {}", scored_memory.memory.metadata.topics.join(", "));
                        }
                        
                        // Display keywords from custom metadata
                        if let Some(keywords) = scored_memory.memory.metadata.custom.get("keywords") {
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

        info!("Search completed: {} results found", results.len());

        Ok(())
    }
}
