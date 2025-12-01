use async_trait::async_trait;
use rig::providers::openai::CompletionModel;
use rig::{
    agent::Agent,
    client::{CompletionClient, EmbeddingsClient},
    completion::Prompt,
    embeddings::EmbeddingsBuilder,
    providers::openai::{Client, EmbeddingModel as OpenAIEmbeddingModel},
};
use tracing::{debug, error, info};

use crate::{
    EmbeddingConfig,
    config::LLMConfig,
    error::{MemoryError, Result},
    llm::extractor_types::*,
};

/// LLM client trait for text generation and embeddings
#[async_trait]
pub trait LLMClient: Send + Sync + dyn_clone::DynClone {
    /// Generate text completion
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Extract key information from memory content
    async fn extract_keywords(&self, content: &str) -> Result<Vec<String>>;

    /// Summarize memory content
    async fn summarize(&self, content: &str, max_length: Option<usize>) -> Result<String>;

    /// Check if the LLM service is available
    async fn health_check(&self) -> Result<bool>;

    // New extractor-based methods

    /// Extract structured facts from text using rig extractor
    async fn extract_structured_facts(&self, prompt: &str) -> Result<StructuredFactExtraction>;

    /// Extract detailed facts with metadata using rig extractor
    async fn extract_detailed_facts(&self, prompt: &str) -> Result<DetailedFactExtraction>;

    /// Extract keywords using rig extractor
    async fn extract_keywords_structured(&self, prompt: &str) -> Result<KeywordExtraction>;

    /// Classify memory type using rig extractor
    async fn classify_memory(&self, prompt: &str) -> Result<MemoryClassification>;

    /// Score memory importance using rig extractor
    async fn score_importance(&self, prompt: &str) -> Result<ImportanceScore>;

    /// Check for duplicates using rig extractor
    async fn check_duplicates(&self, prompt: &str) -> Result<DeduplicationResult>;

    /// Generate summary using rig extractor
    async fn generate_summary(&self, prompt: &str) -> Result<SummaryResult>;

    /// Detect language using rig extractor
    async fn detect_language(&self, prompt: &str) -> Result<LanguageDetection>;

    /// Extract entities using rig extractor
    async fn extract_entities(&self, prompt: &str) -> Result<EntityExtraction>;

    /// Analyze conversation using rig extractor
    async fn analyze_conversation(&self, prompt: &str) -> Result<ConversationAnalysis>;
}

dyn_clone::clone_trait_object!(LLMClient);

/// OpenAI-based LLM client implementation using rig
pub struct OpenAILLMClient {
    completion_model: Agent<CompletionModel>,
    completion_model_name: String,
    embedding_model: OpenAIEmbeddingModel,
    client: Client,
}

impl OpenAILLMClient {
    /// Create a new OpenAI LLM client
    pub fn new(llm_config: &LLMConfig, embedding_config: &EmbeddingConfig) -> Result<Self> {
        let client = Client::builder(&llm_config.api_key)
            .base_url(&llm_config.api_base_url)
            .build();

        let completion_model: Agent<CompletionModel> = client
            .completion_model(&llm_config.model_efficient)
            .completions_api()
            .into_agent_builder()
            .temperature(llm_config.temperature as f64)
            .max_tokens(llm_config.max_tokens as u64)
            .build();

        let embedding_client = Client::builder(&embedding_config.api_key)
            .base_url(&embedding_config.api_base_url)
            .build();
        let embedding_model = embedding_client.embedding_model(&embedding_config.model_name);

        Ok(Self {
            completion_model,
            completion_model_name: llm_config.model_efficient.clone(),
            embedding_model,
            client,
        })
    }

    /// Build a prompt for keyword extraction
    fn build_keyword_prompt(&self, content: &str) -> String {
        format!(
            "Extract the most important keywords and key phrases from the following text. \
            Return only the keywords separated by commas, without any additional explanation.\n\n\
            Text: {}\n\n\
            Keywords:",
            content
        )
    }

    /// Build a prompt for summarization
    fn build_summary_prompt(&self, content: &str, max_length: Option<usize>) -> String {
        let length_instruction = match max_length {
            Some(len) => format!("in approximately {} words", len),
            None => "concisely".to_string(),
        };

        format!(
            "Summarize the following text {}. Focus on the main points and key information.\n\n\
            Text: {}\n\n\
            Summary:",
            length_instruction, content
        )
    }

    /// Parse keywords from LLM response
    fn parse_keywords(&self, response: &str) -> Vec<String> {
        response
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

impl Clone for OpenAILLMClient {
    fn clone(&self) -> Self {
        Self {
            completion_model: self.completion_model.clone(),
            completion_model_name: self.completion_model_name.clone(),
            embedding_model: self.embedding_model.clone(),
            client: self.client.clone(),
        }
    }
}

#[async_trait]
impl LLMClient for OpenAILLMClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let response = self
            .completion_model
            .prompt(prompt)
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))?;

        debug!("Generated completion for prompt length: {}", prompt.len());
        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        Ok(response)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let builder = EmbeddingsBuilder::new(self.embedding_model.clone())
            .document(text)
            .map_err(|e| MemoryError::LLM(e.to_string()))?;

        let embeddings = builder
            .build()
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))?;

        if let Some((_, embedding)) = embeddings.first() {
            debug!("Generated embedding for text length: {}", text.len());
            Ok(embedding.first().vec.iter().map(|&x| x as f32).collect())
        } else {
            Err(MemoryError::LLM("No embedding generated".to_string()))
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();

        // Process in batches to avoid rate limits
        for text in texts {
            let embedding = self.embed(text).await?;
            results.push(embedding);
        }

        debug!("Generated embeddings for {} texts", texts.len());
        Ok(results)
    }

    async fn extract_keywords(&self, content: &str) -> Result<Vec<String>> {
        let prompt = self.build_keyword_prompt(content);

        // Use rig's structured extractor instead of string parsing
        match self.extract_keywords_structured(&prompt).await {
            Ok(keyword_extraction) => {
                debug!(
                    "Extracted {} keywords from content using rig extractor",
                    keyword_extraction.keywords.len()
                );
                Ok(keyword_extraction.keywords)
            }
            Err(e) => {
                // Fallback to traditional method if extractor fails
                debug!(
                    "Rig extractor failed, falling back to traditional method: {}",
                    e
                );

                #[cfg(debug_assertions)]
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                let response = self.complete(&prompt).await?;
                let keywords = self.parse_keywords(&response);
                debug!(
                    "Extracted {} keywords from content using fallback method",
                    keywords.len()
                );
                Ok(keywords)
            }
        }
    }

    async fn summarize(&self, content: &str, max_length: Option<usize>) -> Result<String> {
        let prompt = self.build_summary_prompt(content, max_length);

        // Use rig's structured extractor instead of string parsing
        match self.generate_summary(&prompt).await {
            Ok(summary_result) => {
                debug!(
                    "Generated summary of length: {} using rig extractor",
                    summary_result.summary.len()
                );
                Ok(summary_result.summary.trim().to_string())
            }
            Err(e) => {
                // Fallback to traditional method if extractor fails
                debug!(
                    "Rig extractor failed, falling back to traditional method: {}",
                    e
                );
                let summary = self.complete(&prompt).await?;
                debug!(
                    "Generated summary of length: {} using fallback method",
                    summary.len()
                );
                Ok(summary.trim().to_string())
            }
        }
    }

    async fn health_check(&self) -> Result<bool> {
        // Try a simple embedding request to check if the service is available
        match self.embed("health check").await {
            Ok(_) => {
                info!("LLM service health check passed");
                Ok(true)
            }
            Err(e) => {
                error!("LLM service health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn extract_structured_facts(&self, prompt: &str) -> Result<StructuredFactExtraction> {
        let extractor = self
            .client
            .extractor_completions_api::<StructuredFactExtraction>(&self.completion_model_name)
            .preamble(prompt)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn extract_detailed_facts(&self, prompt: &str) -> Result<DetailedFactExtraction> {
        let extractor = self
            .client
            .extractor_completions_api::<DetailedFactExtraction>(&self.completion_model_name)
            .preamble(prompt)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn extract_keywords_structured(&self, prompt: &str) -> Result<KeywordExtraction> {
        let extractor = self
            .client
            .extractor_completions_api::<KeywordExtraction>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(500)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn classify_memory(&self, prompt: &str) -> Result<MemoryClassification> {
        let extractor = self
            .client
            .extractor_completions_api::<MemoryClassification>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(500)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn score_importance(&self, prompt: &str) -> Result<ImportanceScore> {
        let extractor = self
            .client
            .extractor_completions_api::<ImportanceScore>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(500)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn check_duplicates(&self, prompt: &str) -> Result<DeduplicationResult> {
        let extractor = self
            .client
            .extractor_completions_api::<DeduplicationResult>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(500)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn generate_summary(&self, prompt: &str) -> Result<SummaryResult> {
        let extractor = self
            .client
            .extractor_completions_api::<SummaryResult>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(1000)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn detect_language(&self, prompt: &str) -> Result<LanguageDetection> {
        let extractor = self
            .client
            .extractor_completions_api::<LanguageDetection>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(200)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn extract_entities(&self, prompt: &str) -> Result<EntityExtraction> {
        let extractor = self
            .client
            .extractor_completions_api::<EntityExtraction>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(1000)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }

    async fn analyze_conversation(&self, prompt: &str) -> Result<ConversationAnalysis> {
        let extractor = self
            .client
            .extractor_completions_api::<ConversationAnalysis>(&self.completion_model_name)
            .preamble(prompt)
            .max_tokens(1500)
            .build();

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        extractor
            .extract("")
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))
    }
}

/// Factory function to create LLM clients based on configuration
pub fn create_llm_client(
    llm_config: &LLMConfig,
    embedding_config: &EmbeddingConfig,
) -> Result<Box<dyn LLMClient>> {
    // For now, we only support OpenAI
    let client = OpenAILLMClient::new(llm_config, embedding_config)?;
    Ok(Box::new(client))
}
