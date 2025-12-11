//! 测试数据集生成器
//! 
//! 生成召回率和有效性评估的测试数据集

use anyhow::{Result, Context};
use cortex_mem_core::{Memory, MemoryType};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use tracing::info;

use super::types::*;

/// 测试数据集生成器
pub struct DatasetGenerator {
    /// 随机数生成器
    rng: StdRng,
    /// 配置
    config: GeneratorConfig,
}

/// 生成器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// 随机种子
    pub random_seed: u64,
    /// 生成的数据集大小
    pub dataset_size: usize,
    /// 每个查询的平均相关记忆数
    pub avg_relevant_memories: f64,
    /// 记忆类型分布
    pub memory_type_distribution: HashMap<MemoryType, f64>,
    /// 查询类别
    pub query_categories: Vec<String>,
    /// 查询复杂度分布
    pub complexity_distribution: HashMap<String, f64>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        let mut memory_type_distribution = HashMap::new();
        memory_type_distribution.insert(MemoryType::Conversational, 0.4);
        memory_type_distribution.insert(MemoryType::Procedural, 0.2);
        memory_type_distribution.insert(MemoryType::Factual, 0.15);
        memory_type_distribution.insert(MemoryType::Semantic, 0.1);
        memory_type_distribution.insert(MemoryType::Episodic, 0.1);
        memory_type_distribution.insert(MemoryType::Personal, 0.05);
        
        let mut complexity_distribution = HashMap::new();
        complexity_distribution.insert("simple".to_string(), 0.5);
        complexity_distribution.insert("medium".to_string(), 0.3);
        complexity_distribution.insert("complex".to_string(), 0.2);
        
        Self {
            random_seed: 42,
            dataset_size: 100,
            avg_relevant_memories: 3.0,
            memory_type_distribution,
            query_categories: vec![
                "technology".to_string(),
                "science".to_string(),
                "business".to_string(),
                "health".to_string(),
                "education".to_string(),
                "entertainment".to_string(),
                "sports".to_string(),
                "travel".to_string(),
            ],
            complexity_distribution,
        }
    }
}

impl DatasetGenerator {
    /// 创建新的数据集生成器
    pub fn new(config: GeneratorConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.random_seed);
        Self { rng, config }
    }
    
    /// 生成召回率测试数据集
    pub fn generate_recall_dataset(&mut self) -> Result<RecallTestDataset> {
        info!("生成召回率测试数据集，大小: {}", self.config.dataset_size);
        
        let mut memories = HashMap::new();
        let mut test_cases = Vec::new();
        
        // 生成记忆库
        for i in 0..(self.config.dataset_size * 3) {
            let memory_id = format!("memory_{:04}", i);
            let memory = self.generate_memory(&memory_id);
            memories.insert(memory_id, memory);
        }
        
        // 生成测试用例
        for i in 0..self.config.dataset_size {
            let query_id = format!("query_{:04}", i);
            let test_case = self.generate_recall_test_case(&query_id, &memories);
            test_cases.push(test_case);
        }
        
        // 计算元数据
        let total_relevant_memories: usize = test_cases.iter()
            .map(|tc| tc.relevant_memory_ids.len())
            .sum();
        let avg_relevant_memories = total_relevant_memories as f64 / test_cases.len() as f64;
        
        let metadata = DatasetMetadata {
            name: "recall_evaluation_dataset".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            total_test_cases: test_cases.len(),
            total_memories: memories.len(),
            avg_relevant_memories,
        };
        
        let dataset = RecallTestDataset {
            test_cases,
            memories,
            metadata,
        };
        
        info!("召回率数据集生成完成: {}个测试用例, {}个记忆",
            dataset.test_cases.len(), dataset.memories.len());
        
        Ok(dataset)
    }
    
    /// 生成有效性测试数据集
    pub fn generate_effectiveness_dataset(&mut self) -> Result<EffectivenessTestDataset> {
        info!("生成有效性测试数据集，大小: {}", self.config.dataset_size);
        
        let mut existing_memories = HashMap::new();
        let mut test_cases = Vec::new();
        
        // 生成现有记忆库
        for i in 0..(self.config.dataset_size / 2) {
            let memory_id = format!("existing_memory_{:04}", i);
            let memory = self.generate_memory(&memory_id);
            existing_memories.insert(memory_id, memory);
        }
        
        // 生成测试用例
        for i in 0..self.config.dataset_size {
            let test_case_id = format!("test_case_{:04}", i);
            let test_case = self.generate_effectiveness_test_case(&test_case_id);
            test_cases.push(test_case);
        }
        
        let metadata = DatasetMetadata {
            name: "effectiveness_evaluation_dataset".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            total_test_cases: test_cases.len(),
            total_memories: existing_memories.len(),
            avg_relevant_memories: 0.0, // 不适用于有效性数据集
        };
        
        let dataset = EffectivenessTestDataset {
            test_cases,
            existing_memories,
            metadata,
        };
        
        info!("有效性数据集生成完成: {}个测试用例, {}个现有记忆",
            dataset.test_cases.len(), dataset.existing_memories.len());
        
        Ok(dataset)
    }
    
    /// 生成记忆
    fn generate_memory(&mut self, memory_id: &str) -> Memory {
        let memory_type = self.sample_memory_type();
        let content = self.generate_memory_content(&memory_type);
        
        let mut metadata = cortex_mem_core::types::MemoryMetadata {
            user_id: Some("test_user".to_string()),
            agent_id: None,
            run_id: None,
            actor_id: None,
            role: None,
            memory_type,
            hash: "".to_string(), // 实际应该计算hash
            importance_score: self.rng.gen_range(1.0..=10.0),
            entities: vec![],
            topics: vec![],
            custom: HashMap::new(),
        };
        
        // 计算hash
        metadata.hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
        
        Memory {
            id: memory_id.to_string(),
            content,
            embedding: vec![], // 空向量，实际应该生成嵌入
            metadata,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
    
    /// 生成召回率测试用例
    fn generate_recall_test_case(
        &mut self,
        query_id: &str,
        memories: &HashMap<String, Memory>,
    ) -> RecallTestCase {
        let category = self.sample_query_category();
        let complexity = self.sample_complexity();
        let query = self.generate_query(&category, &complexity);
        
        // 选择相关记忆
        let relevant_memory_ids = self.select_relevant_memories(
            &query,
            memories,
            self.config.avg_relevant_memories as usize,
        );
        
        RecallTestCase {
            query_id: query_id.to_string(),
            query,
            relevant_memory_ids,
            category,
            complexity,
        }
    }
    
    /// 生成有效性测试用例
    fn generate_effectiveness_test_case(&mut self, test_case_id: &str) -> EffectivenessTestCase {
        let category = self.sample_query_category();
        let memory_type = self.sample_memory_type();
        let input_text = self.generate_input_text(&category, &memory_type);
        
        // 生成预期事实
        let expected_facts = self.generate_expected_facts(&input_text);
        
        // 生成重要性评分
        let expected_importance_score = self.rng.gen_range(1..=10);
        
        // 随机决定是否包含重复内容
        let contains_duplicate = self.rng.gen_bool(0.2);
        
        // 随机决定是否需要更新
        let requires_update = self.rng.gen_bool(0.3);
        let existing_memory_id = if requires_update {
            Some(format!("existing_memory_{:04}", self.rng.gen_range(0..100)))
        } else {
            None
        };
        
        EffectivenessTestCase {
            test_case_id: test_case_id.to_string(),
            input_text,
            expected_facts,
            expected_memory_type: memory_type,
            expected_importance_score,
            category,
            contains_duplicate,
            requires_update,
            existing_memory_id,
        }
    }
    
    /// 生成记忆内容
    fn generate_memory_content(&mut self, memory_type: &MemoryType) -> String {
        let base_content = match memory_type {
            MemoryType::Conversational => {
                let topics = ["meeting", "discussion", "chat", "conversation"];
                let topic = topics[self.rng.gen_range(0..topics.len())];
                format!("During our {} yesterday, we talked about project timelines and resource allocation. The team agreed to prioritize the backend API development.", topic)
            }
            MemoryType::Procedural => {
                let procedures = ["deployment", "testing", "debugging", "building"];
                let procedure = procedures[self.rng.gen_range(0..procedures.len())];
                format!("To perform {}, follow these steps: 1. Check prerequisites 2. Run validation 3. Execute main process 4. Verify results 5. Clean up temporary files.", procedure)
            }
            MemoryType::Factual => {
                let facts = [
                    "The capital of France is Paris.",
                    "Water boils at 100 degrees Celsius at sea level.",
                    "The Earth orbits the Sun once every 365.25 days.",
                    "Python was created by Guido van Rossum.",
                ];
                facts[self.rng.gen_range(0..facts.len())].to_string()
            }
            MemoryType::Semantic => {
                let concepts = ["democracy", "machine learning", "sustainability", "innovation"];
                let concept = concepts[self.rng.gen_range(0..concepts.len())];
                format!("{} refers to a system or approach that enables computers to learn from data and improve their performance on tasks without being explicitly programmed for each specific case.", concept)
            }
            MemoryType::Episodic => {
                let events = ["conference", "workshop", "team building", "product launch"];
                let event = events[self.rng.gen_range(0..events.len())];
                format!("At the {} last month, we presented our new architecture design. The audience asked insightful questions about scalability and security.", event)
            }
            MemoryType::Personal => {
                let preferences = ["coffee", "tea", "morning meetings", "agile methodology"];
                let preference = preferences[self.rng.gen_range(0..preferences.len())];
                format!("I prefer {} in the morning as it helps me focus better on complex tasks. This has been consistent for the past few years.", preference)
            }
        };
        
        // 添加一些随机变化
        let variations = [
            " This is important for future reference.",
            " We should consider this in our planning.",
            " This information was verified by multiple sources.",
            " Additional details may be needed for implementation.",
        ];
        
        let variation = variations[self.rng.gen_range(0..variations.len())];
        format!("{}{}", base_content, variation)
    }
    
    /// 生成查询
    fn generate_query(&mut self, category: &str, complexity: &str) -> String {
        let base_query = match category {
            "technology" => match complexity {
                "simple" => "How to deploy application?",
                "medium" => "What are the best practices for API design in microservices?",
                "complex" => "How can we implement zero-downtime deployment with Kubernetes and Istio while maintaining data consistency?",
                _ => "Technology question",
            },
            "science" => match complexity {
                "simple" => "What is machine learning?",
                "medium" => "How does gradient descent optimization work in neural networks?",
                "complex" => "What are the implications of quantum entanglement for secure communication protocols?",
                _ => "Science question",
            },
            "business" => match complexity {
                "simple" => "What is ROI?",
                "medium" => "How to calculate customer lifetime value for SaaS businesses?",
                "complex" => "What strategies can be employed to optimize supply chain resilience while minimizing operational costs in global markets?",
                _ => "Business question",
            },
            "health" => match complexity {
                "simple" => "What is BMI?",
                "medium" => "How does intermittent fasting affect metabolic health?",
                "complex" => "What are the long-term implications of CRISPR gene editing on hereditary disease prevention and ethical considerations?",
                _ => "Health question",
            },
            _ => "General question",
        };
        
        base_query.to_string()
    }
    
    /// 生成输入文本
    fn generate_input_text(&mut self, category: &str, memory_type: &MemoryType) -> String {
        let base_text = match (category, memory_type) {
            ("technology", MemoryType::Procedural) => {
                "To deploy the application, first ensure all tests pass, then build the Docker image, push it to the registry, and update the Kubernetes deployment. Monitor the rollout status and verify health checks."
            }
            ("science", MemoryType::Factual) => {
                "Photosynthesis is the process by which plants convert light energy into chemical energy, producing oxygen as a byproduct. This occurs in chloroplasts and requires water and carbon dioxide."
            }
            ("business", MemoryType::Conversational) => {
                "In our quarterly review meeting, we discussed the declining user engagement metrics. The marketing team suggested A/B testing new onboarding flows, while engineering proposed performance optimizations."
            }
            ("health", MemoryType::Personal) => {
                "I've been tracking my sleep patterns and noticed I sleep better when I avoid screens an hour before bed and maintain a consistent sleep schedule, even on weekends."
            }
            _ => {
                "This is a sample text for testing memory system capabilities. It contains multiple pieces of information that should be extracted and processed appropriately."
            }
        };
        
        base_text.to_string()
    }
    
    /// 生成预期事实
    fn generate_expected_facts(&mut self, input_text: &str) -> Vec<String> {
        // 简化的事实提取：将文本分成句子
        input_text
            .split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .take(3) // 最多取3个事实
            .collect()
    }
    
    /// 选择相关记忆
    fn select_relevant_memories(
        &mut self,
        _query: &str,
        memories: &HashMap<String, Memory>,
        count: usize,
    ) -> Vec<String> {
        let memory_ids: Vec<String> = memories.keys().cloned().collect();
        
        if memory_ids.is_empty() || count == 0 {
            return Vec::new();
        }
        
        // 简化：随机选择一些记忆作为相关记忆
        // 在实际实现中，应该基于语义相似度选择
        let mut selected = Vec::new();
        let mut attempts = 0;
        let max_attempts = count * 3;
        
        while selected.len() < count && attempts < max_attempts {
            let idx = self.rng.gen_range(0..memory_ids.len());
            let memory_id = &memory_ids[idx];
            
            if !selected.contains(memory_id) {
                selected.push(memory_id.clone());
            }
            
            attempts += 1;
        }
        
        selected
    }
    
    /// 采样记忆类型
    fn sample_memory_type(&mut self) -> MemoryType {
        let rand_val = self.rng.gen_range(0.0..1.0);
        let mut cumulative = 0.0;
        
        for (memory_type, probability) in &self.config.memory_type_distribution {
            cumulative += probability;
            if rand_val <= cumulative {
                return memory_type.clone();
            }
        }
        
        // 默认返回对话型记忆
        MemoryType::Conversational
    }
    
    /// 采样查询类别
    fn sample_query_category(&mut self) -> String {
        let idx = self.rng.gen_range(0..self.config.query_categories.len());
        self.config.query_categories[idx].clone()
    }
    
    /// 采样复杂度
    fn sample_complexity(&mut self) -> String {
        let rand_val = self.rng.gen_range(0.0..1.0);
        let mut cumulative = 0.0;
        
        for (complexity, probability) in &self.config.complexity_distribution {
            cumulative += probability;
            if rand_val <= cumulative {
                return complexity.clone();
            }
        }
        
        "medium".to_string()
    }
    
    /// 保存数据集到文件
    pub fn save_dataset<T: serde::Serialize>(
        &self,
        dataset: &T,
        output_path: &str,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(dataset)
            .context("序列化数据集失败")?;
        
        // 确保目录存在
        if let Some(parent) = std::path::Path::new(output_path).parent() {
            fs::create_dir_all(parent)
                .context(format!("创建目录失败: {:?}", parent))?;
        }
        
        fs::write(output_path, json)
            .context(format!("写入数据集文件失败: {}", output_path))?;
        
        info!("数据集已保存到: {}", output_path);
        Ok(())
    }
}

/// 生成测试数据集（公共接口）
pub async fn generate_test_dataset(
    dataset_type: &str,
    output_dir: &std::path::Path,
    size: usize,
) -> Result<()> {
    let mut config = GeneratorConfig::default();
    config.dataset_size = size;
    
    let mut generator = DatasetGenerator::new(config);
    
    match dataset_type.to_lowercase().as_str() {
        "recall" => {
            let dataset = generator.generate_recall_dataset()?;
            let output_path = output_dir.join("test_cases/recall_test_cases.json");
            generator.save_dataset(&dataset, output_path.to_str().unwrap())?;
        }
        "effectiveness" => {
            let dataset = generator.generate_effectiveness_dataset()?;
            let output_path = output_dir.join("test_cases/effectiveness_test_cases.json");
            generator.save_dataset(&dataset, output_path.to_str().unwrap())?;
        }
        "all" => {
            // 生成召回率数据集
            let recall_dataset = generator.generate_recall_dataset()?;
            let recall_path = output_dir.join("test_cases/recall_test_cases.json");
            generator.save_dataset(&recall_dataset, recall_path.to_str().unwrap())?;
            
            // 生成有效性数据集
            let effectiveness_dataset = generator.generate_effectiveness_dataset()?;
            let effectiveness_path = output_dir.join("test_cases/effectiveness_test_cases.json");
            generator.save_dataset(&effectiveness_dataset, effectiveness_path.to_str().unwrap())?;
        }
        _ => {
            anyhow::bail!("未知的数据集类型: {}", dataset_type);
        }
    }
    
    Ok(())
}

/// 验证数据集
pub async fn validate_dataset(
    dataset_path: &std::path::Path,
    dataset_type: &str,
) -> Result<()> {
    let content = fs::read_to_string(dataset_path)
        .context(format!("读取数据集文件失败: {:?}", dataset_path))?;
    
    match dataset_type.to_lowercase().as_str() {
        "recall" => {
            let dataset: RecallTestDataset = serde_json::from_str(&content)
                .context("解析召回率数据集失败")?;
            info!("召回率数据集验证通过: {}个测试用例, {}个记忆",
                dataset.test_cases.len(), dataset.memories.len());
        }
        "effectiveness" => {
            let dataset: EffectivenessTestDataset = serde_json::from_str(&content)
                .context("解析有效性数据集失败")?;
            info!("有效性数据集验证通过: {}个测试用例, {}个现有记忆",
                dataset.test_cases.len(), dataset.existing_memories.len());
        }
        _ => {
            anyhow::bail!("未知的数据集类型: {}", dataset_type);
        }
    }
    
    Ok(())
}