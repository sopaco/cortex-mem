use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    config::MemoryConfig,
    error::{MemoryError, Result},
    llm::LLMClient,
    memory::{
        classification::{MemoryClassifier, create_memory_classifier},
        deduplication::{DuplicateDetector, create_duplicate_detector},
        extractor::{FactExtractor, create_fact_extractor},
        importance::{ImportanceEvaluator, create_importance_evaluator},
        prompts::PROCEDURAL_MEMORY_SYSTEM_PROMPT,
        updater::{MemoryAction, MemoryUpdater, create_memory_updater},
    },
    types::{Filters, Memory, MemoryEvent, MemoryMetadata, MemoryResult, MemoryType, ScoredMemory},
    vector_store::VectorStore,
};

/// Core memory manager that orchestrates memory operations
pub struct MemoryManager {
    vector_store: Box<dyn VectorStore>,
    llm_client: Box<dyn LLMClient>,
    config: MemoryConfig,
    fact_extractor: Box<dyn FactExtractor + 'static>,
    memory_updater: Box<dyn MemoryUpdater + 'static>,
    importance_evaluator: Box<dyn ImportanceEvaluator + 'static>,
    duplicate_detector: Box<dyn DuplicateDetector + 'static>,
    memory_classifier: Box<dyn MemoryClassifier + 'static>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(
        vector_store: Box<dyn VectorStore>,
        llm_client: Box<dyn LLMClient>,
        config: MemoryConfig,
    ) -> Self {
        // Create extractors/updaters with cloned boxes
        let fact_extractor = create_fact_extractor(dyn_clone::clone_box(llm_client.as_ref()));
        let memory_updater = create_memory_updater(
            dyn_clone::clone_box(llm_client.as_ref()),
            dyn_clone::clone_box(vector_store.as_ref()),
            config.similarity_threshold,
            config.merge_threshold,
        );
        let importance_evaluator = create_importance_evaluator(
            dyn_clone::clone_box(llm_client.as_ref()),
            config.auto_enhance, // Use LLM evaluation when auto_enhance is enabled
            Some(0.5),           // Hybrid threshold
        );
        let duplicate_detector = create_duplicate_detector(
            dyn_clone::clone_box(vector_store.as_ref()),
            dyn_clone::clone_box(llm_client.as_ref()),
            config.auto_enhance, // Use advanced detection when auto_enhance is enabled
            config.similarity_threshold,
            config.merge_threshold,
        );
        let memory_classifier = create_memory_classifier(
            dyn_clone::clone_box(llm_client.as_ref()),
            config.auto_enhance, // Use LLM classification when auto_enhance is enabled
            Some(100),           // Hybrid threshold: use LLM for content longer than 100 chars
        );

        Self {
            vector_store,
            llm_client,
            config,
            fact_extractor,
            memory_updater,
            importance_evaluator,
            duplicate_detector,
            memory_classifier,
        }
    }

    /// Generate a hash for memory content
    fn generate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get a reference to the LLM client
    pub fn llm_client(&self) -> &dyn LLMClient {
        self.llm_client.as_ref()
    }

    /// Check if memory with the same content already exists
    async fn check_duplicate(&self, content: &str, filters: &Filters) -> Result<Option<Memory>> {
        let hash = self.generate_hash(content);

        // Search for memories with the same hash
        let existing_memories = self.vector_store.list(filters, Some(100)).await?;

        for memory in existing_memories {
            if memory.metadata.hash == hash {
                // Check if the existing memory has empty content
                if memory.content.trim().is_empty() {
                    warn!(
                        "Found duplicate memory {} with empty content, skipping",
                        memory.id
                    );
                    continue;
                }
                debug!("Found duplicate memory with ID: {}", memory.id);
                return Ok(Some(memory));
            }
        }

        Ok(None)
    }

    /// Enhance memory content with LLM-generated metadata
    async fn enhance_memory(&self, memory: &mut Memory) -> Result<()> {
        // Extract keywords
        if let Ok(keywords) = self.llm_client.extract_keywords(&memory.content).await {
            memory.metadata.custom.insert(
                "keywords".to_string(),
                serde_json::Value::Array(
                    keywords
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        // Generate summary if content is long
        if memory.content.len() > self.config.auto_summary_threshold {
            if let Ok(summary) = self.llm_client.summarize(&memory.content, Some(200)).await {
                memory
                    .metadata
                    .custom
                    .insert("summary".to_string(), serde_json::Value::String(summary));
            }
        }

        // Classify memory type and extract metadata
        if let Ok(memory_type) = self
            .memory_classifier
            .classify_memory(&memory.content)
            .await
        {
            memory.metadata.memory_type = memory_type;
        }

        // Extract entities and topics
        if let Ok(entities) = self
            .memory_classifier
            .extract_entities(&memory.content)
            .await
        {
            memory.metadata.entities = entities;
        }

        if let Ok(topics) = self.memory_classifier.extract_topics(&memory.content).await {
            memory.metadata.topics = topics;
        }

        // Evaluate importance using importance evaluator
        if let Ok(importance) = self.importance_evaluator.evaluate_importance(memory).await {
            memory.metadata.importance_score = importance;
        }

        // Check for duplicates and merge if necessary
        if let Ok(duplicates) = self.duplicate_detector.detect_duplicates(memory).await {
            if !duplicates.is_empty() {
                // Merge with existing duplicates
                let mut all_memories = vec![memory.clone()];
                all_memories.extend(duplicates);

                if let Ok(merged_memory) =
                    self.duplicate_detector.merge_memories(&all_memories).await
                {
                    *memory = merged_memory;

                    // Remove the old duplicate memories from vector store
                    for duplicate in &all_memories[1..] {
                        let _ = self.vector_store.delete(&duplicate.id).await;
                    }
                }
            }
        }

        // Extract facts using fact extractor
        // Note: This would need conversation messages, for now we skip fact extraction
        // TODO: Implement fact extraction for single memory content

        Ok(())
    }

    /// Create a new memory from content and metadata
    pub async fn create_memory(&self, content: String, metadata: MemoryMetadata) -> Result<Memory> {
        // Validate content
        if content.trim().is_empty() {
            return Err(MemoryError::Validation(
                "Content cannot be empty when creating memory".to_string(),
            ));
        }

        debug!("Creating memory with content length: {}", content.len());

        // Generate embedding
        let embedding = self.llm_client.embed(&content).await?;

        // Create memory object
        let now = Utc::now();
        let mut memory = Memory {
            id: Uuid::new_v4().to_string(),
            content: content.to_owned(),
            embedding,
            metadata: MemoryMetadata {
                hash: self.generate_hash(&content),
                ..metadata
            },
            created_at: now,
            updated_at: now,
        };

        // Enhance with LLM-generated metadata if enabled
        if self.config.auto_enhance {
            self.enhance_memory(&mut memory).await?;
        }

        Ok(memory)
    }

    /// Add memory from conversation messages with full fact extraction and update pipeline
    pub async fn add_memory(
        &self,
        messages: &[crate::types::Message],
        metadata: MemoryMetadata,
    ) -> Result<Vec<MemoryResult>> {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        // Check if this should be a procedural memory based on agent_id and memory type
        if metadata.agent_id.is_some() && metadata.memory_type == MemoryType::Procedural {
            return self.create_procedural_memory(messages, metadata).await;
        }

        // Extract facts using appropriate extraction method
        let extracted_facts = self.fact_extractor.extract_facts(messages).await?;
        let mut final_extracted_facts = extracted_facts;

        // If no facts extracted, try alternative extraction methods
        if final_extracted_facts.is_empty() {
            debug!("No facts extracted, trying alternative extraction methods");

            // Try to extract facts from user messages only
            let user_messages: Vec<_> = messages
                .iter()
                .filter(|msg| msg.role == "user")
                .cloned()
                .collect();

            if !user_messages.is_empty() {
                if let Ok(user_facts) = self.fact_extractor.extract_user_facts(&user_messages).await
                {
                    if !user_facts.is_empty() {
                        debug!(
                            "Extracted {} facts from user messages fallback",
                            user_facts.len()
                        );
                        final_extracted_facts = user_facts;
                    }
                }
            }

            // If still no facts, try to extract from individual messages
            if final_extracted_facts.is_empty() {
                let mut single_message_facts = Vec::new();
                for message in messages {
                    if let Ok(mut facts) = self
                        .fact_extractor
                        .extract_facts_from_text(&message.content)
                        .await
                    {
                        for fact in &mut facts {
                            fact.source_role = message.role.clone();
                        }
                        single_message_facts.extend(facts);
                    }
                }

                if !single_message_facts.is_empty() {
                    final_extracted_facts = single_message_facts;
                    debug!(
                        "Extracted {} facts from individual messages",
                        final_extracted_facts.len()
                    );
                }
            }

            // If still no facts, store only user messages as final fallback
            if final_extracted_facts.is_empty() {
                let user_content = messages
                    .iter()
                    .filter(|msg| msg.role == "user")
                    .map(|msg| format!("用户: {}", msg.content))
                    .collect::<Vec<_>>()
                    .join("\n");

                if !user_content.trim().is_empty() {
                    let memory_id = self.store(user_content.clone(), metadata).await?;
                    return Ok(vec![MemoryResult {
                        id: memory_id.clone(),
                        memory: user_content,
                        event: MemoryEvent::Add,
                        actor_id: messages.last().and_then(|msg| msg.name.clone()),
                        role: messages.last().map(|msg| msg.role.clone()),
                        previous_memory: None,
                    }]);
                }

                // Ultimate fallback: if no user content, skip storing
                debug!("No memorable content found in conversation, skipping storage");
                return Ok(vec![]);
            }
        }

        // Search for existing similar memories
        let mut all_actions = Vec::new();
        let mut created_memory_ids = Vec::new();

        for fact in &final_extracted_facts {
            // Search for similar existing memories
            let filters = Filters {
                user_id: metadata.user_id.clone(),
                agent_id: metadata.agent_id.clone(),
                run_id: metadata.run_id.clone(),
                memory_type: None, // Search across all types
                actor_id: metadata.actor_id.clone(),
                min_importance: None,
                max_importance: None,
                created_after: None,
                created_before: None,
                updated_after: None,
                updated_before: None,
                entities: None,
                topics: None,
                custom: HashMap::new(),
            };

            let query_embedding = self.llm_client.embed(&fact.content).await?;
            // 使用配置中的搜索相似度阈值进行过滤
            let existing_memories = self
                .vector_store
                .search_with_threshold(
                    &query_embedding,
                    &filters,
                    5,
                    self.config.search_similarity_threshold,
                )
                .await?;

            // Use memory updater to determine actions
            let update_result = self
                .memory_updater
                .update_memories(&[fact.clone()], &existing_memories, &metadata)
                .await?;

            // Apply the actions
            for action in &update_result.actions_performed {
                match action {
                    MemoryAction::Create { content, metadata } => {
                        let memory_id = self.store(content.clone(), metadata.clone()).await?;
                        created_memory_ids.push(memory_id.clone());

                        all_actions.push(MemoryResult {
                            id: memory_id.clone(),
                            memory: content.clone(),
                            event: MemoryEvent::Add,
                            actor_id: messages.last().and_then(|msg| msg.name.clone()),
                            role: messages.last().map(|msg| msg.role.clone()),
                            previous_memory: None,
                        });
                    }
                    MemoryAction::Update { id, content } => {
                        self.update(id, content.clone()).await?;
                        all_actions.push(MemoryResult {
                            id: id.clone(),
                            memory: content.clone(),
                            event: MemoryEvent::Update,
                            actor_id: messages.last().and_then(|msg| msg.name.clone()),
                            role: messages.last().map(|msg| msg.role.clone()),
                            previous_memory: None,
                        });
                    }
                    MemoryAction::Merge {
                        target_id,
                        source_ids,
                        merged_content,
                    } => {
                        self.update(target_id, merged_content.clone()).await?;
                        for source_id in source_ids {
                            let _ = self.delete(source_id).await;
                        }
                        all_actions.push(MemoryResult {
                            id: target_id.clone(),
                            memory: merged_content.clone(),
                            event: MemoryEvent::Update,
                            actor_id: messages.last().and_then(|msg| msg.name.clone()),
                            role: messages.last().map(|msg| msg.role.clone()),
                            previous_memory: None,
                        });
                    }
                    MemoryAction::Delete { id } => {
                        self.delete(id).await?;
                        all_actions.push(MemoryResult {
                            id: id.clone(),
                            memory: "".to_string(),
                            event: MemoryEvent::Delete,
                            actor_id: messages.last().and_then(|msg| msg.name.clone()),
                            role: messages.last().map(|msg| msg.role.clone()),
                            previous_memory: None,
                        });
                    }
                }
            }
        }

        info!(
            "Added memory from conversation: {} actions performed",
            all_actions.len()
        );
        Ok(all_actions)
    }

    /// Store a memory in the vector store
    pub async fn store(&self, content: String, metadata: MemoryMetadata) -> Result<String> {
        // Log content for debugging
        debug!(
            "Storing memory with content: '{}...'",
            content.chars().take(50).collect::<String>()
        );

        // Check if content is empty
        if content.trim().is_empty() {
            warn!("Attempting to store memory with empty content, skipping");
            return Err(MemoryError::Validation(
                "Content cannot be empty".to_string(),
            ));
        }

        // Check for duplicates if enabled
        if self.config.deduplicate {
            let filters = Filters {
                user_id: metadata.user_id.clone(),
                agent_id: metadata.agent_id.clone(),
                run_id: metadata.run_id.clone(),
                memory_type: Some(metadata.memory_type.clone()),
                actor_id: metadata.actor_id.clone(),
                min_importance: None,
                max_importance: None,
                created_after: None,
                created_before: None,
                updated_after: None,
                updated_before: None,
                entities: None,
                topics: None,
                custom: metadata.custom.clone(),
            };

            if let Some(existing) = self.check_duplicate(&content, &filters).await? {
                // Check if existing memory has empty content
                if existing.content.trim().is_empty() {
                    warn!(
                        "Existing memory {} has empty content, creating new memory instead",
                        existing.id
                    );
                } else {
                    info!(
                        "Duplicate memory found, returning existing ID: {}",
                        existing.id
                    );
                    return Ok(existing.id);
                }
            }
        }

        // Create and store new memory
        let memory = self.create_memory(content, metadata).await?;
        let memory_id = memory.id.clone();

        // Verify memory content before storing
        if memory.content.trim().is_empty() {
            warn!("Created memory has empty content: {}", memory_id);
        }

        self.vector_store.insert(&memory).await?;

        info!(
            "Stored new memory with ID: {} (content length: {})",
            memory_id,
            memory.content.len()
        );
        Ok(memory_id)
    }

    /// Search for similar memories with importance-weighted ranking
    pub async fn search(
        &self,
        query: &str,
        filters: &Filters,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>> {
        let search_similarity_threshold = self.config.search_similarity_threshold;
        self.search_with_threshold(query, filters, limit, search_similarity_threshold)
            .await
    }

    /// Search for similar memories with optional similarity threshold
    pub async fn search_with_threshold(
        &self,
        query: &str,
        filters: &Filters,
        limit: usize,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<ScoredMemory>> {
        // Generate query embedding
        let query_embedding = self.llm_client.embed(query).await?;

        // Use provided threshold or fall back to config
        let threshold = similarity_threshold.or(self.config.search_similarity_threshold);

        // Search in vector store with threshold
        let mut results = self
            .vector_store
            .search_with_threshold(&query_embedding, filters, limit, threshold)
            .await?;

        // Sort by combined score: similarity + importance + time freshness
        results.sort_by(|a, b| {
            let score_a = a.score * 0.7 + a.memory.metadata.importance_score * 0.3;
            let score_b = b.score * 0.7 + b.memory.metadata.importance_score * 0.3;

            // First, sort by combined score
            match score_b.partial_cmp(&score_a) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // When scores are equal, prefer newer memories
                    b.memory.created_at.cmp(&a.memory.created_at)
                }
                Some(ordering) => ordering,
            }
        });

        debug!(
            "Found {} similar memories for query with threshold {:?}",
            results.len(),
            threshold
        );
        Ok(results)
    }

    /// Search for similar memories using config threshold if set
    pub async fn search_with_config_threshold(
        &self,
        query: &str,
        filters: &Filters,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>> {
        self.search_with_threshold(
            query,
            filters,
            limit,
            self.config.search_similarity_threshold,
        )
        .await
    }

    /// Search with application-layer similarity filtering (备选方案)
    /// This method performs search first and then filters results by similarity threshold
    pub async fn search_with_app_filter(
        &self,
        query: &str,
        filters: &Filters,
        limit: usize,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<ScoredMemory>> {
        // Perform regular search first (get more results to account for filtering)
        let search_limit = if similarity_threshold.is_some() {
            limit * 3 // Get more results initially
        } else {
            limit
        };

        let mut results = self.search(query, filters, search_limit).await?;

        // Apply similarity threshold filter if provided
        if let Some(threshold) = similarity_threshold {
            results.retain(|scored_memory| scored_memory.score >= threshold);

            // Trim to requested limit if we have more results after filtering
            if results.len() > limit {
                results.truncate(limit);
            }
        }

        debug!(
            "Found {} similar memories for query with app-layer threshold {:?}",
            results.len(),
            similarity_threshold
        );
        Ok(results)
    }

    /// Retrieve a memory by ID
    pub async fn get(&self, id: &str) -> Result<Option<Memory>> {
        self.vector_store.get(id).await
    }

    /// Update memory metadata only (for reclassification)
    pub async fn update_metadata(
        &self,
        id: &str,
        new_memory_type: crate::types::MemoryType,
    ) -> Result<()> {
        self.update_complete_memory(id, None, Some(new_memory_type), None, None, None, None)
            .await
    }

    /// Update complete memory with all fields
    pub async fn update_complete_memory(
        &self,
        id: &str,
        new_content: Option<String>,
        new_memory_type: Option<crate::types::MemoryType>,
        new_importance: Option<f32>,
        new_entities: Option<Vec<String>>,
        new_topics: Option<Vec<String>>,
        new_custom: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        // Get existing memory
        let mut memory = self
            .vector_store
            .get(id)
            .await?
            .ok_or_else(|| MemoryError::NotFound { id: id.to_string() })?;

        // Update content if provided
        if let Some(content) = new_content {
            memory.content = content;
            memory.embedding = self.llm_client.embed(&memory.content).await?;
            memory.metadata.hash = self.generate_hash(&memory.content);
        }

        // Update metadata
        if let Some(memory_type) = new_memory_type {
            debug!(
                "Updating memory {} type from {:?} to {:?}",
                id, memory.metadata.memory_type, memory_type
            );
            memory.metadata.memory_type = memory_type;
        }
        if let Some(importance) = new_importance {
            memory.metadata.importance_score = importance;
        }
        if let Some(entities) = new_entities {
            memory.metadata.entities = entities;
        }
        if let Some(topics) = new_topics {
            memory.metadata.topics = topics;
        }
        if let Some(custom) = new_custom {
            memory.metadata.custom.extend(custom);
        }

        memory.updated_at = Utc::now();

        // Update in vector store
        debug!(
            "Storing updated memory with ID: {}, type: {:?}",
            id, memory.metadata.memory_type
        );
        self.vector_store.update(&memory).await?;

        info!(
            "Updated complete memory with ID: {}, new type: {:?}",
            id, memory.metadata.memory_type
        );
        Ok(())
    }

    /// Update an existing memory
    pub async fn update(&self, id: &str, content: String) -> Result<()> {
        // Get existing memory
        let mut memory = self
            .vector_store
            .get(id)
            .await?
            .ok_or_else(|| MemoryError::NotFound { id: id.to_string() })?;

        // Update content and regenerate embedding
        memory.content = content;
        memory.embedding = self.llm_client.embed(&memory.content).await?;
        memory.metadata.hash = self.generate_hash(&memory.content);
        memory.updated_at = Utc::now();

        // Re-enhance if enabled
        if self.config.auto_enhance {
            self.enhance_memory(&mut memory).await?;
        }

        // Update in vector store
        self.vector_store.update(&memory).await?;

        info!("Updated memory with ID: {}", id);
        Ok(())
    }

    /// Update an existing memory using smart merging with fact extraction
    pub async fn smart_update(&self, id: &str, new_content: String) -> Result<()> {
        // Get existing memory
        let _memory = self
            .vector_store
            .get(id)
            .await?
            .ok_or_else(|| MemoryError::NotFound { id: id.to_string() })?;

        // For now, just do a simple update
        // TODO: Implement smart merging using memory updater when we have conversation context
        self.update(id, new_content).await
    }

    /// Delete a memory by ID
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.vector_store.delete(id).await?;
        info!("Deleted memory with ID: {}", id);
        Ok(())
    }

    /// List memories with optional filters
    pub async fn list(&self, filters: &Filters, limit: Option<usize>) -> Result<Vec<Memory>> {
        self.vector_store.list(filters, limit).await
    }

    /// Create procedural memory using specialized prompt system
    /// This method follows mem0's pattern for creating procedural memories
    pub async fn create_procedural_memory(
        &self,
        messages: &[crate::types::Message],
        metadata: MemoryMetadata,
    ) -> Result<Vec<MemoryResult>> {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        // Format messages for procedural memory processing
        let formatted_messages = self.format_conversation_for_procedural_memory(messages);

        // Use procedural memory system prompt
        let prompt = format!(
            "{}

对话记录:
{}",
            PROCEDURAL_MEMORY_SYSTEM_PROMPT, formatted_messages
        );

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Get LLM response with procedural memory summarization
        let response = self.llm_client.complete(&prompt).await?;

        // Store the procedural memory result
        let memory_id = self.store(response.clone(), metadata).await?;

        info!("Created procedural memory with ID: {}", memory_id);

        Ok(vec![MemoryResult {
            id: memory_id.clone(),
            memory: response,
            event: MemoryEvent::Add,
            actor_id: messages.last().and_then(|msg| msg.name.clone()),
            role: messages.last().map(|msg| msg.role.clone()),
            previous_memory: None,
        }])
    }

    /// Format conversation messages for procedural memory processing
    fn format_conversation_for_procedural_memory(
        &self,
        messages: &[crate::types::Message],
    ) -> String {
        let mut formatted = String::new();

        for message in messages {
            match message.role.as_str() {
                "assistant" => {
                    formatted.push_str(&format!(
                        "**智能体动作**: {}
**动作结果**: {}

",
                        self.extract_action_from_assistant_message(&message.content),
                        message.content
                    ));
                }
                "user" => {
                    formatted.push_str(&format!(
                        "**用户输入**: {}
",
                        message.content
                    ));
                }
                _ => {}
            }
        }

        formatted
    }

    /// Extract action description from assistant message
    fn extract_action_from_assistant_message(&self, content: &str) -> String {
        // This is a simplified extraction - in a real implementation,
        // this could use more sophisticated NLP to identify actions
        if content.contains("正在") || content.contains("执行") || content.contains("处理") {
            "执行智能体操作".to_string()
        } else if content.contains("返回") || content.contains("结果") {
            "处理并返回结果".to_string()
        } else {
            "生成响应".to_string()
        }
    }

    /// Get memory statistics
    pub async fn get_stats(&self, filters: &Filters) -> Result<MemoryStats> {
        let memories = self.vector_store.list(filters, None).await?;

        let mut stats = MemoryStats {
            total_count: memories.len(),
            by_type: HashMap::new(),
            by_user: HashMap::new(),
            by_agent: HashMap::new(),
        };

        for memory in memories {
            // Count by type
            *stats
                .by_type
                .entry(memory.metadata.memory_type.clone())
                .or_insert(0) += 1;

            // Count by user
            if let Some(user_id) = &memory.metadata.user_id {
                *stats.by_user.entry(user_id.clone()).or_insert(0) += 1;
            }

            // Count by agent
            if let Some(agent_id) = &memory.metadata.agent_id {
                *stats.by_agent.entry(agent_id.clone()).or_insert(0) += 1;
            }
        }

        Ok(stats)
    }

    /// Perform health check on all components
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let vector_store_healthy = self.vector_store.health_check().await?;
        let llm_healthy = self.llm_client.health_check().await?;

        Ok(HealthStatus {
            vector_store: vector_store_healthy,
            llm_service: llm_healthy,
            overall: vector_store_healthy && llm_healthy,
        })
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_count: usize,
    pub by_type: HashMap<MemoryType, usize>,
    pub by_user: HashMap<String, usize>,
    pub by_agent: HashMap<String, usize>,
}

/// Health status of memory system components
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub vector_store: bool,
    pub llm_service: bool,
    pub overall: bool,
}
