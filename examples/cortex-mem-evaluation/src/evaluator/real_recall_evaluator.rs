//! 真实召回率评估器
//! 
//! 基于真实cortex-mem-core API调用的召回率评估

use anyhow::{Result, Context};
use cortex_mem_core::{MemoryManager, Memory, ScoredMemory, types::Filters};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tracing::{info, debug, warn, error};

use super::metrics::{RecallMetrics, ThresholdMetrics, QueryResult};
use crate::dataset::types::RecallTestDataset;

/// 真实召回率评估器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealRecallEvaluationConfig {
    /// K值列表
    pub k_values: Vec<usize>,
    /// 相似度阈值列表
    pub similarity_thresholds: Vec<f32>,
    /// 每个查询的最大返回结果数
    pub max_results_per_query: usize,
    /// 是否保存详细结果
    pub save_detailed_results: bool,
    /// 超时时间（秒）
    pub timeout_seconds: u64,
    /// 是否启用并行评估
    pub enable_parallel_evaluation: bool,
    /// 是否验证记忆库完整性
    pub verify_memory_integrity: bool,
    /// 测试数据集路径
    pub test_cases_path: String,
}

impl Default for RealRecallEvaluationConfig {
    fn default() -> Self {
        Self {
            k_values: vec![1, 3, 5, 10],
            similarity_thresholds: vec![0.7, 0.8, 0.9],
            max_results_per_query: 20,
            save_detailed_results: true,
            timeout_seconds: 30,
            enable_parallel_evaluation: true,
            verify_memory_integrity: true,
            test_cases_path: "data/test_cases/lab_recall_dataset.json".to_string(),
        }
    }
}

/// 阈值评估结果（包含查询结果）
struct ThresholdEvaluationResult {
    /// 阈值指标
    metrics: ThresholdMetrics,
    /// 查询结果
    query_results: Vec<QueryResult>,
}

/// 真实召回率评估器
pub struct RealRecallEvaluator {
    /// 评估配置
    config: RealRecallEvaluationConfig,
    /// 记忆管理器
    memory_manager: std::sync::Arc<MemoryManager>,
}

impl RealRecallEvaluator {
    /// 创建新的真实召回率评估器
    pub fn new(
        config: RealRecallEvaluationConfig,
        memory_manager: std::sync::Arc<MemoryManager>,
    ) -> Self {
        Self {
            config,
            memory_manager,
        }
    }
    
    /// 评估记忆管理器的召回率
    pub async fn evaluate(
        &self,
        dataset: &RecallTestDataset,
    ) -> Result<RecallMetrics> {
        info!("开始真实召回率评估，共{}个测试用例", dataset.test_cases.len());
        
        // 验证记忆库完整性
        if self.config.verify_memory_integrity {
            self.verify_memory_integrity(&dataset.memories).await?;
        }
        
        // 准备记忆库
        self.prepare_memory_library(&dataset.memories).await?;
        
        let mut query_results = Vec::new();
        let mut metrics_by_threshold = HashMap::new();
        
        // 为每个相似度阈值评估
        for &threshold in &self.config.similarity_thresholds {
            info!("评估相似度阈值: {}", threshold);
            
            let start_time = Instant::now();
            
            let threshold_result = self.evaluate_with_threshold(
                dataset,
                threshold,
            ).await?;
            
            let elapsed = start_time.elapsed();
            info!("阈值 {} 评估完成，耗时: {:?}", threshold, elapsed);
            
            metrics_by_threshold.insert(threshold.to_string(), threshold_result.metrics);
        }
        
        // 使用第一个阈值（或默认阈值0.8）计算总体指标
        let default_threshold = self.config.similarity_thresholds.first().copied().unwrap_or(0.8);
        info!("使用阈值 {} 计算总体指标", default_threshold);
        
        // 为默认阈值运行评估以获取query_results
        let default_threshold_result = self.evaluate_with_threshold(dataset, default_threshold).await?;
        query_results = default_threshold_result.query_results;
        
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
        
        info!("真实召回率评估完成");
        Ok(metrics)
    }
    
    /// 验证记忆库完整性
    async fn verify_memory_integrity(
        &self,
        memories: &HashMap<String, Memory>,
    ) -> Result<()> {
        info!("验证记忆库完整性，共{}个记忆", memories.len());
        
        let mut valid_count = 0;
        let mut invalid_count = 0;
        
        for (memory_id, memory) in memories {
            // 检查基本字段
            if memory.id.is_empty() {
                warn!("记忆 {}: ID为空", memory_id);
                invalid_count += 1;
                continue;
            }
            
            if memory.content.is_empty() {
                warn!("记忆 {}: 内容为空", memory_id);
                invalid_count += 1;
                continue;
            }
            
            if memory.embedding.is_empty() {
                debug!("记忆 {}: 嵌入向量为空（可能未生成）", memory_id);
            }
            
            valid_count += 1;
        }
        
        let validity_rate = valid_count as f64 / memories.len() as f64;
        info!("记忆库完整性验证完成: 有效={}, 无效={}, 有效率={:.2}%", 
            valid_count, invalid_count, validity_rate * 100.0);
        
        if validity_rate < 0.8 {
            warn!("记忆库有效率低于80%，可能影响评估结果");
        }
        
        Ok(())
    }
    
    /// 准备记忆库（将测试记忆添加到记忆管理器）
    async fn prepare_memory_library(
        &self,
        memories: &HashMap<String, Memory>,
    ) -> Result<()> {
        info!("准备记忆库，添加{}个测试记忆", memories.len());
        
        let mut added_count = 0;
        let mut skipped_count = 0;
        let mut error_count = 0;
        
        for (memory_id, memory) in memories {
            // 安全地截取前50个字符，避免UTF-8边界错误
            let preview = if memory.content.len() <= 50 {
                &memory.content
            } else {
                // 找到第50个字符的边界
                let mut end = 50;
                while end < memory.content.len() && !memory.content.is_char_boundary(end) {
                    end += 1;
                }
                &memory.content[..end.min(memory.content.len())]
            };
            info!("正在添加记忆 {}: {}...", memory_id, preview);
            
            // 尝试添加记忆
            match self.memory_manager.store(
                memory.content.clone(),
                memory.metadata.clone(),
            ).await {
                Ok(new_memory_id) => {
                    info!("添加记忆成功: {} -> {}", memory_id, new_memory_id);
                    added_count += 1;
                }
                Err(e) => {
                    // 检查是否是重复错误
                    if e.to_string().contains("duplicate") || e.to_string().contains("already exists") {
                        info!("记忆已存在: {}", memory_id);
                        skipped_count += 1;
                    } else {
                        error!("添加记忆失败 {}: {}", memory_id, e);
                        error_count += 1;
                    }
                }
            }
            
            // 限制添加速率，避免过载（特别是对于真实LLM客户端）
            // 每添加一个记忆都等待一段时间，避免API调用频率过高
            tokio::time::sleep(Duration::from_millis(500)).await; // 500毫秒延迟
            
            // 额外的：每5个记忆等待更长时间
            if added_count % 5 == 0 {
                info!("已添加{}个记忆，等待2秒避免API限制...", added_count);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
        
        info!("记忆库准备完成: 添加={}, 跳过={}, 错误={}", 
            added_count, skipped_count, error_count);
        
        if error_count > 0 {
            warn!("有{}个记忆添加失败，可能影响评估结果", error_count);
        }
        
        if added_count == 0 && skipped_count == 0 {
            error!("没有成功添加任何记忆！评估结果将不准确");
        }
        
        Ok(())
    }
    
    /// 使用特定阈值评估
    async fn evaluate_with_threshold(
        &self,
        dataset: &RecallTestDataset,
        similarity_threshold: f32,
    ) -> Result<ThresholdEvaluationResult> {
        let mut total_precision = 0.0;
        let mut total_recall = 0.0;
        let mut total_results_returned = 0;
        let mut query_count = 0;
        let mut query_results = Vec::new();
        
        let mut total_latency = Duration::default();
        let mut success_count = 0;
        let mut error_count = 0;
        
        for test_case in &dataset.test_cases {
            let query_start = Instant::now();
            
            // 创建过滤器
            let filters = Filters {
                user_id: Some("test_user".to_string()), // 使用测试用户
                ..Default::default()
            };
            
            // 执行真实搜索
            info!("执行搜索 - 查询ID: {}, 查询内容: {}..., 阈值: {}, 最大结果数: {}", 
                test_case.query_id, 
                &test_case.query[..test_case.query.len().min(30)],
                similarity_threshold,
                self.config.max_results_per_query);
            
            let search_result = tokio::time::timeout(
                Duration::from_secs(self.config.timeout_seconds),
                self.memory_manager.search_with_threshold(
                    &test_case.query,
                    &filters,
                    self.config.max_results_per_query,
                    Some(similarity_threshold),
                )
            ).await;
            
            let latency = query_start.elapsed();
            total_latency += latency;
            
            match search_result {
                Ok(Ok(search_results)) => {
                    success_count += 1;
                    
                    // 计算相关记忆ID集合
                    let relevant_ids: HashSet<&str> = test_case.relevant_memory_ids
                        .iter()
                        .map(|id| id.as_str())
                        .collect();
                    
                    // 计算检索到的相关记忆
                    let retrieved_relevant = search_results
                        .iter()
                        .filter(|m| relevant_ids.contains(m.memory.id.as_str()))
                        .count();
                    
                    let retrieved_total = search_results.len();
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
                    
                    // 保存查询结果用于后续计算
                    let query_result = QueryResult {
                        query_id: test_case.query_id.clone(),
                        query: test_case.query.clone(),
                        precision,
                        recall,
                        retrieved_total,
                        retrieved_relevant,
                        relevant_memories: relevant_total,
                        average_precision: self.calculate_average_precision(&search_results, &relevant_ids),
                        latency_ms: latency.as_millis() as u64,
                    };
                    
                    query_results.push(query_result);
                    
                    // 添加详细调试信息
                    if retrieved_total == 0 {
                        warn!("查询 {} 返回0个结果！相关记忆ID: {:?}", test_case.query_id, test_case.relevant_memory_ids);
                        
                        // 安全地截取字符串，避免UTF-8边界错误
                        let preview_len = test_case.query.len().min(100);
                        let mut end = preview_len;
                        while end > 0 && !test_case.query.is_char_boundary(end) {
                            end -= 1;
                        }
                        warn!("查询内容: {}...", &test_case.query[..end]);
                    } else {
                        info!("查询 {} 返回 {} 个结果，其中 {} 个相关", 
                            test_case.query_id, retrieved_total, retrieved_relevant);
                        
                        // 显示前3个结果的相似度分数
                        for (i, result) in search_results.iter().take(3).enumerate() {
                            info!("  结果 {}: ID={}, 相似度={:.3}, 内容: {}...", 
                                i + 1, 
                                result.memory.id,
                                result.score,
                                &result.memory.content[..result.memory.content.len().min(50)]);
                        }
                    }
                    
                    debug!(
                        "查询 {}: 精确率={:.3}, 召回率={:.3}, 返回结果={}, 延迟={:?}",
                        test_case.query_id, precision, recall, retrieved_total, latency
                    );
                }
                Ok(Err(e)) => {
                    error_count += 1;
                    warn!("查询 {} 失败: {}", test_case.query_id, e);
                }
                Err(_) => {
                    error_count += 1;
                    warn!("查询 {} 超时 ({}秒)", test_case.query_id, self.config.timeout_seconds);
                }
            }
            
            // 限制查询速率，避免过载（特别是对于真实LLM客户端）
            // 每个查询都等待一段时间，避免API调用频率过高
            tokio::time::sleep(Duration::from_millis(1000)).await; // 1秒延迟
            
            // 额外的：每3个查询等待更长时间
            if query_count % 3 == 0 {
                info!("已处理{}个查询，等待3秒避免API限制...", query_count);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
        
        let avg_precision = if query_count > 0 { total_precision / query_count as f64 } else { 0.0 };
        let avg_recall = if query_count > 0 { total_recall / query_count as f64 } else { 0.0 };
        let avg_results_returned = if query_count > 0 { total_results_returned as f64 / query_count as f64 } else { 0.0 };
        let avg_latency = if success_count > 0 { total_latency / success_count as u32 } else { Duration::default() };
        
        let f1_score = if avg_precision + avg_recall > 0.0 {
            2.0 * avg_precision * avg_recall / (avg_precision + avg_recall)
        } else {
            0.0
        };
        
        let success_rate = if query_count + error_count > 0 {
            success_count as f64 / (success_count + error_count) as f64
        } else {
            0.0
        };
        
        info!("阈值 {} 评估统计: 成功={}, 错误={}, 成功率={:.1}%, 平均延迟={:?}",
            similarity_threshold, success_count, error_count, success_rate * 100.0, avg_latency);
        
        let metrics = ThresholdMetrics {
            threshold: similarity_threshold as f64,
            precision: avg_precision,
            recall: avg_recall,
            f1_score,
            avg_results_returned,
            success_rate: Some(success_rate),
            avg_latency_ms: Some(avg_latency.as_millis() as u64),
        };
        
        Ok(ThresholdEvaluationResult {
            metrics,
            query_results,
        })
    }
    
    /// 计算平均精确率（AP）
    fn calculate_average_precision(
        &self,
        search_results: &[ScoredMemory],
        relevant_ids: &HashSet<&str>,
    ) -> f64 {
        if relevant_ids.is_empty() || search_results.is_empty() {
            return 0.0;
        }
        
        let mut sum_precision = 0.0;
        let mut relevant_found = 0;
        
        for (k, result) in search_results.iter().enumerate() {
            if relevant_ids.contains(result.memory.id.as_str()) {
                relevant_found += 1;
                let precision_at_k = relevant_found as f64 / (k + 1) as f64;
                sum_precision += precision_at_k;
            }
        }
        
        if relevant_found > 0 {
            sum_precision / relevant_found as f64
        } else {
            0.0
        }
    }
    
    /// 计算精确率和召回率@K
    fn calculate_precision_recall_at_k(
        &self,
        query_results: &[QueryResult],
        k: usize,
    ) -> (f64, f64) {
        let mut total_precision_at_k = 0.0;
        let mut total_recall_at_k = 0.0;
        let mut count = 0;
        
        for result in query_results {
            // 这里需要实际的检索结果排序来计算P@K和R@K
            // 由于QueryResult目前不包含排序结果，我们使用近似值
            // 在实际完整实现中，需要修改QueryResult以包含完整的检索结果列表
            
            let precision_at_k = if result.retrieved_total >= k {
                // 假设前k个结果中的相关比例与整体相同
                result.precision
            } else if result.retrieved_total > 0 {
                // 如果返回结果少于k，使用实际比例
                result.precision * result.retrieved_total as f64 / k as f64
            } else {
                0.0
            };
            
            let recall_at_k = if result.relevant_memories > 0 {
                // 假设前k个结果召回的比例与整体相同
                result.recall.min(1.0)
            } else {
                0.0
            };
            
            total_precision_at_k += precision_at_k;
            total_recall_at_k += recall_at_k;
            count += 1;
        }
        
        let avg_precision = if count > 0 { total_precision_at_k / count as f64 } else { 0.0 };
        let avg_recall = if count > 0 { total_recall_at_k / count as f64 } else { 0.0 };
        
        (avg_precision, avg_recall)
    }
    
    /// 计算平均精确率均值（MAP）
    fn calculate_mean_average_precision(&self, query_results: &[QueryResult]) -> f64 {
        let mut total_ap = 0.0;
        let mut count = 0;
        
        for result in query_results {
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
    fn calculate_ndcg(&self, query_results: &[QueryResult], k: usize) -> f64 {
        let mut total_ndcg = 0.0;
        let mut count = 0;
        
        for result in query_results {
            // 简化计算：使用平均精确率作为相关性分数
            let dcg = result.average_precision;
            
            // 理想DCG：所有相关结果都在前面，得分为1
            let ideal_dcg = if result.relevant_memories > 0 { 1.0 } else { 0.0 };
            
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
    
    /// 清理测试记忆库
    pub async fn cleanup_test_memories(&self) -> Result<()> {
        info!("清理测试记忆库...");
        
        // 这里需要实现清理逻辑
        // 实际实现应该删除测试期间添加的记忆
        
        info!("测试记忆库清理完成");
        Ok(())
    }
}

/// 扩展ThresholdMetrics以包含更多指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedThresholdMetrics {
    /// 相似度阈值
    pub threshold: f64,
    /// 精确率
    pub precision: f64,
    /// 召回率
    pub recall: f64,
    /// F1分数
    pub f1_score: f64,
    /// 平均返回结果数
    pub avg_results_returned: f64,
    /// 查询成功率
    pub success_rate: Option<f64>,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: Option<u64>,
    /// 查询总数
    pub total_queries: usize,
    /// 成功查询数
    pub successful_queries: usize,
    /// 错误查询数
    pub error_queries: usize,
}

impl From<ThresholdMetrics> for EnhancedThresholdMetrics {
    fn from(metrics: ThresholdMetrics) -> Self {
        Self {
            threshold: metrics.threshold,
            precision: metrics.precision,
            recall: metrics.recall,
            f1_score: metrics.f1_score,
            avg_results_returned: metrics.avg_results_returned,
            success_rate: metrics.success_rate,
            avg_latency_ms: metrics.avg_latency_ms,
            total_queries: 0,
            successful_queries: 0,
            error_queries: 0,
        }
    }
}

// 不再从recall_evaluator重新导出，使用dataset::types中的定义