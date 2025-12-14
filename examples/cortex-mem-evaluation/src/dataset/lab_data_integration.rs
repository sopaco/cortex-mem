//! 实验室数据集成模块
//! 
//! 集成实验室真实数据，创建丰富多样的测试数据集

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tracing::{info, warn, debug};

use super::types::*;
use super::types::RecallTestCase;

/// 实验室数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabDataSource {
    /// 数据源名称
    pub name: String,
    /// 数据文件路径
    pub path: String,
    /// 数据格式：json, csv, txt
    pub format: String,
    /// 数据领域：conversation, technical, business, medical, etc.
    pub domain: String,
    /// 数据质量评分（1-10）
    pub quality_score: u8,
    /// 是否包含标注信息
    pub has_annotations: bool,
}

/// 实验室数据集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabDataset {
    /// 数据集名称
    pub name: String,
    /// 数据源列表
    pub sources: Vec<LabDataSource>,
    /// 总样本数
    pub total_samples: usize,
    /// 领域分布
    pub domain_distribution: HashMap<String, usize>,
    /// 平均文本长度
    pub avg_text_length: f64,
    /// 数据质量指标
    pub quality_metrics: QualityMetrics,
}

/// 数据质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// 完整性（0-1）
    pub completeness: f64,
    /// 一致性（0-1）
    pub consistency: f64,
    /// 准确性（0-1）
    pub accuracy: f64,
    /// 多样性（0-1）
    pub diversity: f64,
    /// 相关性（0-1）
    pub relevance: f64,
}

/// 实验室数据集成器
pub struct LabDataIntegrator {
    /// 实验室数据集
    pub datasets: Vec<LabDataset>,
    /// 数据缓存
    pub data_cache: HashMap<String, Vec<LabDataSample>>,
}

/// 实验室数据样本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabDataSample {
    /// 样本ID
    pub id: String,
    /// 原始文本
    pub text: String,
    /// 领域
    pub domain: String,
    /// 子领域
    pub subdomain: Option<String>,
    /// 关键词列表
    pub keywords: Vec<String>,
    /// 实体列表
    pub entities: Vec<String>,
    /// 情感倾向（-1到1）
    pub sentiment: Option<f32>,
    /// 复杂度评分（1-10）
    pub complexity: u8,
    /// 标注信息（如果有）
    pub annotations: Option<Annotations>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 标注信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotations {
    /// 事实列表
    pub facts: Vec<String>,
    /// 主题列表
    pub topics: Vec<String>,
    /// 意图分类
    pub intent: Option<String>,
    /// 情感标签
    pub sentiment_label: Option<String>,
    /// 关系标注
    pub relations: Vec<Relation>,
}

/// 关系标注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
}

impl LabDataIntegrator {
    /// 创建新的实验室数据集成器
    pub fn new() -> Self {
        Self {
            datasets: Vec::new(),
            data_cache: HashMap::new(),
        }
    }
    
    /// 加载实验室数据集配置
    pub fn load_datasets(&mut self, config_path: &str) -> Result<()> {
        let content = fs::read_to_string(config_path)
            .context(format!("读取数据集配置文件失败: {}", config_path))?;
        
        let datasets: Vec<LabDataset> = serde_json::from_str(&content)
            .context("解析数据集配置JSON失败")?;
        
        self.datasets = datasets;
        info!("加载了 {} 个实验室数据集", self.datasets.len());
        
        Ok(())
    }
    
    /// 从实验室数据生成召回率测试数据集
    pub async fn generate_recall_dataset_from_lab(
        &mut self,
        dataset_name: &str,
        num_queries: usize,
        avg_relevant_per_query: usize,
    ) -> Result<RecallTestDataset> {
        info!("从实验室数据生成召回率测试数据集: {}", dataset_name);
        
        // 1. 加载或生成实验室数据样本
        let samples = self.load_or_generate_samples(dataset_name).await?;
        
        if samples.is_empty() {
            anyhow::bail!("没有可用的实验室数据样本");
        }
        
        info!("加载了 {} 个实验室数据样本", samples.len());
        
        // 2. 创建记忆库（从样本中提取）
        let memories = self.create_memories_from_samples(&samples, samples.len() / 2)?;
        
        // 3. 生成查询和相关性标注
        let test_cases = self.generate_test_cases_with_semantic_relations(
            &samples,
            &memories,
            num_queries,
            avg_relevant_per_query,
        )?;
        
        // 4. 创建数据集元数据
        let metadata = DatasetMetadata {
            name: format!("lab_recall_dataset_{}", dataset_name),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            total_test_cases: test_cases.len(),
            total_memories: memories.len(),
            avg_relevant_memories: avg_relevant_per_query as f64,
        };
        
        let dataset = RecallTestDataset {
            test_cases,
            memories,
            metadata,
        };
        
        info!("实验室召回率数据集生成完成: {}个测试用例, {}个记忆",
            dataset.test_cases.len(), dataset.memories.len());
        
        Ok(dataset)
    }
    
    /// 从实验室数据生成有效性测试数据集
    pub async fn generate_effectiveness_dataset_from_lab(
        &mut self,
        dataset_name: &str,
        num_cases: usize,
    ) -> Result<EffectivenessTestDataset> {
        info!("从实验室数据生成有效性测试数据集: {}", dataset_name);
        
        // 1. 加载实验室数据样本
        let samples = self.load_or_generate_samples(dataset_name).await?;
        
        if samples.is_empty() {
            anyhow::bail!("没有可用的实验室数据样本");
        }
        
        // 2. 生成测试用例
        let test_cases = self.generate_effectiveness_test_cases(&samples, num_cases)?;
        
        // 3. 创建现有记忆库
        let existing_memories = self.create_existing_memories(&samples, num_cases / 3)?;
        
        // 4. 创建数据集元数据
        let metadata = DatasetMetadata {
            name: format!("lab_effectiveness_dataset_{}", dataset_name),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
            total_test_cases: test_cases.len(),
            total_memories: existing_memories.len(),
            avg_relevant_memories: 0.0,
        };
        
        let dataset = EffectivenessTestDataset {
            test_cases,
            existing_memories,
            metadata,
        };
        
        info!("实验室有效性数据集生成完成: {}个测试用例, {}个现有记忆",
            dataset.test_cases.len(), dataset.existing_memories.len());
        
        Ok(dataset)
    }
    
    /// 加载或生成实验室数据样本
    async fn load_or_generate_samples(&mut self, dataset_name: &str) -> Result<Vec<LabDataSample>> {
        // 检查缓存
        if let Some(cached) = self.data_cache.get(dataset_name) {
            info!("使用缓存的实验室数据样本: {}", dataset_name);
            return Ok(cached.clone());
        }
        
        // 查找数据集配置
        let dataset_config = self.datasets.iter()
            .find(|d| d.name == dataset_name)
            .context(format!("未找到数据集配置: {}", dataset_name))?;
        
        let mut all_samples = Vec::new();
        
        // 从每个数据源加载数据
        for source in &dataset_config.sources {
            info!("从数据源加载数据: {} ({})", source.name, source.format);
            
            let samples = match source.format.as_str() {
                "json" => self.load_json_samples(&source.path, &source.domain).await?,
                "csv" => self.load_csv_samples(&source.path, &source.domain).await?,
                "txt" => self.load_text_samples(&source.path, &source.domain).await?,
                _ => {
                    warn!("不支持的数据格式: {}, 跳过", source.format);
                    continue;
                }
            };
            
            info!("从 {} 加载了 {} 个样本", source.name, samples.len());
            all_samples.extend(samples);
        }
        
        // 缓存数据
        self.data_cache.insert(dataset_name.to_string(), all_samples.clone());
        
        Ok(all_samples)
    }
    
    /// 从JSON文件加载样本
    async fn load_json_samples(&self, path: &str, domain: &str) -> Result<Vec<LabDataSample>> {
        let content = fs::read_to_string(path)
            .context(format!("读取JSON文件失败: {}", path))?;
        
        // 尝试解析为LabDataSample数组
        if let Ok(samples) = serde_json::from_str::<Vec<LabDataSample>>(&content) {
            return Ok(samples);
        }
        
        // 如果失败，尝试通用JSON格式
        let json_value: serde_json::Value = serde_json::from_str(&content)
            .context("解析JSON失败")?;
        
        let mut samples = Vec::new();
        
        // 处理不同的JSON结构
        match json_value {
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        let sample = self.create_sample_from_json(item, i, text, domain);
                        samples.push(sample);
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    let sample = self.create_sample_from_json(&serde_json::Value::Object(obj.clone()), 0, text, domain);
                    samples.push(sample);
                }
            }
            _ => {
                warn!("不支持的JSON格式: {}", path);
            }
        }
        
        Ok(samples)
    }
    
    /// 从JSON值创建样本
    fn create_sample_from_json(
        &self,
        json_value: &serde_json::Value,
        index: usize,
        text: &str,
        domain: &str,
    ) -> LabDataSample {
        let mut metadata = HashMap::new();
        
        // 提取可能的字段
        if let serde_json::Value::Object(obj) = json_value {
            for (key, value) in obj {
                if key != "text" {
                    metadata.insert(key.clone(), value.clone());
                }
            }
        }
        
        // 提取关键词（如果存在）
        let keywords = metadata.get("keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| self.extract_keywords_from_text(text));
        
        // 提取实体（如果存在）
        let entities = metadata.get("entities")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(Vec::new);
        
        // 计算复杂度
        let complexity = self.calculate_complexity(text);
        
        LabDataSample {
            id: format!("lab_sample_{:06}", index),
            text: text.to_string(),
            domain: domain.to_string(),
            subdomain: None,
            keywords,
            entities,
            sentiment: None,
            complexity,
            annotations: None,
            metadata,
        }
    }
    
    /// 从CSV文件加载样本
    async fn load_csv_samples(&self, path: &str, domain: &str) -> Result<Vec<LabDataSample>> {
        let mut rdr = csv::Reader::from_path(path)
            .context(format!("打开CSV文件失败: {}", path))?;
        
        let mut samples = Vec::new();
        
        for (i, result) in rdr.records().enumerate() {
            let record = result.context("读取CSV记录失败")?;
            
            // 假设第一列是文本
            if let Some(text) = record.get(0) {
                let sample = LabDataSample {
                    id: format!("csv_sample_{:06}", i),
                    text: text.to_string(),
                    domain: domain.to_string(),
                    subdomain: None,
                    keywords: self.extract_keywords_from_text(text),
                    entities: Vec::new(),
                    sentiment: None,
                    complexity: self.calculate_complexity(text),
                    annotations: None,
                    metadata: HashMap::new(),
                };
                samples.push(sample);
            }
        }
        
        Ok(samples)
    }
    
    /// 从文本文件加载样本
    async fn load_text_samples(&self, path: &str, domain: &str) -> Result<Vec<LabDataSample>> {
        let content = fs::read_to_string(path)
            .context(format!("读取文本文件失败: {}", path))?;
        
        // 按段落分割
        let paragraphs: Vec<&str> = content.split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();
        
        let mut samples = Vec::new();
        
        for (i, paragraph) in paragraphs.iter().enumerate() {
            let text = paragraph.trim();
            if text.len() > 10 { // 忽略太短的段落
                let sample = LabDataSample {
                    id: format!("txt_sample_{:06}", i),
                    text: text.to_string(),
                    domain: domain.to_string(),
                    subdomain: None,
                    keywords: self.extract_keywords_from_text(text),
                    entities: Vec::new(),
                    sentiment: None,
                    complexity: self.calculate_complexity(text),
                    annotations: None,
                    metadata: HashMap::new(),
                };
                samples.push(sample);
            }
        }
        
        Ok(samples)
    }
    
    /// 从样本创建记忆
    fn create_memories_from_samples(
        &self,
        samples: &[LabDataSample],
        num_memories: usize,
    ) -> Result<HashMap<String, cortex_mem_core::Memory>> {
        use cortex_mem_core::{Memory, MemoryMetadata, MemoryType};
        use std::collections::HashMap as StdHashMap;
        
        let mut memories = StdHashMap::new();
        
        // 选择样本创建记忆
        let selected_samples: Vec<&LabDataSample> = samples
            .iter()
            .take(num_memories.min(samples.len()))
            .collect();
        
        for (i, sample) in selected_samples.iter().enumerate() {
            let memory_id = format!("lab_memory_{:06}", i);
            
            // 确定记忆类型
            let memory_type = self.determine_memory_type(&sample.domain, &sample.text);
            
            let metadata = MemoryMetadata {
                user_id: Some("lab_user".to_string()),
                agent_id: None,
                run_id: None,
                actor_id: None,
                role: None,
                memory_type,
                hash: self.calculate_hash(&sample.text),
                importance_score: self.calculate_importance_score(sample),
                entities: sample.entities.clone(),
                topics: sample.keywords.clone(),
                custom: StdHashMap::new(),
            };
            
            let memory = Memory {
                id: memory_id.clone(),
                content: sample.text.clone(),
                embedding: vec![], // 实际使用时需要生成嵌入
                metadata,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            
            memories.insert(memory_id, memory);
        }
        
        Ok(memories)
    }
    
    /// 生成带有语义关联的测试用例
    fn generate_test_cases_with_semantic_relations(
        &self,
        samples: &[LabDataSample],
        memories: &HashMap<String, cortex_mem_core::Memory>,
        num_queries: usize,
        avg_relevant: usize,
    ) -> Result<Vec<RecallTestCase>> {
        let mut test_cases = Vec::new();
        
        // 选择查询样本 - 放宽条件，确保有足够的查询
        let mut query_samples: Vec<&LabDataSample> = samples
            .iter()
            .filter(|s| s.complexity >= 3) // 降低复杂度要求
            .take(num_queries)
            .collect();
        
        if query_samples.is_empty() {
            warn!("没有符合条件的查询样本，使用所有样本作为查询");
            // 如果没有符合条件的样本，使用前num_queries个样本
            query_samples = samples
                .iter()
                .take(num_queries)
                .collect();
        }
        
        info!("选择了 {} 个查询样本", query_samples.len());
        
        for (i, sample) in query_samples.iter().enumerate() {
            let query_id = format!("lab_query_{:06}", i);
            
            // 基于关键词匹配选择相关记忆
            let relevant_memory_ids = self.select_relevant_memories_by_keywords(
                &sample.keywords,
                memories,
                avg_relevant,
            );
            
            info!("查询 {}: 关键词={:?}, 找到相关记忆={:?}", 
                query_id, sample.keywords, relevant_memory_ids);
            
            // 确定查询类别和复杂度
            let category = sample.domain.clone();
            let complexity = match sample.complexity {
                1..=3 => "simple",
                4..=7 => "medium",
                _ => "complex",
            }.to_string();
            
            let test_case = RecallTestCase {
                query_id,
                query: sample.text.clone(),
                relevant_memory_ids,
                category,
                complexity,
            };
            
            test_cases.push(test_case);
        }
        
        info!("生成了 {} 个测试用例", test_cases.len());
        Ok(test_cases)
    }
    
    /// 生成有效性测试用例
    fn generate_effectiveness_test_cases(
        &self,
        samples: &[LabDataSample],
        num_cases: usize,
    ) -> Result<Vec<EffectivenessTestCase>> {
        use cortex_mem_core::MemoryType;
        
        let mut test_cases = Vec::new();
        
        // 选择测试样本
        let test_samples: Vec<&LabDataSample> = samples
            .iter()
            .take(num_cases.min(samples.len()))
            .collect();
        
        for (i, sample) in test_samples.iter().enumerate() {
            let test_case_id = format!("lab_effectiveness_{:06}", i);
            
            // 确定预期记忆类型
            let expected_memory_type = self.determine_memory_type(&sample.domain, &sample.text);
            
            // 生成预期事实（从关键词或标注中提取）
            let expected_facts = if let Some(ann) = &sample.annotations {
                ann.facts.clone()
            } else {
                sample.keywords.iter()
                    .take(3)
                    .map(|kw| format!("包含关键词: {}", kw))
                    .collect()
            };
            
            // 计算预期重要性评分
            let expected_importance_score = self.calculate_importance_score(sample) as u8;
            
            // 随机决定是否包含重复内容（20%概率）
            let contains_duplicate = i % 5 == 0;
            
            // 随机决定是否需要更新（30%概率）
            let requires_update = i % 3 == 0;
            let existing_memory_id = if requires_update {
                Some(format!("existing_memory_{:06}", i))
            } else {
                None
            };
            
            let test_case = EffectivenessTestCase {
                test_case_id,
                input_text: sample.text.clone(),
                expected_facts,
                expected_memory_type,
                expected_importance_score,
                category: sample.domain.clone(),
                contains_duplicate,
                requires_update,
                existing_memory_id,
            };
            
            test_cases.push(test_case);
        }
        
        Ok(test_cases)
    }
    
    /// 创建现有记忆库
    fn create_existing_memories(
        &self,
        samples: &[LabDataSample],
        num_memories: usize,
    ) -> Result<HashMap<String, cortex_mem_core::Memory>> {
        // 使用与召回率数据集相同的方法
        self.create_memories_from_samples(samples, num_memories)
    }
    
    /// 基于关键词选择相关记忆
    fn select_relevant_memories_by_keywords(
        &self,
        query_keywords: &[String],
        memories: &HashMap<String, cortex_mem_core::Memory>,
        target_count: usize,
    ) -> Vec<String> {
        let mut scored_memories: Vec<(String, usize)> = Vec::new();
        
        for (memory_id, memory) in memories {
            // 计算关键词匹配分数
            let mut score = 0;
            for keyword in query_keywords {
                if memory.content.contains(keyword) {
                    score += 1;
                }
                // 检查metadata中的topics
                if memory.metadata.topics.contains(keyword) {
                    score += 2; // topics中的匹配权重更高
                }
            }
            
            if score > 0 {
                scored_memories.push((memory_id.clone(), score));
            }
        }
        
        // 按分数排序
        scored_memories.sort_by(|a, b| b.1.cmp(&a.1));
        
        // 选择前target_count个
        let mut selected: Vec<String> = scored_memories.iter()
            .take(target_count.min(scored_memories.len()))
            .map(|(id, _)| id.clone())
            .collect();
        
        // 如果没有找到匹配的记忆，返回随机记忆作为回退
        if selected.is_empty() && !memories.is_empty() {
            warn!("没有找到关键词匹配的记忆，返回随机记忆作为回退");
            let memory_ids: Vec<String> = memories.keys().cloned().collect();
            let mut rng = rand::thread_rng();
            let count = target_count.min(memory_ids.len());
            
            // 随机选择记忆
            use rand::seq::SliceRandom;
            selected = memory_ids.choose_multiple(&mut rng, count).cloned().collect();
        }
        
        selected
    }
    
    /// 从文本提取关键词
    fn extract_keywords_from_text(&self, text: &str) -> Vec<String> {
        // 简化实现：提取名词性词汇
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut keywords = Vec::new();
        
        // 选择长度适中的单词作为关键词
        for word in words {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
            if clean_word.len() >= 4 && clean_word.len() <= 20 {
                // 简单过滤：排除常见停用词
                let lower_word = clean_word.to_lowercase();
                if !self.is_stop_word(&lower_word) {
                    keywords.push(clean_word.to_string());
                }
            }
        }
        
        // 去重并限制数量
        let mut unique_keywords: Vec<String> = keywords.into_iter().collect();
        unique_keywords.sort();
        unique_keywords.dedup();
        
        unique_keywords.into_iter().take(10).collect()
    }
    
    /// 判断是否为停用词
    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "and", "that", "for", "with", "this", "from", "have", "what",
            "which", "about", "would", "could", "should", "will", "can", "may",
            "might", "must", "shall", "a", "an", "in", "on", "at", "to", "of",
            "by", "as", "is", "are", "was", "were", "be", "been", "being",
        ];
        
        stop_words.contains(&word)
    }
    
    /// 计算文本复杂度
    fn calculate_complexity(&self, text: &str) -> u8 {
        let word_count = text.split_whitespace().count();
        let sentence_count = text.split(|c| c == '.' || c == '!' || c == '?').count();
        let avg_sentence_length = if sentence_count > 0 {
            word_count as f64 / sentence_count as f64
        } else {
            0.0
        };
        
        // 基于句子长度和词汇多样性评分
        let mut score = 1;
        
        if word_count > 50 { score += 2; }
        if word_count > 100 { score += 2; }
        if avg_sentence_length > 15.0 { score += 2; }
        if avg_sentence_length > 25.0 { score += 2; }
        
        // 检查是否有复杂结构
        let complex_indicators = ["however", "although", "despite", "furthermore", "therefore"];
        for indicator in &complex_indicators {
            if text.to_lowercase().contains(indicator) {
                score += 1;
            }
        }
        
        score.min(10) as u8
    }
    
    /// 确定记忆类型
    fn determine_memory_type(&self, domain: &str, text: &str) -> cortex_mem_core::MemoryType {
        use cortex_mem_core::MemoryType;
        
        match domain.to_lowercase().as_str() {
            "conversation" | "chat" | "dialogue" => MemoryType::Conversational,
            "technical" | "procedure" | "tutorial" => MemoryType::Procedural,
            "fact" | "knowledge" | "encyclopedia" => MemoryType::Factual,
            "concept" | "theory" | "semantic" => MemoryType::Semantic,
            "event" | "experience" | "story" => MemoryType::Episodic,
            "personal" | "preference" | "profile" => MemoryType::Personal,
            _ => {
                // 基于内容启发式判断
                if text.contains("how to") || text.contains("step") || text.contains("procedure") {
                    MemoryType::Procedural
                } else if text.contains("I prefer") || text.contains("my favorite") {
                    MemoryType::Personal
                } else if text.contains("event") || text.contains("happened") {
                    MemoryType::Episodic
                } else {
                    MemoryType::Conversational
                }
            }
        }
    }
    
    /// 计算哈希值
    fn calculate_hash(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// 计算重要性评分
    fn calculate_importance_score(&self, sample: &LabDataSample) -> f32 {
        let mut score = 5.0; // 基础分
        
        // 基于复杂度加分
        score += sample.complexity as f32 * 0.3;
        
        // 基于关键词数量加分
        if !sample.keywords.is_empty() {
            score += (sample.keywords.len() as f32).min(5.0) * 0.2;
        }
        
        // 基于文本长度（适中的长度更重要）
        let text_len = sample.text.len();
        if text_len > 50 && text_len < 500 {
            score += 2.0;
        }
        
        // 基于领域重要性
        match sample.domain.as_str() {
            "medical" | "safety" | "security" => score += 3.0,
            "technical" | "business" => score += 2.0,
            "personal" | "preference" => score += 1.0,
            _ => {}
        }
        
        score.min(10.0).max(1.0)
    }
    
    /// 计算数据集质量
    fn calculate_dataset_quality(&self, samples: &[LabDataSample]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let mut total_score = 0.0;
        
        for sample in samples {
            let mut sample_score = 0.0;
            
            // 文本质量
            if sample.text.len() > 10 {
                sample_score += 0.3;
            }
            
            // 关键词质量
            if !sample.keywords.is_empty() {
                sample_score += 0.2;
            }
            
            // 复杂度适中
            if sample.complexity >= 3 && sample.complexity <= 8 {
                sample_score += 0.2;
            }
            
            // 领域明确性
            if !sample.domain.is_empty() {
                sample_score += 0.2;
            }
            
            // 标注信息（如果有）
            if sample.annotations.is_some() {
                sample_score += 0.1;
            }
            
            total_score += sample_score;
        }
        
        total_score / samples.len() as f64
    }
}

/// 创建实验室数据集配置示例
pub fn create_example_lab_config() -> LabDataset {
    LabDataset {
        name: "example_lab_dataset".to_string(),
        sources: vec![
            LabDataSource {
                name: "conversation_samples".to_string(),
                path: "data/lab/conversations.json".to_string(),
                format: "json".to_string(),
                domain: "conversation".to_string(),
                quality_score: 8,
                has_annotations: true,
            },
            LabDataSource {
                name: "technical_docs".to_string(),
                path: "data/lab/technical_docs.csv".to_string(),
                format: "csv".to_string(),
                domain: "technical".to_string(),
                quality_score: 9,
                has_annotations: false,
            },
            LabDataSource {
                name: "business_reports".to_string(),
                path: "data/lab/business_reports.txt".to_string(),
                format: "txt".to_string(),
                domain: "business".to_string(),
                quality_score: 7,
                has_annotations: false,
            },
        ],
        total_samples: 1000,
        domain_distribution: {
            let mut map = HashMap::new();
            map.insert("conversation".to_string(), 400);
            map.insert("technical".to_string(), 300);
            map.insert("business".to_string(), 300);
            map
        },
        avg_text_length: 150.5,
        quality_metrics: QualityMetrics {
            completeness: 0.85,
            consistency: 0.90,
            accuracy: 0.88,
            diversity: 0.75,
            relevance: 0.82,
        },
    }
}

/// 生成实验室数据集的公共接口
pub async fn generate_lab_dataset(
    dataset_type: &str,
    dataset_name: &str,
    output_dir: &std::path::Path,
    size: usize,
) -> Result<()> {
    let mut integrator = LabDataIntegrator::new();
    
    // 创建示例配置（实际使用时应该从文件加载）
    let example_config = create_example_lab_config();
    integrator.datasets.push(example_config);
    
    match dataset_type.to_lowercase().as_str() {
        "recall" => {
            let dataset = integrator.generate_recall_dataset_from_lab(
                dataset_name,
                size,
                3, // avg_relevant_per_query
            ).await?;
            
            let output_path = output_dir.join("test_cases/lab_recall_dataset.json");
            save_dataset_to_file(&dataset, &output_path)?;
        }
        "effectiveness" => {
            let dataset = integrator.generate_effectiveness_dataset_from_lab(
                dataset_name,
                size,
            ).await?;
            
            let output_path = output_dir.join("test_cases/lab_effectiveness_dataset.json");
            save_dataset_to_file(&dataset, &output_path)?;
        }
        _ => {
            anyhow::bail!("未知的数据集类型: {}", dataset_type);
        }
    }
    
    Ok(())
}

/// 保存数据集到文件
fn save_dataset_to_file<T: serde::Serialize>(
    dataset: &T,
    output_path: &std::path::Path,
) -> Result<()> {
    let json = serde_json::to_string_pretty(dataset)
        .context("序列化数据集失败")?;
    
    // 确保目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .context(format!("创建目录失败: {:?}", parent))?;
    }
    
    fs::write(output_path, json)
        .context(format!("写入数据集文件失败: {:?}", output_path))?;
    
    info!("实验室数据集已保存到: {:?}", output_path);
    Ok(())
}