//! 记忆有效性评估器
//! 
//! 评估记忆提取、分类、去重、重要性评估等核心功能的有效性

use anyhow::{Result, Context};
use cortex_mem_core::{MemoryManager, Memory, MemoryType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, debug, warn};

use super::metrics::{
    EffectivenessMetrics, FactExtractionMetrics, ClassificationMetrics,
    ImportanceMetrics, DeduplicationMetrics, UpdateMetrics,
    FactExtractionResult,
};

/// 有效性测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessTestCase {
    /// 测试用例ID
    pub test_case_id: String,
    /// 输入文本
    pub input_text: String,
    /// 预期提取的关键事实
    pub expected_facts: Vec<String>,
    /// 预期记忆类型
    pub expected_memory_type: MemoryType,
    /// 预期重要性评分（1-10）
    pub expected_importance_score: u8,
    /// 测试类别
    pub category: String,
    /// 是否包含重复内容
    pub contains_duplicate: bool,
    /// 是否需要更新现有记忆
    pub requires_update: bool,
    /// 现有记忆ID（如果需要更新）
    pub existing_memory_id: Option<String>,
}

/// 有效性测试数据集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessTestDataset {
    /// 测试用例列表
    pub test_cases: Vec<EffectivenessTestCase>,
    /// 现有记忆库（用于更新测试）
    pub existing_memories: HashMap<String, Memory>,
    /// 数据集元数据
    pub metadata: crate::dataset::types::DatasetMetadata,
}

/// 有效性评估器
pub struct EffectivenessEvaluator {
    /// 评估配置
    config: EffectivenessEvaluationConfig,
}

/// 有效性评估配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessEvaluationConfig {
    /// 是否验证事实提取
    pub verify_fact_extraction: bool,
    /// 是否验证记忆分类
    pub verify_classification: bool,
    /// 是否验证重要性评估
    pub verify_importance_evaluation: bool,
    /// 是否验证去重效果
    pub verify_deduplication: bool,
    /// 是否验证记忆更新逻辑
    pub verify_memory_update: bool,
    /// 重要性评分容差
    pub importance_score_tolerance: u8,
    /// 是否使用LLM辅助评估
    pub llm_evaluation_enabled: bool,
    /// 测试用例路径
    pub test_cases_path: String,
}

impl Default for EffectivenessEvaluationConfig {
    fn default() -> Self {
        Self {
            verify_fact_extraction: true,
            verify_classification: true,
            verify_importance_evaluation: true,
            verify_deduplication: true,
            verify_memory_update: true,
            importance_score_tolerance: 1,
            llm_evaluation_enabled: false,
            test_cases_path: "data/test_cases/effectiveness_test_cases.json".to_string(),
        }
    }
}

impl EffectivenessEvaluator {
    /// 创建新的有效性评估器
    pub fn new(config: EffectivenessEvaluationConfig) -> Self {
        Self { config }
    }
    
    /// 评估记忆管理器的有效性
    pub async fn evaluate(
        &self,
        memory_manager: &MemoryManager,
        dataset: &EffectivenessTestDataset,
    ) -> Result<EffectivenessMetrics> {
        info!("开始有效性评估，共{}个测试用例", dataset.test_cases.len());
        
        let mut fact_extraction_results = Vec::new();
        let mut classification_results = Vec::new();
        let mut importance_results = Vec::new();
        let mut deduplication_results = Vec::new();
        let mut update_results = Vec::new();
        
        // 首先添加所有现有记忆
        for (memory_id, memory) in &dataset.existing_memories {
            // 这里需要实际添加记忆到内存管理器
            // 由于API限制，我们暂时跳过这一步
            debug!("现有记忆: {} - {}", memory_id, &memory.content[..50.min(memory.content.len())]);
        }
        
        // 评估每个测试用例
        for test_case in &dataset.test_cases {
            debug!("评估测试用例: {}", test_case.test_case_id);
            
            // 事实提取评估
            if self.config.verify_fact_extraction {
                if let Ok(result) = self.evaluate_fact_extraction(
                    memory_manager,
                    test_case,
                ).await {
                    fact_extraction_results.push(result);
                }
            }
            
            // 记忆分类评估
            if self.config.verify_classification {
                if let Ok(result) = self.evaluate_classification(
                    memory_manager,
                    test_case,
                ).await {
                    classification_results.push(result);
                }
            }
            
            // 重要性评估
            if self.config.verify_importance_evaluation {
                if let Ok(result) = self.evaluate_importance(
                    memory_manager,
                    test_case,
                ).await {
                    importance_results.push(result);
                }
            }
            
            // 去重评估（如果包含重复内容）
            if self.config.verify_deduplication && test_case.contains_duplicate {
                if let Ok(result) = self.evaluate_deduplication(
                    memory_manager,
                    test_case,
                ).await {
                    deduplication_results.push(result);
                }
            }
            
            // 记忆更新评估
            if self.config.verify_memory_update && test_case.requires_update {
                if let Ok(result) = self.evaluate_memory_update(
                    memory_manager,
                    test_case,
                    &dataset.existing_memories,
                ).await {
                    update_results.push(result);
                }
            }
        }
        
        // 计算各项指标
        let fact_extraction_metrics = self.calculate_fact_extraction_metrics(&fact_extraction_results);
        let classification_metrics = self.calculate_classification_metrics(&classification_results);
        let importance_metrics = self.calculate_importance_metrics(&importance_results);
        let deduplication_metrics = self.calculate_deduplication_metrics(&deduplication_results);
        let update_metrics = self.calculate_update_metrics(&update_results);
        
        // 计算综合得分
        let overall_score = self.calculate_overall_score(
            &fact_extraction_metrics,
            &classification_metrics,
            &importance_metrics,
            &deduplication_metrics,
            &update_metrics,
        );
        
        let metrics = EffectivenessMetrics {
            fact_extraction_accuracy: fact_extraction_metrics,
            classification_accuracy: classification_metrics,
            importance_evaluation_quality: importance_metrics,
            deduplication_effectiveness: deduplication_metrics,
            memory_update_correctness: update_metrics,
            overall_score,
        };
        
        info!("有效性评估完成，综合得分: {:.2}", overall_score);
        Ok(metrics)
    }
    
    /// 评估事实提取
    async fn evaluate_fact_extraction(
        &self,
        memory_manager: &MemoryManager,
        test_case: &EffectivenessTestCase,
    ) -> Result<FactExtractionResult> {
        // 调用MemoryManager的事实提取功能
        // 注意：cortex-mem-core可能没有直接的事实提取API
        // 这里我们使用搜索功能来近似评估事实提取
        
        info!("评估事实提取 - 测试用例: {}", test_case.test_case_id);
        
        // 尝试从输入文本中提取关键信息
        // 在实际实现中，应该调用memory_manager的提取器功能
        let extracted_facts = Vec::new(); // 暂时为空，需要实际实现
        
        // 计算匹配的事实数量
        let matched_facts = extracted_facts
            .iter()
            .filter(|fact| test_case.expected_facts.contains(fact))
            .count();
        
        let is_perfect_match = matched_facts == test_case.expected_facts.len() &&
            matched_facts == extracted_facts.len();
        
        Ok(FactExtractionResult {
            input_text: test_case.input_text.clone(),
            extracted_facts,
            ground_truth_facts: test_case.expected_facts.clone(),
            matched_facts,
            is_perfect_match,
        })
    }
    
    /// 评估记忆分类
    async fn evaluate_classification(
        &self,
        memory_manager: &MemoryManager,
        test_case: &EffectivenessTestCase,
    ) -> Result<ClassificationResult> {
        // 调用MemoryManager的分类功能
        info!("评估记忆分类 - 测试用例: {}", test_case.test_case_id);
        
        // 在实际实现中，应该调用memory_manager的分类器
        // 这里我们暂时使用默认类型，需要实际实现
        let predicted_type = MemoryType::Conversational; // 默认类型，需要实际实现
        
        let is_correct = predicted_type == test_case.expected_memory_type;
        
        Ok(ClassificationResult {
            test_case_id: test_case.test_case_id.clone(),
            input_text: test_case.input_text.clone(),
            predicted_type,
            expected_type: test_case.expected_memory_type.clone(),
            is_correct,
        })
    }
    
    /// 评估重要性
    async fn evaluate_importance(
        &self,
        memory_manager: &MemoryManager,
        test_case: &EffectivenessTestCase,
    ) -> Result<ImportanceResult> {
        // 调用MemoryManager的重要性评估功能
        info!("评估重要性 - 测试用例: {}", test_case.test_case_id);
        
        // 在实际实现中，应该调用memory_manager的重要性评估器
        // 这里我们暂时使用默认值，需要实际实现
        let predicted_score = 5; // 默认中等重要性，需要实际实现
        
        let error = (predicted_score as i16 - test_case.expected_importance_score as i16).abs();
        let within_tolerance = error <= self.config.importance_score_tolerance as i16;
        
        Ok(ImportanceResult {
            test_case_id: test_case.test_case_id.clone(),
            input_text: test_case.input_text.clone(),
            predicted_score,
            expected_score: test_case.expected_importance_score,
            error: error as u8,
            within_tolerance,
        })
    }
    
    /// 评估去重效果
    async fn evaluate_deduplication(
        &self,
        memory_manager: &MemoryManager,
        test_case: &EffectivenessTestCase,
    ) -> Result<DeduplicationResult> {
        // 测试重复内容的检测和合并
        info!("评估去重效果 - 测试用例: {}", test_case.test_case_id);
        
        // 在实际实现中，应该测试memory_manager的去重功能
        // 这里我们暂时返回默认值，需要实际实现
        
        Ok(DeduplicationResult {
            test_case_id: test_case.test_case_id.clone(),
            duplicate_detected: false, // 需要实际测试
            correctly_merged: false, // 需要实际测试
            merge_quality: 0.0, // 需要实际测试
        })
    }
    
    /// 评估记忆更新
    async fn evaluate_memory_update(
        &self,
        memory_manager: &MemoryManager,
        test_case: &EffectivenessTestCase,
        existing_memories: &HashMap<String, Memory>,
    ) -> Result<UpdateResult> {
        // 测试记忆更新逻辑
        info!("评估记忆更新 - 测试用例: {}", test_case.test_case_id);
        
        // 在实际实现中，应该测试memory_manager的更新功能
        // 这里我们暂时返回默认值，需要实际实现
        
        Ok(UpdateResult {
            test_case_id: test_case.test_case_id.clone(),
            update_correct: false, // 需要实际测试
            merge_correct: false, // 需要实际测试
            conflict_resolved: false, // 需要实际测试
            updated_quality: 0.0, // 需要实际测试
        })
    }
    
    /// 计算事实提取指标
    fn calculate_fact_extraction_metrics(
        &self,
        results: &[FactExtractionResult],
    ) -> FactExtractionMetrics {
        let total_facts_extracted: usize = results.iter()
            .map(|r| r.extracted_facts.len())
            .sum();
        
        let total_correct_facts: usize = results.iter()
            .map(|r| r.matched_facts)
            .sum();
        
        let total_expected_facts: usize = results.iter()
            .map(|r| r.ground_truth_facts.len())
            .sum();
        
        let precision = if total_facts_extracted > 0 {
            total_correct_facts as f64 / total_facts_extracted as f64
        } else {
            0.0
        };
        
        let recall = if total_expected_facts > 0 {
            total_correct_facts as f64 / total_expected_facts as f64
        } else {
            0.0
        };
        
        let f1_score = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        
        FactExtractionMetrics {
            precision,
            recall,
            f1_score,
            facts_extracted: total_facts_extracted,
            correct_facts: total_correct_facts,
            detailed_results: results.to_vec(),
        }
    }
    
    /// 计算分类指标
    fn calculate_classification_metrics(
        &self,
        results: &[ClassificationResult],
    ) -> ClassificationMetrics {
        let total_correct = results.iter().filter(|r| r.is_correct).count();
        let accuracy = if !results.is_empty() {
            total_correct as f64 / results.len() as f64
        } else {
            0.0
        };
        
        // 按类别统计
        let mut confusion_matrix: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut precision_by_class: HashMap<String, f64> = HashMap::new();
        let mut recall_by_class: HashMap<String, f64> = HashMap::new();
        let mut f1_by_class: HashMap<String, f64> = HashMap::new();
        
        for result in results {
            let predicted = format!("{:?}", result.predicted_type);
            let expected = format!("{:?}", result.expected_type);
            
            *confusion_matrix
                .entry(expected.clone())
                .or_default()
                .entry(predicted.clone())
                .or_default() += 1;
        }
        
        // 计算每个类别的指标
        for (expected_class, predictions) in &confusion_matrix {
            let total_predicted_as_class: usize = confusion_matrix.values()
                .map(|pred_map| pred_map.get(expected_class).copied().unwrap_or(0))
                .sum();
            
            let true_positives = predictions.get(expected_class).copied().unwrap_or(0);
            let false_positives = total_predicted_as_class - true_positives;
            let false_negatives: usize = predictions.values().sum::<usize>() - true_positives;
            
            let precision = if true_positives + false_positives > 0 {
                true_positives as f64 / (true_positives + false_positives) as f64
            } else {
                0.0
            };
            
            let recall = if true_positives + false_negatives > 0 {
                true_positives as f64 / (true_positives + false_negatives) as f64
            } else {
                0.0
            };
            
            let f1 = if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            };
            
            precision_by_class.insert(expected_class.clone(), precision);
            recall_by_class.insert(expected_class.clone(), recall);
            f1_by_class.insert(expected_class.clone(), f1);
        }
        
        ClassificationMetrics {
            accuracy,
            precision_by_class,
            recall_by_class,
            f1_by_class,
            confusion_matrix,
        }
    }
    
    /// 计算重要性评估指标
    fn calculate_importance_metrics(
        &self,
        results: &[ImportanceResult],
    ) -> ImportanceMetrics {
        if results.is_empty() {
            return ImportanceMetrics {
                correlation_score: 0.0,
                mean_absolute_error: 0.0,
                root_mean_squared_error: 0.0,
                score_distribution: HashMap::new(),
                within_tolerance_rate: 0.0,
            };
        }
        
        let mut total_abs_error = 0.0;
        let mut total_squared_error = 0.0;
        let mut predicted_scores = Vec::new();
        let mut expected_scores = Vec::new();
        let mut score_distribution: HashMap<usize, usize> = HashMap::new();
        
        for result in results {
            let error = result.error as f64;
            total_abs_error += error;
            total_squared_error += error * error;
            
            predicted_scores.push(result.predicted_score as f64);
            expected_scores.push(result.expected_score as f64);
            
            *score_distribution.entry(result.predicted_score as usize).or_default() += 1;
        }
        
        let mean_absolute_error = total_abs_error / results.len() as f64;
        let root_mean_squared_error = (total_squared_error / results.len() as f64).sqrt();
        
        // 简化相关性计算（实际应该使用皮尔逊相关系数）
        let correlation_score = if !predicted_scores.is_empty() && !expected_scores.is_empty() {
            // 模拟相关性计算
            let avg_predicted: f64 = predicted_scores.iter().sum::<f64>() / predicted_scores.len() as f64;
            let avg_expected: f64 = expected_scores.iter().sum::<f64>() / expected_scores.len() as f64;
            
            let mut covariance = 0.0;
            let mut var_predicted = 0.0;
            let mut var_expected = 0.0;
            
            for i in 0..predicted_scores.len() {
                let diff_pred = predicted_scores[i] - avg_predicted;
                let diff_exp = expected_scores[i] - avg_expected;
                covariance += diff_pred * diff_exp;
                var_predicted += diff_pred * diff_pred;
                var_expected += diff_exp * diff_exp;
            }
            
            if var_predicted > 0.0 && var_expected > 0.0 {
                covariance / (var_predicted.sqrt() * var_expected.sqrt())
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        ImportanceMetrics {
            correlation_score: correlation_score.max(0.0).min(1.0),
            mean_absolute_error,
            root_mean_squared_error,
            score_distribution,
            within_tolerance_rate: 0.0, // 暂时设为0，后续可以计算实际值
        }
    }
    
    /// 计算去重指标
    fn calculate_deduplication_metrics(
        &self,
        results: &[DeduplicationResult],
    ) -> DeduplicationMetrics {
        if results.is_empty() {
            return DeduplicationMetrics {
                duplicate_detection_precision: 0.0,
                duplicate_detection_recall: 0.0,
                merge_accuracy: 0.0,
                duplicate_pairs_detected: 0,
                actual_duplicate_pairs: 0,
                avg_merge_quality: 0.0,
            };
        }
        
        let true_positives = results.iter().filter(|r| r.duplicate_detected).count();
        let false_positives = 0; // 简化：假设没有误报
        let false_negatives = 0; // 简化：假设没有漏报
        
        let precision = if true_positives + false_positives > 0 {
            true_positives as f64 / (true_positives + false_positives) as f64
        } else {
            0.0
        };
        
        let recall = if true_positives + false_negatives > 0 {
            true_positives as f64 / (true_positives + false_negatives) as f64
        } else {
            0.0
        };
        
        let merge_accuracy = if !results.is_empty() {
            results.iter().filter(|r| r.correctly_merged).count() as f64 / results.len() as f64
        } else {
            0.0
        };
        
        DeduplicationMetrics {
            duplicate_detection_precision: precision,
            duplicate_detection_recall: recall,
            merge_accuracy,
            duplicate_pairs_detected: true_positives,
            actual_duplicate_pairs: true_positives, // 简化：假设检测到的都是实际的
            avg_merge_quality: merge_accuracy, // 暂时用合并准确率作为合并质量
        }
    }
    
    /// 计算更新指标
    fn calculate_update_metrics(
        &self,
        results: &[UpdateResult],
    ) -> UpdateMetrics {
        if results.is_empty() {
            return UpdateMetrics {
                update_operation_accuracy: 0.0,
                merge_operation_accuracy: 0.0,
                conflict_resolution_accuracy: 0.0,
                updated_memory_quality: 0.0,
            };
        }
        
        let update_accuracy = results.iter().filter(|r| r.update_correct).count() as f64 / results.len() as f64;
        let merge_accuracy = results.iter().filter(|r| r.merge_correct).count() as f64 / results.len() as f64;
        let conflict_accuracy = results.iter().filter(|r| r.conflict_resolved).count() as f64 / results.len() as f64;
        let avg_quality = results.iter().map(|r| r.updated_quality).sum::<f64>() / results.len() as f64;
        
        UpdateMetrics {
            update_operation_accuracy: update_accuracy,
            merge_operation_accuracy: merge_accuracy,
            conflict_resolution_accuracy: conflict_accuracy,
            updated_memory_quality: avg_quality,
        }
    }
    
    /// 计算综合得分
    fn calculate_overall_score(
        &self,
        fact_metrics: &FactExtractionMetrics,
        classification_metrics: &ClassificationMetrics,
        importance_metrics: &ImportanceMetrics,
        deduplication_metrics: &DeduplicationMetrics,
        update_metrics: &UpdateMetrics,
    ) -> f64 {
        let mut total_score = 0.0;
        let mut weight_sum = 0.0;
        
        // 事实提取权重：0.3
        if self.config.verify_fact_extraction {
            let fact_score = (fact_metrics.f1_score + fact_metrics.precision + fact_metrics.recall) / 3.0;
            total_score += fact_score * 0.3;
            weight_sum += 0.3;
        }
        
        // 分类权重：0.2
        if self.config.verify_classification {
            total_score += classification_metrics.accuracy * 0.2;
            weight_sum += 0.2;
        }
        
        // 重要性评估权重：0.2
        if self.config.verify_importance_evaluation {
            let importance_score = 1.0 - importance_metrics.mean_absolute_error / 10.0; // 归一化到0-1
            total_score += importance_score.max(0.0).min(1.0) * 0.2;
            weight_sum += 0.2;
        }
        
        // 去重权重：0.15
        if self.config.verify_deduplication {
            let dedup_score = (deduplication_metrics.duplicate_detection_precision +
                deduplication_metrics.duplicate_detection_recall +
                deduplication_metrics.merge_accuracy) / 3.0;
            total_score += dedup_score * 0.15;
            weight_sum += 0.15;
        }
        
        // 更新权重：0.15
        if self.config.verify_memory_update {
            let update_score = (update_metrics.update_operation_accuracy +
                update_metrics.merge_operation_accuracy +
                update_metrics.conflict_resolution_accuracy +
                update_metrics.updated_memory_quality) / 4.0;
            total_score += update_score * 0.15;
            weight_sum += 0.15;
        }
        
        if weight_sum > 0.0 {
            total_score / weight_sum
        } else {
            0.0
        }
    }
    
    /// 加载测试数据集
    pub fn load_dataset(path: &str) -> Result<EffectivenessTestDataset> {
        let content = std::fs::read_to_string(path)
            .context(format!("读取数据集文件失败: {}", path))?;
        
        let dataset: EffectivenessTestDataset = serde_json::from_str(&content)
            .context("解析数据集JSON失败")?;
        
        info!("加载有效性测试数据集: {}个测试用例, {}个现有记忆",
            dataset.test_cases.len(), dataset.existing_memories.len());
        
        Ok(dataset)
    }
    
    /// 保存评估结果
    pub fn save_results(&self, metrics: &EffectivenessMetrics, output_path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(metrics)
            .context("序列化评估结果失败")?;
        
        std::fs::write(output_path, json)
            .context(format!("写入评估结果文件失败: {}", output_path))?;
        
        info!("有效性评估结果已保存到: {}", output_path);
        Ok(())
    }
}

// 辅助结构体
#[derive(Debug, Clone)]
struct ClassificationResult {
    test_case_id: String,
    input_text: String,
    predicted_type: MemoryType,
    expected_type: MemoryType,
    is_correct: bool,
}

#[derive(Debug, Clone)]
struct ImportanceResult {
    test_case_id: String,
    input_text: String,
    predicted_score: u8,
    expected_score: u8,
    error: u8,
    within_tolerance: bool,
}

#[derive(Debug, Clone)]
struct DeduplicationResult {
    test_case_id: String,
    duplicate_detected: bool,
    correctly_merged: bool,
    merge_quality: f64,
}

#[derive(Debug, Clone)]
struct UpdateResult {
    test_case_id: String,
    update_correct: bool,
    merge_correct: bool,
    conflict_resolved: bool,
    updated_quality: f64,
}