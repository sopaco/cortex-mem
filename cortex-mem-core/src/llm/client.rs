use crate::Result;
use rig::providers::openai::Client;
use serde::{Deserialize, Serialize};

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model_efficient: String,
    pub temperature: f32,
    pub max_tokens: usize,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            api_base_url: std::env::var("LLM_API_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: std::env::var("LLM_API_KEY")
                .unwrap_or_else(|_| "".to_string()),
            model_efficient: std::env::var("LLM_MODEL")
                .unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
            temperature: 0.1,
            max_tokens: 4096,
        }
    }
}

/// Memory extraction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExtractionResponse {
    pub facts: Vec<ExtractedFactRaw>,
    pub decisions: Vec<ExtractedDecisionRaw>,
    pub entities: Vec<ExtractedEntityRaw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFactRaw {
    pub content: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub confidence: f32,
    #[serde(default)]
    pub importance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDecisionRaw {
    pub decision: String,
    pub context: String,
    pub rationale: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntityRaw {
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub confidence: f32,
}

/// LLM Client trait for dependency injection and testing
#[async_trait::async_trait]
pub trait LLMClient: Send + Sync {
    /// Simple completion without tools or streaming
    async fn complete(&self, prompt: &str) -> Result<String>;
    
    /// Generate completion with system message
    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String>;
    
    /// Extract memories from conversation
    async fn extract_memories(&self, prompt: &str) -> Result<MemoryExtractionResponse>;
    
    /// Extract structured facts using rig extractor
    async fn extract_structured_facts(&self, prompt: &str) -> Result<crate::llm::extractor_types::StructuredFactExtraction>;
    
    /// Extract detailed facts using rig extractor
    async fn extract_detailed_facts(&self, prompt: &str) -> Result<crate::llm::extractor_types::DetailedFactExtraction>;
    
    /// Get the model name
    fn model_name(&self) -> &str;
    
    /// Get the config
    fn config(&self) -> &LLMConfig;
}

/// LLM Client wrapper for rig-core
/// 
/// This is a lightweight wrapper that creates agents for LLM interactions.
/// Following the rig pattern: Client -> CompletionModel -> Agent
pub struct LLMClientImpl {
    client: Client,
    config: LLMConfig,
}

impl LLMClientImpl {
    /// Create a new LLM client
    /// 
    /// Note: For rig-core 0.29+, we use Client::new() and then configure the client
    /// with custom base URL through environment variables or client methods
    pub fn new(config: LLMConfig) -> Result<Self> {
        // Using Client::builder pattern from rig-core 0.23
        // This matches the pattern used in examples/cortex-mem-tars
        let client = Client::builder(&config.api_key)
            .base_url(&config.api_base_url)
            .build();

        Ok(Self { client, config })
    }

    /// Create a default LLM config
    pub fn default_config() -> LLMConfig {
        LLMConfig::default()
    }

    /// Create an agent with a system prompt
    /// 
    /// This is the recommended way to interact with LLMs in rig-core.
    /// Returns an Agent that can handle streaming and tool calls.
    pub async fn create_agent(&self, system_prompt: &str) -> Result<rig::agent::Agent<rig::providers::openai::CompletionModel>> {
        use rig::client::CompletionClient;
        
        let agent = self.client
            .completion_model(&self.config.model_efficient)
            .completions_api()
            .into_agent_builder()
            .preamble(system_prompt)
            .build();
            
        Ok(agent)
    }

    /// Simple completion without tools or streaming
    /// For basic use cases - creates a temporary agent
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        use rig::completion::Prompt;
        
        let agent = self.create_agent("You are a helpful assistant.").await?;
        let response = agent
            .prompt(prompt)
            .await
            .map_err(|e| crate::Error::Llm(format!("LLM completion failed: {}", e)))?;

        Ok(response)
    }

    /// Generate completion with system message
    pub async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String> {
        use rig::completion::Prompt;
        
        let agent = self.create_agent(system).await?;
        let response = agent
            .prompt(prompt)
            .await
            .map_err(|e| crate::Error::Llm(format!("LLM completion failed: {}", e)))?;
            
        Ok(response)
    }

    /// Extract memories from conversation
    pub async fn extract_memories(&self, prompt: &str) -> Result<MemoryExtractionResponse> {
        let response: String = self.complete(prompt).await?;
        
        // Debug: print raw LLM response
        eprintln!("\n[DEBUG] LLM Raw Response:");
        eprintln!("{}", response);
        eprintln!("[DEBUG] Response length: {} chars\n", response.len());
        
        // Try to parse as structured response first
        if let Ok(extracted) = serde_json::from_str::<MemoryExtractionResponse>(&response) {
            eprintln!("[DEBUG] Successfully parsed as MemoryExtractionResponse");
            return Ok(extracted);
        }
        
        // Try to parse as just an array of facts (fallback)
        if let Ok(facts) = serde_json::from_str::<Vec<ExtractedFactRaw>>(&response) {
            eprintln!("[DEBUG] Parsed as facts array, found {} facts", facts.len());
            return Ok(MemoryExtractionResponse {
                facts,
                decisions: Vec::new(),
                entities: Vec::new(),
            });
        }
        
        // Try to parse as just an array of decisions (fallback)
        if let Ok(decisions) = serde_json::from_str::<Vec<ExtractedDecisionRaw>>(&response) {
            eprintln!("[DEBUG] Parsed as decisions array, found {} decisions", decisions.len());
            return Ok(MemoryExtractionResponse {
                facts: Vec::new(),
                decisions,
                entities: Vec::new(),
            });
        }
        
        // Try to parse as just an array of entities (fallback)
        if let Ok(entities) = serde_json::from_str::<Vec<ExtractedEntityRaw>>(&response) {
            eprintln!("[DEBUG] Parsed as entities array, found {} entities", entities.len());
            return Ok(MemoryExtractionResponse {
                facts: Vec::new(),
                decisions: Vec::new(),
                entities,
            });
        }
        
        eprintln!("[DEBUG] Failed to parse JSON, returning empty extraction");
        // If all parsing fails, return empty extraction
        Ok(MemoryExtractionResponse {
            facts: Vec::new(),
            decisions: Vec::new(),
            entities: Vec::new(),
        })
    }

    /// Get the underlying rig Client
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait::async_trait]
impl LLMClient for LLMClientImpl {
    async fn complete(&self, prompt: &str) -> Result<String> {
        use rig::completion::Prompt;
        
        let agent = self.create_agent("You are a helpful assistant.").await?;
        let response = agent
            .prompt(prompt)
            .await
            .map_err(|e| crate::Error::Llm(format!("LLM completion failed: {}", e)))?;

        Ok(response)
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String> {
        use rig::completion::Prompt;
        
        let agent = self.create_agent(system).await?;
        let response = agent
            .prompt(prompt)
            .await
            .map_err(|e| crate::Error::Llm(format!("LLM completion failed: {}", e)))?;
            
        Ok(response)
    }

    async fn extract_memories(&self, prompt: &str) -> Result<MemoryExtractionResponse> {
        let response: String = self.complete(prompt).await?;
        
        // Try to parse as structured response first
        if let Ok(extracted) = serde_json::from_str::<MemoryExtractionResponse>(&response) {
            return Ok(extracted);
        }
        
        // Try to parse as just an array of facts (fallback)
        if let Ok(facts) = serde_json::from_str::<Vec<ExtractedFactRaw>>(&response) {
            return Ok(MemoryExtractionResponse {
                facts,
                decisions: Vec::new(),
                entities: Vec::new(),
            });
        }
        
        // Try to parse as just an array of decisions (fallback)
        if let Ok(decisions) = serde_json::from_str::<Vec<ExtractedDecisionRaw>>(&response) {
            return Ok(MemoryExtractionResponse {
                facts: Vec::new(),
                decisions,
                entities: Vec::new(),
            });
        }
        
        // Try to parse as just an array of entities (fallback)
        if let Ok(entities) = serde_json::from_str::<Vec<ExtractedEntityRaw>>(&response) {
            return Ok(MemoryExtractionResponse {
                facts: Vec::new(),
                decisions: Vec::new(),
                entities,
            });
        }
        
        // If all parsing fails, return empty extraction
        Ok(MemoryExtractionResponse {
            facts: Vec::new(),
            decisions: Vec::new(),
            entities: Vec::new(),
        })
    }

    async fn extract_structured_facts(&self, prompt: &str) -> Result<crate::llm::extractor_types::StructuredFactExtraction> {
        // Simplified implementation - just return empty facts for now
        // TODO: Implement proper extraction using rig's extractor API
        let response = self.complete(prompt).await?;
        
        // Try to parse as structured facts
        if let Ok(facts) = serde_json::from_str::<crate::llm::extractor_types::StructuredFactExtraction>(&response) {
            return Ok(facts);
        }
        
        // Fallback: return empty extraction
        Ok(crate::llm::extractor_types::StructuredFactExtraction {
            facts: vec![],
        })
    }

    async fn extract_detailed_facts(&self, prompt: &str) -> Result<crate::llm::extractor_types::DetailedFactExtraction> {
        // Simplified implementation - just return empty facts for now
        // TODO: Implement proper extraction using rig's extractor API
        let response = self.complete(prompt).await?;
        
        // Try to parse as detailed facts
        if let Ok(facts) = serde_json::from_str::<crate::llm::extractor_types::DetailedFactExtraction>(&response) {
            return Ok(facts);
        }
        
        // Fallback: return empty extraction
        Ok(crate::llm::extractor_types::DetailedFactExtraction {
            facts: vec![],
        })
    }

    fn model_name(&self) -> &str {
        &self.config.model_efficient
    }

    fn config(&self) -> &LLMConfig {
        &self.config
    }
}

// 核心功能测试已迁移至 cortex-mem-tools/tests/core_functionality_tests.rs
