use crate::{
    error::Result,
    llm::LLMClient,
    types::Memory,
    vector_store::VectorStore,
};
use async_trait::async_trait;


/// Trait for detecting and handling duplicate memories
#[async_trait]
pub trait DuplicateDetector: Send + Sync {
    /// Detect if a memory is a duplicate of existing memories
    async fn detect_duplicates(&self, memory: &Memory) -> Result<Vec<Memory>>;
    
    /// Merge similar memories into a single memory
    async fn merge_memories(&self, memories: &[Memory]) -> Result<Memory>;
    
    /// Check if two memories are similar enough to be considered duplicates
    async fn are_similar(&self, memory1: &Memory, memory2: &Memory) -> Result<bool>;
}

/// Advanced duplicate detector using semantic similarity and LLM-based merging
pub struct AdvancedDuplicateDetector {
    vector_store: Box<dyn VectorStore>,
    llm_client: Box<dyn LLMClient>,
    similarity_threshold: f32,
    _merge_threshold: f32,
}

impl AdvancedDuplicateDetector {
    pub fn new(
        vector_store: Box<dyn VectorStore>,
        llm_client: Box<dyn LLMClient>,
        similarity_threshold: f32,
        merge_threshold: f32,
    ) -> Self {
        Self {
            vector_store,
            llm_client,
            similarity_threshold,
            _merge_threshold: merge_threshold,
        }
    }

    /// Calculate semantic similarity between two memories
    fn calculate_semantic_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        // Calculate cosine similarity between embeddings
        let dot_product: f32 = memory1.embedding.iter()
            .zip(memory2.embedding.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm1: f32 = memory1.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = memory2.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        dot_product / (norm1 * norm2)
    }

    /// Calculate content similarity using various metrics
    fn calculate_content_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        let content1 = memory1.content.to_lowercase();
        let content2 = memory2.content.to_lowercase();

        // Jaccard similarity for word overlap
        let words1: std::collections::HashSet<&str> = content1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = content2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f32 / union as f32
    }

    /// Calculate metadata similarity
    fn calculate_metadata_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        let mut similarity_score = 0.0;
        let mut total_factors = 0.0;

        // Memory type similarity
        if memory1.metadata.memory_type == memory2.metadata.memory_type {
            similarity_score += 1.0;
        }
        total_factors += 1.0;

        // User/agent similarity
        if memory1.metadata.user_id == memory2.metadata.user_id {
            similarity_score += 1.0;
        }
        total_factors += 1.0;

        if memory1.metadata.agent_id == memory2.metadata.agent_id {
            similarity_score += 1.0;
        }
        total_factors += 1.0;

        // Entity overlap
        let entities1: std::collections::HashSet<_> = memory1.metadata.entities.iter().collect();
        let entities2: std::collections::HashSet<_> = memory2.metadata.entities.iter().collect();
        
        if !entities1.is_empty() || !entities2.is_empty() {
            let intersection = entities1.intersection(&entities2).count();
            let union = entities1.union(&entities2).count();
            if union > 0 {
                similarity_score += intersection as f32 / union as f32;
            }
            total_factors += 1.0;
        }

        // Topic overlap
        let topics1: std::collections::HashSet<_> = memory1.metadata.topics.iter().collect();
        let topics2: std::collections::HashSet<_> = memory2.metadata.topics.iter().collect();
        
        if !topics1.is_empty() || !topics2.is_empty() {
            let intersection = topics1.intersection(&topics2).count();
            let union = topics1.union(&topics2).count();
            if union > 0 {
                similarity_score += intersection as f32 / union as f32;
            }
            total_factors += 1.0;
        }

        if total_factors > 0.0 {
            similarity_score / total_factors
        } else {
            0.0
        }
    }

    /// Create a merge prompt for LLM
    fn create_merge_prompt(&self, memories: &[Memory]) -> String {
        let mut prompt = String::from(
            "You are tasked with merging similar memories into a single, comprehensive memory. \
            Please combine the following memories while preserving all important information:\n\n"
        );

        for (i, memory) in memories.iter().enumerate() {
            prompt.push_str(&format!(
                "Memory {}: {}\n",
                i + 1,
                memory.content
            ));
        }

        prompt.push_str(
            "\nPlease provide a merged memory that:\n\
            1. Combines all unique information from the memories\n\
            2. Removes redundant information\n\
            3. Maintains the most important details\n\
            4. Uses clear and concise language\n\n\
            Merged memory:"
        );

        prompt
    }
}

#[async_trait]
impl DuplicateDetector for AdvancedDuplicateDetector {
    async fn detect_duplicates(&self, memory: &Memory) -> Result<Vec<Memory>> {
        // Search for similar memories using vector similarity
        let filters = crate::types::Filters {
            user_id: memory.metadata.user_id.clone(),
            agent_id: memory.metadata.agent_id.clone(),
            memory_type: Some(memory.metadata.memory_type.clone()),
            ..Default::default()
        };

        let similar_memories = self.vector_store
            .search(&memory.embedding, &filters, 10)
            .await?;

        let mut duplicates = Vec::new();

        for scored_memory in similar_memories {
            if scored_memory.memory.id != memory.id {
                let is_similar = self.are_similar(memory, &scored_memory.memory).await?;
                if is_similar {
                    duplicates.push(scored_memory.memory);
                }
            }
        }

        Ok(duplicates)
    }

    async fn merge_memories(&self, memories: &[Memory]) -> Result<Memory> {
        if memories.is_empty() {
            return Err(crate::error::MemoryError::validation("No memories to merge"));
        }

        if memories.len() == 1 {
            return Ok(memories[0].clone());
        }

        // Use LLM to merge content
        let prompt = self.create_merge_prompt(memories);
        let merged_content = self.llm_client.complete(&prompt).await?;

        // Create merged memory based on the most recent memory
        let base_memory = &memories[0];
        let mut merged_memory = base_memory.clone();
        merged_memory.content = merged_content.trim().to_string();

        // Merge metadata
        let mut all_entities = std::collections::HashSet::new();
        let mut all_topics = std::collections::HashSet::new();
        let mut max_importance = 0.0f32;

        for memory in memories {
            for entity in &memory.metadata.entities {
                all_entities.insert(entity.clone());
            }
            for topic in &memory.metadata.topics {
                all_topics.insert(topic.clone());
            }
            max_importance = max_importance.max(memory.metadata.importance_score);
        }

        merged_memory.metadata.entities = all_entities.into_iter().collect();
        merged_memory.metadata.topics = all_topics.into_iter().collect();
        merged_memory.metadata.importance_score = max_importance;

        // Update timestamps
        merged_memory.updated_at = chrono::Utc::now();

        // Re-generate embedding for merged content
        let new_embedding = self.llm_client.embed(&merged_memory.content).await?;
        merged_memory.embedding = new_embedding;

        Ok(merged_memory)
    }

    async fn are_similar(&self, memory1: &Memory, memory2: &Memory) -> Result<bool> {
        // Calculate different similarity metrics
        let semantic_similarity = self.calculate_semantic_similarity(memory1, memory2);
        let content_similarity = self.calculate_content_similarity(memory1, memory2);
        let metadata_similarity = self.calculate_metadata_similarity(memory1, memory2);

        // Weighted combination of similarities
        let combined_similarity = semantic_similarity * 0.5 
            + content_similarity * 0.3 
            + metadata_similarity * 0.2;

        Ok(combined_similarity >= self.similarity_threshold)
    }
}

/// Simple rule-based duplicate detector for faster processing
pub struct RuleBasedDuplicateDetector {
    similarity_threshold: f32,
}

impl RuleBasedDuplicateDetector {
    pub fn new(similarity_threshold: f32) -> Self {
        Self { similarity_threshold }
    }

    fn calculate_simple_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        // Simple content-based similarity
        let content1 = memory1.content.to_lowercase();
        let content2 = memory2.content.to_lowercase();

        // Exact match
        if content1 == content2 {
            return 1.0;
        }

        // Length-based similarity
        let len_diff = (content1.len() as f32 - content2.len() as f32).abs();
        let max_len = content1.len().max(content2.len()) as f32;
        
        if max_len == 0.0 {
            return 1.0;
        }

        1.0 - (len_diff / max_len)
    }
}

#[async_trait]
impl DuplicateDetector for RuleBasedDuplicateDetector {
    async fn detect_duplicates(&self, _memory: &Memory) -> Result<Vec<Memory>> {
        // For rule-based detection, we would need access to existing memories
        // This is a simplified implementation
        Ok(Vec::new())
    }

    async fn merge_memories(&self, memories: &[Memory]) -> Result<Memory> {
        if memories.is_empty() {
            return Err(crate::error::MemoryError::validation("No memories to merge"));
        }

        // Simple merge: take the longest content
        let longest_memory = memories.iter()
            .max_by_key(|m| m.content.len())
            .unwrap();

        Ok(longest_memory.clone())
    }

    async fn are_similar(&self, memory1: &Memory, memory2: &Memory) -> Result<bool> {
        let similarity = self.calculate_simple_similarity(memory1, memory2);
        Ok(similarity >= self.similarity_threshold)
    }
}

/// Factory function to create duplicate detectors
pub fn create_duplicate_detector(
    vector_store: Box<dyn VectorStore>,
    llm_client: Box<dyn LLMClient>,
    use_advanced: bool,
    similarity_threshold: f32,
    merge_threshold: f32,
) -> Box<dyn DuplicateDetector> {
    if use_advanced {
        Box::new(AdvancedDuplicateDetector::new(
            vector_store,
            llm_client,
            similarity_threshold,
            merge_threshold,
        ))
    } else {
        Box::new(RuleBasedDuplicateDetector::new(similarity_threshold))
    }
}

