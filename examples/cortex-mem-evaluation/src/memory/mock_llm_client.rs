//! 模拟LLM客户端实现
//! 
//! 用于评估的简单LLM客户端

use async_trait::async_trait;
use cortex_mem_core::{
    error::Result,
    llm::{
        LLMClient, StructuredFactExtraction, DetailedFactExtraction, KeywordExtraction,
        MemoryClassification, ImportanceScore, DeduplicationResult, SummaryResult,
        LanguageDetection, EntityExtraction, ConversationAnalysis,
    },
};
use rand::Rng;
use std::collections::HashMap;

/// 模拟LLM客户端
#[derive(Clone)]
pub struct MockLLMClient {
    /// 是否启用详细输出
    verbose: bool,
}

impl MockLLMClient {
    /// 创建新的模拟LLM客户端
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
    
    /// 生成随机嵌入向量
    fn generate_random_embedding(dim: usize) -> Vec<f32> {
        let mut rng = rand::thread_rng();
        (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
    }
    
    /// 从文本中提取关键词（简化版）
    fn extract_keywords_from_text(text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut keywords = Vec::new();
        
        // 简单的关键词提取：取前5个非停用词
        let stop_words = ["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by"];
        
        for word in words {
            let word_lower = word.to_lowercase();
            if !stop_words.contains(&word_lower.as_str()) && word.len() > 3 {
                keywords.push(word.to_string());
                if keywords.len() >= 5 {
                    break;
                }
            }
        }
        
        keywords
    }
    
    /// 根据内容生成记忆类型
    fn classify_memory_from_content(content: &str) -> cortex_mem_core::types::MemoryType {
        let content_lower = content.to_lowercase();
        
        if content_lower.contains("how to") || content_lower.contains("step") || content_lower.contains("procedure") {
            cortex_mem_core::types::MemoryType::Procedural
        } else if content_lower.contains("fact") || content_lower.contains("data") || content_lower.contains("statistic") {
            cortex_mem_core::types::MemoryType::Factual
        } else if content_lower.contains("concept") || content_lower.contains("meaning") || content_lower.contains("definition") {
            cortex_mem_core::types::MemoryType::Semantic
        } else if content_lower.contains("event") || content_lower.contains("happened") || content_lower.contains("experience") {
            cortex_mem_core::types::MemoryType::Episodic
        } else if content_lower.contains("prefer") || content_lower.contains("like") || content_lower.contains("dislike") {
            cortex_mem_core::types::MemoryType::Personal
        } else {
            cortex_mem_core::types::MemoryType::Conversational
        }
    }
    
    /// 根据内容生成重要性评分
    fn score_importance_from_content(content: &str) -> f32 {
        let mut score: f32 = 0.5; // 基础分数
        
        // 基于内容长度
        let length = content.len();
        if length > 200 {
            score += 0.2;
        } else if length < 50 {
            score -= 0.1;
        }
        
        // 基于关键词
        let keywords = Self::extract_keywords_from_text(content);
        if keywords.len() > 3 {
            score += 0.1;
        }
        
        // 基于特殊标记
        if content.contains("important") || content.contains("critical") || content.contains("essential") {
            score += 0.2;
        }
        
        // 确保分数在0-1之间
        score.max(0.0).min(1.0)
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        if self.verbose {
            println!("[MockLLM] Completing prompt: {}", &prompt[..prompt.len().min(50)]);
        }
        
        // 生成简单的回复
        Ok(format!("This is a mock response to: {}", &prompt[..prompt.len().min(100)]))
    }
    
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if self.verbose {
            println!("[MockLLM] Embedding text: {}", &text[..text.len().min(50)]);
        }
        
        // 生成384维的随机嵌入向量
        Ok(Self::generate_random_embedding(384))
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if self.verbose {
            println!("[MockLLM] Embedding batch of {} texts", texts.len());
        }
        
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
    
    async fn extract_keywords(&self, content: &str) -> Result<Vec<String>> {
        if self.verbose {
            println!("[MockLLM] Extracting keywords from: {}", &content[..content.len().min(50)]);
        }
        
        Ok(Self::extract_keywords_from_text(content))
    }
    
    async fn summarize(&self, content: &str, max_length: Option<usize>) -> Result<String> {
        if self.verbose {
            println!("[MockLLM] Summarizing content: {}", &content[..content.len().min(50)]);
        }
        
        let max_len = max_length.unwrap_or(100);
        let summary = if content.len() > max_len {
            format!("{}...", &content[..max_len])
        } else {
            content.to_string()
        };
        
        Ok(summary)
    }
    
    async fn health_check(&self) -> Result<bool> {
        Ok(true) // 模拟客户端总是健康的
    }
    
    // 以下是rig extractor方法
    
    async fn extract_structured_facts(&self, prompt: &str) -> Result<StructuredFactExtraction> {
        if self.verbose {
            println!("[MockLLM] Extracting structured facts from: {}", &prompt[..prompt.len().min(50)]);
        }
        
        Ok(StructuredFactExtraction {
            facts: vec![format!("Mock fact from: {}", &prompt[..prompt.len().min(50)])],
        })
    }
    
    async fn extract_detailed_facts(&self, prompt: &str) -> Result<DetailedFactExtraction> {
        if self.verbose {
            println!("[MockLLM] Extracting detailed facts from: {}", &prompt[..prompt.len().min(50)]);
        }
        
        let fact = cortex_mem_core::llm::StructuredFact {
            content: format!("Mock fact: {}", &prompt[..prompt.len().min(30)]),
            importance: 0.7,
            category: "general".to_string(),
            entities: Self::extract_keywords_from_text(prompt),
            source_role: "user".to_string(),
        };
        
        Ok(DetailedFactExtraction {
            facts: vec![fact],
        })
    }
    
    async fn extract_keywords_structured(&self, prompt: &str) -> Result<KeywordExtraction> {
        if self.verbose {
            println!("[MockLLM] Extracting structured keywords from: {}", &prompt[..prompt.len().min(50)]);
        }
        
        let keywords = Self::extract_keywords_from_text(prompt);
        
        Ok(KeywordExtraction {
            keywords,
        })
    }
    
    async fn classify_memory(&self, prompt: &str) -> Result<MemoryClassification> {
        if self.verbose {
            println!("[MockLLM] Classifying memory: {}", &prompt[..prompt.len().min(50)]);
        }
        
        let memory_type = Self::classify_memory_from_content(prompt);
        
        Ok(MemoryClassification {
            memory_type: format!("{:?}", memory_type),
            confidence: 0.85,
            reasoning: "Based on content analysis".to_string(),
        })
    }
    
    async fn score_importance(&self, prompt: &str) -> Result<ImportanceScore> {
        if self.verbose {
            println!("[MockLLM] Scoring importance: {}", &prompt[..prompt.len().min(50)]);
        }
        
        let score = Self::score_importance_from_content(prompt);
        
        Ok(ImportanceScore {
            score,
            reasoning: "Based on content length and keyword density".to_string(),
        })
    }
    
    async fn check_duplicates(&self, prompt: &str) -> Result<DeduplicationResult> {
        if self.verbose {
            println!("[MockLLM] Checking duplicates: {}", &prompt[..prompt.len().min(50)]);
        }
        
        Ok(DeduplicationResult {
            is_duplicate: false,
            similarity_score: 0.3,
            original_memory_id: None,
        })
    }
    
    async fn generate_summary(&self, prompt: &str) -> Result<SummaryResult> {
        if self.verbose {
            println!("[MockLLM] Generating summary: {}", &prompt[..prompt.len().min(50)]);
        }
        
        let summary = if prompt.len() > 100 {
            format!("{}...", &prompt[..100])
        } else {
            prompt.to_string()
        };
        
        Ok(SummaryResult {
            summary,
            key_points: Self::extract_keywords_from_text(prompt),
        })
    }
    
    async fn detect_language(&self, prompt: &str) -> Result<LanguageDetection> {
        if self.verbose {
            println!("[MockLLM] Detecting language: {}", &prompt[..prompt.len().min(50)]);
        }
        
        Ok(LanguageDetection {
            language: "en".to_string(),
            confidence: 0.95,
        })
    }
    
    async fn extract_entities(&self, prompt: &str) -> Result<EntityExtraction> {
        if self.verbose {
            println!("[MockLLM] Extracting entities: {}", &prompt[..prompt.len().min(50)]);
        }
        
        let keywords = Self::extract_keywords_from_text(prompt);
        
        // 创建Entity对象
        let entities: Vec<cortex_mem_core::llm::Entity> = keywords
            .into_iter()
            .map(|text| cortex_mem_core::llm::Entity {
                text,
                label: "keyword".to_string(),
                confidence: 0.7,
            })
            .collect();
        
        Ok(EntityExtraction {
            entities,
        })
    }
    
    async fn analyze_conversation(&self, prompt: &str) -> Result<ConversationAnalysis> {
        if self.verbose {
            println!("[MockLLM] Analyzing conversation: {}", &prompt[..prompt.len().min(50)]);
        }
        
        Ok(ConversationAnalysis {
            topics: Self::extract_keywords_from_text(prompt),
            sentiment: "neutral".to_string(),
            user_intent: "information_request".to_string(),
            key_information: Self::extract_keywords_from_text(prompt),
        })
    }
}