use crate::{MemoryError, error::Result, llm::LLMClient, types::MemoryType};
use async_trait::async_trait;
use tracing::debug;

/// Trait for classifying memory types
#[async_trait]
pub trait MemoryClassifier: Send + Sync {
    /// Classify the type of a memory based on its content
    async fn classify_memory(&self, content: &str) -> Result<MemoryType>;

    /// Classify multiple memories in batch
    async fn classify_batch(&self, contents: &[String]) -> Result<Vec<MemoryType>>;

    /// Extract entities from memory content
    async fn extract_entities(&self, content: &str) -> Result<Vec<String>>;

    /// Extract topics from memory content
    async fn extract_topics(&self, content: &str) -> Result<Vec<String>>;
}

/// LLM-based memory classifier
pub struct LLMMemoryClassifier {
    llm_client: Box<dyn LLMClient>,
}

impl LLMMemoryClassifier {
    pub fn new(llm_client: Box<dyn LLMClient>) -> Self {
        Self { llm_client }
    }

    fn create_classification_prompt(&self, content: &str) -> String {
        format!(
            r#"Classify the following memory content into one of these categories:

1. Conversational - Dialogue, conversations, or interactive exchanges
2. Procedural - Instructions, how-to information, or step-by-step processes
3. Factual - Objective facts, data, or verifiable information
4. Semantic - Concepts, meanings, definitions, or general knowledge
5. Episodic - Specific events, experiences, or temporal information
6. Personal - Personal preferences, characteristics, or individual-specific information

Content: "{}"

Respond with only the category name (e.g., "Conversational", "Procedural", etc.):"#,
            content
        )
    }

    fn create_entity_extraction_prompt(&self, content: &str) -> String {
        format!(
            r#"Extract named entities from the following text. Focus on:
- People (names, roles, titles)
- Organizations (companies, institutions)
- Locations (cities, countries, places)
- Products (software, tools, brands)
- Concepts (technical terms, important keywords)

Text: "{}"

Return the entities as a comma-separated list. If no entities found, return "None"."#,
            content
        )
    }

    fn create_topic_extraction_prompt(&self, content: &str) -> String {
        format!(
            r#"Extract the main topics or themes from the following text. Focus on:
- Subject areas (technology, business, health, etc.)
- Activities (programming, cooking, traveling, etc.)
- Domains (AI, finance, education, etc.)
- Key themes or concepts

Text: "{}"

Return the topics as a comma-separated list. If no clear topics, return "None"."#,
            content
        )
    }

    fn parse_memory_type(&self, response: &str) -> MemoryType {
        let response = response.trim().to_lowercase();
        match response.as_str() {
            "conversational" => MemoryType::Conversational,
            "procedural" => MemoryType::Procedural,
            "factual" => MemoryType::Factual,
            "semantic" => MemoryType::Semantic,
            "episodic" => MemoryType::Episodic,
            "personal" => MemoryType::Personal,
            _ => MemoryType::Conversational, // Default fallback
        }
    }

    fn parse_list_response(&self, response: &str) -> Vec<String> {
        if response.trim().to_lowercase() == "none" {
            return Vec::new();
        }

        response
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[async_trait]
impl MemoryClassifier for LLMMemoryClassifier {
    async fn classify_memory(&self, content: &str) -> Result<MemoryType> {
        let prompt = self.create_classification_prompt(content);

        // Use rig's structured extractor instead of string parsing
        match self.llm_client.classify_memory(&prompt).await {
            Ok(classification) => {
                let memory_type = match classification.memory_type.as_str() {
                    "Conversational" => MemoryType::Conversational,
                    "Procedural" => MemoryType::Procedural,
                    "Factual" => MemoryType::Factual,
                    "Semantic" => MemoryType::Semantic,
                    "Episodic" => MemoryType::Episodic,
                    "Personal" => MemoryType::Personal,
                    _ => MemoryType::Conversational, // Default fallback
                };
                Ok(memory_type)
            }
            Err(e) => {
                // Fallback to traditional method if extractor fails
                debug!(
                    "Rig extractor failed, falling back to traditional method: {}",
                    e
                );

                #[cfg(debug_assertions)]
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                let response = self.llm_client.complete(&prompt).await?;
                Ok(self.parse_memory_type(&response))
            }
        }
    }

    async fn classify_batch(&self, contents: &[String]) -> Result<Vec<MemoryType>> {
        let mut results = Vec::with_capacity(contents.len());

        for content in contents {
            let memory_type = self.classify_memory(content).await?;
            results.push(memory_type);
        }

        Ok(results)
    }

    async fn extract_entities(&self, content: &str) -> Result<Vec<String>> {
        let prompt = self.create_entity_extraction_prompt(content);

        // Use rig's structured extractor instead of string parsing
        match self.llm_client.extract_entities(&prompt).await {
            Ok(entity_extraction) => {
                let entities: Vec<String> = entity_extraction
                    .entities
                    .into_iter()
                    .map(|entity| entity.text)
                    .collect();
                Ok(entities)
            }
            Err(e) => {
                // Fallback to traditional method if extractor fails
                debug!(
                    "Rig extractor failed, falling back to traditional method: {}",
                    e
                );
                #[cfg(debug_assertions)]
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                let response = self.llm_client.complete(&prompt).await?;
                Ok(self.parse_list_response(&response))
            }
        }
    }

    async fn extract_topics(&self, content: &str) -> Result<Vec<String>> {
        let prompt = self.create_topic_extraction_prompt(content);

        #[cfg(debug_assertions)]
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let response = self.llm_client.complete(&prompt).await?;
        Ok(self.parse_list_response(&response))
    }
}

/// Rule-based memory classifier for faster processing
pub struct RuleBasedMemoryClassifier;

impl RuleBasedMemoryClassifier {
    pub fn new() -> Self {
        Self
    }

    fn classify_by_keywords(&self, content: &str) -> Option<MemoryType> {
        let content_lower = content.to_lowercase();

        // Personal indicators
        let personal_keywords = [
            "i like",
            "我喜欢",
            "i prefer",
            "我擅长",
            "my name",
            "我叫",
            "我的名字叫",
            "i am",
            "我是",
            "i work",
            "我的工作",
            "i live",
            "我住在",
            "my favorite",
            "我擅长",
            "i hate",
            "我讨厌",
            "i love",
            "我喜欢",
            "my birthday",
            "我的生日",
            "my phone",
            "我的联系方式",
            "我的手机号",
            "我的电话",
            "my email",
            "我的邮箱",
            "my address",
            "我的住址",
            "i want",
            "我想要",
            "i need",
            "我需要",
            "i think",
            "我认为",
        ];

        // Procedural indicators
        let procedural_keywords = [
            "how to",
            "怎么",
            "step",
            "步骤",
            "first",
            "首先",
            "then",
            "然后",
            "其次",
            "next",
            "接下来",
            "finally",
            "最后",
            "instructions",
            "说明",
            "procedure",
            "步骤",
            "process",
            "流程",
            "method",
            "方法",
            "way to",
            "办法",
            "tutorial",
            "尝试",
            "guide",
            "指导",
            "recipe",
            "菜谱",
            "食谱",
            "algorithm",
            "算法",
        ];

        // Factual indicators
        let factual_keywords = [
            "fact",
            "事实",
            "data",
            "数据",
            "statistics",
            "统计数据",
            "number",
            "date",
            "time",
            "location",
            "address",
            "phone",
            "email",
            "website",
            "price",
            "cost",
            "amount",
            "quantity",
            "measurement",
        ];

        // Episodic indicators
        let episodic_keywords = [
            "yesterday",
            "昨天",
            "today",
            "今天",
            "tomorrow",
            "明天",
            "last week",
            "上周",
            "next month",
            "下个月",
            "happened",
            "发生",
            "occurred",
            "event",
            "日程",
            "meeting",
            "约会",
            "appointment",
            "约定",
            "remember when",
            "that time",
            "那时候",
            "experience",
            "经历",
            "体验",
            "story",
        ];

        // Semantic indicators
        let semantic_keywords = [
            "definition",
            "定义",
            "meaning",
            "意义",
            "concept",
            "概念",
            "theory",
            "理论",
            "principle",
            "原则",
            "knowledge",
            "知识",
            "understanding",
            "领悟",
            "explanation",
            "解释",
            "阐释",
            "describes",
            "描述",
            "refers to",
            "参考",
            "means",
            "意味",
            "is defined as",
            "界定为",
        ];

        // Check for personal keywords first (highest priority)
        if personal_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            return Some(MemoryType::Personal);
        }

        // Check for procedural keywords
        if procedural_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            return Some(MemoryType::Procedural);
        }

        // Check for episodic keywords
        if episodic_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            return Some(MemoryType::Episodic);
        }

        // Check for factual keywords
        if factual_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            return Some(MemoryType::Factual);
        }

        // Check for semantic keywords
        if semantic_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            return Some(MemoryType::Semantic);
        }

        None
    }

    fn extract_simple_entities(&self, content: &str) -> Vec<String> {
        let mut entities = Vec::new();

        // Simple pattern matching for common entities
        let words: Vec<&str> = content.split_whitespace().collect();

        for word in words {
            // Capitalized words might be entities (names, places, etc.)
            if word.len() > 2 && word.chars().next().unwrap().is_uppercase() {
                let clean_word = word.trim_matches(|c: char| !c.is_alphabetic());
                if !clean_word.is_empty() && clean_word.len() > 2 {
                    entities.push(clean_word.to_string());
                }
            }
        }

        entities.sort();
        entities.dedup();
        entities
    }

    fn extract_simple_topics(&self, content: &str) -> Vec<String> {
        let mut topics = Vec::new();
        let content_lower = content.to_lowercase();

        // Technology topics
        let tech_keywords = [
            "programming",
            "代码",
            "程序",
            "编码",
            "software",
            "软件",
            "computer",
            "计算机",
            "ai",
            "大模型",
            "machine learning",
            "机械学习",
            "神经网络",
            "database",
            "数据库",
        ];
        if tech_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            topics.push("Technology".to_string());
        }

        // Business topics
        let business_keywords = [
            "business",
            "company",
            "meeting",
            "project",
            "work",
            "office",
            "商业",
            "公司",
            "会议",
            "商业项目",
            "办公",
            "办公室",
        ];
        if business_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            topics.push("Business".to_string());
        }

        // Personal topics
        let personal_keywords = [
            "family",
            "friend",
            "hobby",
            "interest",
            "personal",
            "家庭",
            "朋友",
            "爱好",
            "兴趣",
            "个人的",
        ];
        if personal_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            topics.push("Personal".to_string());
        }

        // Health topics
        let health_keywords = [
            "health", "medical", "doctor", "medicine", "exercise", "健康", "医疗", "医生", "药",
            "体检",
        ];
        if health_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
        {
            topics.push("Health".to_string());
        }

        topics
    }
}

#[async_trait]
impl MemoryClassifier for RuleBasedMemoryClassifier {
    async fn classify_memory(&self, content: &str) -> Result<MemoryType> {
        self.classify_by_keywords(content)
            .ok_or(MemoryError::NotFound { id: "".to_owned() })
    }

    async fn classify_batch(&self, contents: &[String]) -> Result<Vec<MemoryType>> {
        let mut results = Vec::with_capacity(contents.len());

        for content in contents {
            let memory_type = self
                .classify_by_keywords(content)
                .ok_or(MemoryError::NotFound { id: "".to_owned() })?;
            results.push(memory_type);
        }

        Ok(results)
    }

    async fn extract_entities(&self, content: &str) -> Result<Vec<String>> {
        Ok(self.extract_simple_entities(content))
    }

    async fn extract_topics(&self, content: &str) -> Result<Vec<String>> {
        Ok(self.extract_simple_topics(content))
    }
}

/// Hybrid classifier that combines LLM and rule-based approaches
pub struct HybridMemoryClassifier {
    llm_classifier: LLMMemoryClassifier,
    rule_classifier: RuleBasedMemoryClassifier,
    use_llm_threshold: usize, // Use LLM for content longer than this
}

impl HybridMemoryClassifier {
    pub fn new(llm_client: Box<dyn LLMClient>, use_llm_threshold: usize) -> Self {
        Self {
            llm_classifier: LLMMemoryClassifier::new(llm_client),
            rule_classifier: RuleBasedMemoryClassifier::new(),
            use_llm_threshold,
        }
    }
}

#[async_trait]
impl MemoryClassifier for HybridMemoryClassifier {
    async fn classify_memory(&self, content: &str) -> Result<MemoryType> {
        if content.len() > self.use_llm_threshold {
            self.llm_classifier.classify_memory(content).await
        } else {
            self.rule_classifier.classify_memory(content).await
        }
    }

    async fn classify_batch(&self, contents: &[String]) -> Result<Vec<MemoryType>> {
        let mut results = Vec::with_capacity(contents.len());

        for content in contents {
            let memory_type = self.classify_memory(content).await?;
            results.push(memory_type);
        }

        Ok(results)
    }

    async fn extract_entities(&self, content: &str) -> Result<Vec<String>> {
        if content.len() > self.use_llm_threshold {
            self.llm_classifier.extract_entities(content).await
        } else {
            self.rule_classifier.extract_entities(content).await
        }
    }

    async fn extract_topics(&self, content: &str) -> Result<Vec<String>> {
        if content.len() > self.use_llm_threshold {
            self.llm_classifier.extract_topics(content).await
        } else {
            self.rule_classifier.extract_topics(content).await
        }
    }
}

/// Factory function to create memory classifiers
pub fn create_memory_classifier(
    llm_client: Box<dyn LLMClient>,
    use_llm: bool,
    hybrid_threshold: Option<usize>,
) -> Box<dyn MemoryClassifier> {
    match (use_llm, hybrid_threshold) {
        (true, Some(threshold)) => Box::new(HybridMemoryClassifier::new(llm_client, threshold)),
        (true, None) => Box::new(LLMMemoryClassifier::new(llm_client)),
        (false, _) => Box::new(RuleBasedMemoryClassifier::new()),
    }
}
