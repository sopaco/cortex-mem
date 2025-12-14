//! 召回率评估器
//! 
//! 评估向量检索的召回率和精确率

use anyhow::{Result, Context};
use cortex_mem_core::{MemoryManager, Memory, MemoryType, ScoredMemory};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{info, debug, warn};

use super::metrics::{RecallMetrics, ThresholdMetrics, QueryResult};

/// 召回率测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallTestCase {
    /// 查询ID
    pub query_id: String,
    /// 查询文本
    pub query: String,
    /// 相关记忆ID列表
    pub relevant_memory_ids: Vec<String>,
    /// 查询类别
    pub category: String,
    /// 查询复杂度：simple, medium, complex
    pub complexity: String,
}

/// 召回率测试数据集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallTestDataset {
    /// 测试用例列表
    pub test_cases: Vec<RecallTestCase>,
    /// 记忆库（ID -> 记忆内容）
    pub memories: HashMap<String, Memory>,
    /// 数据集元数据
    pub metadata: crate::dataset::types::DatasetMetadata,
}

/// 数据集元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    /// 数据集名称
    pub name: String,
    /// 创建时间
    pub created_at: String,
    /// 版本
    pub version: String,
    /// 总测试用例数
    pub total_test_cases: usize,
    /// 总记忆数
    pub total_memories: usize,
    /// 平均相关记忆数
    pub avg_relevant_memories: f64,
}

/// 召回率评估器
pub struct RecallEvaluator {
    /// 评估配置
    config: RecallEvaluationConfig,
}

/// 召回率评估配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallEvaluationConfig {
    /// K值列表
    pub k_values: Vec<usize>,
    /// 相似度阈值列表
    pub similarity_thresholds: Vec<f64>,
    /// 每个查询的最大返回结果数
    pub max_results_per_query: usize,
    /// 是否保存详细结果
    pub save_detailed_results: bool,
    /// 是否使用真实评估器
    pub use_real_evaluator: bool,
    /// 测试用例路径
    pub test_cases_path: String,
}

impl Default for RecallEvaluationConfig {
    fn default() -> Self {
        Self {
            k_values: vec![1, 3, 5, 10],
            similarity_thresholds: vec![0.7, 0.8, 0.9],
            max_results_per_query: 20,
            save_detailed_results: true,
            use_real_evaluator: false,
            test_cases_path: "data/test_cases/recall_test_cases.json".to_string(),
        }
    }
}

impl RecallEvaluator {
    /// 创建新的召回率评估器
    pub fn new(config: RecallEvaluationConfig) -> Self {
        Self { config }
    }
    
    /// 评估记忆管理器的召回率
    pub async fn evaluate(
        &self,
        memory_manager: &MemoryManager,
        dataset: &RecallTestDataset,
    ) -> Result<RecallMetrics> {
        info!("开始召回率评估，共{}个测试用例", dataset.test_cases.len());
        
        let mut query_results = Vec::new();
        let mut metrics_by_threshold = HashMap::new();
        
        // 为每个相似度阈值评估
        for &threshold in &self.config.similarity_thresholds {
            info!("评估相似度阈值: {}", threshold);
            
            let threshold_metrics = self.evaluate_with_threshold(
                memory_manager,
                dataset,
                threshold,
            ).await?;
            
            metrics_by_threshold.insert(threshold.to_string(), threshold_metrics);
        }
        
        // 计算总体指标（使用默认阈值0.8）
        let default_threshold = 0.8;
        let mut precision_at_k = HashMap::new();
        let mut recall_at_k = HashMap::new();
        
        for &k in &self.config.k_values {
            let (precision, recall) = self.calculate_precision_recall_at_k(
                &query_results,
                k,
            );
            precision_at_k.insert(k, precision);
            recall_at_k.insert(k, recall);
        }
        
        // 计算MAP和NDCG
        let mean_average_precision = self.calculate_mean_average_precision(&query_results);
        let normalized_discounted_cumulative_gain = 
            self.calculate_ndcg(&query_results, self.config.k_values.last().copied().unwrap_or(10));
        
        let metrics = RecallMetrics {
            precision_at_k,
            recall_at_k,
            mean_average_precision,
            normalized_discounted_cumulative_gain,
            metrics_by_threshold,
            query_level_results: if self.config.save_detailed_results {
                query_results
            } else {
                Vec::new()
            },
        };
        
        info!("召回率评估完成");
        Ok(metrics)
    }
    
    /// 使用特定阈值评估
    async fn evaluate_with_threshold(
        &self,
        memory_manager: &MemoryManager,
        dataset: &RecallTestDataset,
        similarity_threshold: f64,
    ) -> Result<ThresholdMetrics> {
        let mut total_precision = 0.0;
        let mut total_recall = 0.0;
        let mut total_results_returned = 0;
        let mut query_count = 0;
        
        for test_case in &dataset.test_cases {
            // 执行搜索
            let search_results = memory_manager.search(
                &test_case.query,
                &cortex_mem_core::types::Filters::default(),
                self.config.max_results_per_query,
            ).await
            .context("搜索记忆失败")?;
            
            // 过滤低于阈值的結果
            let filtered_results: Vec<&ScoredMemory> = search_results
                .iter()
                .filter(|m| m.score >= similarity_threshold as f32)
                .collect();
            
            // 计算相关记忆ID集合
            let relevant_ids: HashSet<&str> = test_case.relevant_memory_ids
                .iter()
                .map(|id| id.as_str())
                .collect();
            
            // 计算检索到的相关记忆
            let retrieved_relevant = filtered_results
                .iter()
                .filter(|m| relevant_ids.contains(m.memory.id.as_str()))
                .count();
            
            let retrieved_total = filtered_results.len();
            let relevant_total = relevant_ids.len();
            
            // 计算精确率和召回率
            let precision = if retrieved_total > 0 {
                retrieved_relevant as f64 / retrieved_total as f64
            } else {
                0.0
            };
            
            let recall = if relevant_total > 0 {
                retrieved_relevant as f64 / relevant_total as f64
            } else {
                0.0
            };
            
            total_precision += precision;
            total_recall += recall;
            total_results_returned += retrieved_total;
            query_count += 1;
            
            debug!(
                "查询 {}: 精确率={:.3}, 召回率={:.3}, 返回结果={}",
                test_case.query_id, precision, recall, retrieved_total
            );
        }
        
        let avg_precision = total_precision / query_count as f64;
        let avg_recall = total_recall / query_count as f64;
        let avg_results_returned = total_results_returned as f64 / query_count as f64;
        let f1_score = if avg_precision + avg_recall > 0.0 {
            2.0 * avg_precision * avg_recall / (avg_precision + avg_recall)
        } else {
            0.0
        };
        
        Ok(ThresholdMetrics {
            threshold: similarity_threshold,
            precision: avg_precision,
            recall: avg_recall,
            f1_score,
            avg_results_returned,
            success_rate: None, // 暂时设为None
            avg_latency_ms: None, // 暂时设为None
        })
    }
    
    /// 计算精确率和召回率@K
    fn calculate_precision_recall_at_k(
        &self,
        query_results: &[QueryResult],
        k: usize,
    ) -> (f64, f64) {
        let mut total_precision = 0.0;
        let mut total_recall = 0.0;
        let mut count = 0;
        
        // 注意：这里需要实际的检索结果来计算P@K和R@K
        // 由于我们目前没有保存每个查询的检索结果排序，这里使用近似计算
        // 在实际实现中，需要修改以保存完整的检索结果
        
        for result in query_results {
            // 近似计算：假设前k个结果中的相关比例
            let _max_relevant_at_k = result.relevant_memories.min(k);
            let expected_relevant_at_k = if result.retrieved_total > 0 {
                (result.retrieved_relevant as f64 * k as f64 / 
                 result.retrieved_total as f64).round() as usize
            } else {
                0
            };
            
            let precision_at_k = if k > 0 {
                expected_relevant_at_k as f64 / k as f64
            } else {
                0.0
            };
            
            let recall_at_k = if result.relevant_memories > 0 {
                expected_relevant_at_k as f64 / result.relevant_memories as f64
            } else {
                0.0
            };
            
            total_precision += precision_at_k;
            total_recall += recall_at_k;
            count += 1;
        }
        
        let avg_precision = if count > 0 { total_precision / count as f64 } else { 0.0 };
        let avg_recall = if count > 0 { total_recall / count as f64 } else { 0.0 };
        
        (avg_precision, avg_recall)
    }
    
    /// 计算平均精确率均值（MAP）
    fn calculate_mean_average_precision(&self, query_results: &[QueryResult]) -> f64 {
        let mut total_ap = 0.0;
        let mut count = 0;
        
        for result in query_results {
            // 使用查询结果中的平均精确率
            total_ap += result.average_precision;
            count += 1;
        }
        
        if count > 0 {
            total_ap / count as f64
        } else {
            0.0
        }
    }
    
    /// 计算归一化折损累计增益（NDCG）
    fn calculate_ndcg(&self, query_results: &[QueryResult], _k: usize) -> f64 {
        let mut total_ndcg = 0.0;
        let mut count = 0;
        
        for result in query_results {
            // 简化计算：使用精确率作为相关性分数
            let dcg = result.precision; // 简化：使用精确率作为第一个结果的增益
            
            // 理想DCG：所有相关结果都在前面
            let ideal_dcg = if result.relevant_memories > 0 {
                1.0 // 简化：理想情况下第一个结果就是相关的
            } else {
                0.0
            };
            
            let ndcg = if ideal_dcg > 0.0 {
                dcg / ideal_dcg
            } else {
                0.0
            };
            
            total_ndcg += ndcg;
            count += 1;
        }
        
        if count > 0 {
            total_ndcg / count as f64
        } else {
            0.0
        }
    }
    
    /// 加载测试数据集
    pub fn load_dataset(path: &str) -> Result<RecallTestDataset> {
        let content = std::fs::read_to_string(path)
            .context(format!("读取数据集文件失败: {}", path))?;
        
        let dataset: RecallTestDataset = serde_json::from_str(&content)
            .context("解析数据集JSON失败")?;
        
        info!("加载召回率测试数据集: {}个测试用例, {}个记忆",
            dataset.test_cases.len(), dataset.memories.len());
        
        Ok(dataset)
    }
    
    /// 保存评估结果
    pub fn save_results(&self, metrics: &RecallMetrics, output_path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(metrics)
            .context("序列化评估结果失败")?;
        
        std::fs::write(output_path, json)
            .context(format!("写入评估结果文件失败: {}", output_path))?;
        
        info!("评估结果已保存到: {}", output_path);
        Ok(())
    }
}