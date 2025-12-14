//! 实验运行器
//! 
//! 负责协调和运行完整的评估实验

use anyhow::{Result, Context};
use config::Config;
use std::path::PathBuf;
use tracing::{info, error};

use crate::{
    evaluator::{
        RealRecallEvaluator, RealRecallEvaluationConfig,
        RealEffectivenessEvaluator, RealEffectivenessEvaluationConfig,
    },
    dataset::DatasetLoader,
    memory,
};

/// 实验运行器
pub struct ExperimentRunner {
    /// 配置
    config: ExperimentConfig,
    /// 输出目录
    output_dir: PathBuf,
}

/// 实验配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperimentConfig {
    /// 评估模式
    pub mode: String,
    /// 召回率评估配置
    pub recall_config: RealRecallEvaluationConfig,
    /// 有效性评估配置
    pub effectiveness_config: RealEffectivenessEvaluationConfig,
    /// 是否保存详细结果
    pub save_detailed_results: bool,
    /// MemoryManager配置文件路径（可选）
    #[serde(default)]
    pub memory_config_path: Option<String>,
}

impl ExperimentRunner {
    /// 创建新的实验运行器
    pub fn new(config_path: PathBuf, output_dir: PathBuf) -> Result<Self> {
        // 加载配置
        let config = Self::load_config(&config_path)?;
        
        // 创建输出目录
        std::fs::create_dir_all(&output_dir)
            .context(format!("创建输出目录失败: {:?}", output_dir))?;
        
        Ok(Self { config, output_dir })
    }
    
    /// 加载配置
    fn load_config(config_path: &PathBuf) -> Result<ExperimentConfig> {
        let config_builder = Config::builder()
            .add_source(config::File::from(config_path.clone()))
            .add_source(config::Environment::with_prefix("CORTEX_MEM_EVAL"))
            .build()
            .context("加载配置失败")?;
        
        let mode = config_builder.get_string("general.mode")
            .unwrap_or_else(|_| "all".to_string());
        
        let recall_config = RealRecallEvaluationConfig {
            k_values: config_builder.get_array("recall_evaluation.k_values")
                .unwrap_or_else(|_| vec![1.into(), 3.into(), 5.into(), 10.into()])
                .into_iter()
                .filter_map(|v| v.into_int().ok().map(|n| n as usize))
                .collect(),
            similarity_thresholds: config_builder.get_array("recall_evaluation.similarity_thresholds")
                .unwrap_or_else(|_| vec![0.7.into(), 0.8.into(), 0.9.into()])
                .into_iter()
                .filter_map(|v| v.into_float().ok().map(|f| f as f32))
                .collect(),
            max_results_per_query: config_builder.get_int("recall_evaluation.max_results_per_query")
                .unwrap_or(20) as usize,
            save_detailed_results: config_builder.get_bool("recall_evaluation.save_detailed_results")
                .unwrap_or(true),
            timeout_seconds: config_builder.get_int("recall_evaluation.timeout_seconds")
                .unwrap_or(30) as u64,
            enable_parallel_evaluation: config_builder.get_bool("recall_evaluation.enable_parallel_evaluation")
                .unwrap_or(true),
            verify_memory_integrity: config_builder.get_bool("recall_evaluation.verify_memory_integrity")
                .unwrap_or(true),
            test_cases_path: config_builder.get_string("recall_evaluation.test_cases_path")
                .unwrap_or_else(|_| "data/test_cases/lab_recall_dataset.json".to_string()),
        };
        
        let effectiveness_config = RealEffectivenessEvaluationConfig {
            verify_fact_extraction: config_builder.get_bool("effectiveness_evaluation.verify_fact_extraction")
                .unwrap_or(true),
            verify_classification: config_builder.get_bool("effectiveness_evaluation.verify_classification")
                .unwrap_or(true),
            verify_importance_evaluation: config_builder.get_bool("effectiveness_evaluation.verify_importance_evaluation")
                .unwrap_or(true),
            verify_deduplication: config_builder.get_bool("effectiveness_evaluation.verify_deduplication")
                .unwrap_or(true),
            verify_memory_update: config_builder.get_bool("effectiveness_evaluation.verify_memory_update")
                .unwrap_or(true),
            importance_score_tolerance: config_builder.get_int("effectiveness_evaluation.importance_score_tolerance")
                .unwrap_or(1) as u8,
            timeout_seconds: config_builder.get_int("effectiveness_evaluation.timeout_seconds")
                .unwrap_or(30) as u64,
            enable_verbose_logging: config_builder.get_bool("effectiveness_evaluation.enable_verbose_logging")
                .unwrap_or(false),
            cleanup_test_data: config_builder.get_bool("effectiveness_evaluation.cleanup_test_data")
                .unwrap_or(true),
            test_cases_path: config_builder.get_string("effectiveness_evaluation.test_cases_path")
                .unwrap_or_else(|_| "data/test_cases/lab_effectiveness_dataset.json".to_string()),
        };
        
        let config = ExperimentConfig {
            mode,
            recall_config,
            effectiveness_config,
            save_detailed_results: config_builder.get_bool("general.save_detailed_results")
                .unwrap_or(true),
            memory_config_path: config_builder.get_string("general.memory_config_path").ok(),
        };
        
        info!("实验配置加载完成: mode={}", config.mode);
        Ok(config)
    }
    
    /// 运行完整评估
    pub async fn run_full_evaluation(&self) -> Result<()> {
        info!("开始完整评估...");
        
        // 运行真实评估
        self.run_real_evaluation().await?;
        
        info!("完整评估完成");
        Ok(())
    }
    
    /// 运行召回率评估
    pub async fn run_recall_evaluation(&self) -> Result<()> {
        info!("开始召回率评估...");
        
        // 检查数据集
        let dataset_path = PathBuf::from(&self.config.recall_config.test_cases_path);
        if !dataset_path.exists() {
            anyhow::bail!("召回率测试数据集不存在: {:?}", dataset_path);
        }
        
        // 加载数据集
        let dataset = DatasetLoader::load_recall_dataset(&dataset_path)?;
        
        // 创建MemoryManager实例
        let memory_manager = memory::create_memory_manager_for_real_evaluation(&self.config).await?;
        
        // 创建评估器
        let evaluator = RealRecallEvaluator::new(self.config.recall_config.clone(), memory_manager);
        
        // 运行评估
        let metrics = evaluator.evaluate(&dataset).await?;
        
        // 保存结果
        let result_path = self.output_dir.join("real_recall_evaluation_result.json");
        let result_json = serde_json::to_string_pretty(&metrics)?;
        std::fs::write(result_path, result_json)?;
        
        info!("召回率评估完成");
        Ok(())
    }
    
    /// 运行有效性评估
    pub async fn run_effectiveness_evaluation(&self) -> Result<()> {
        info!("开始有效性评估...");
        
        // 检查数据集
        let dataset_path = PathBuf::from(&self.config.effectiveness_config.test_cases_path);
        if !dataset_path.exists() {
            anyhow::bail!("有效性测试数据集不存在: {:?}", dataset_path);
        }
        
        // 加载数据集
        let dataset = DatasetLoader::load_effectiveness_dataset(&dataset_path)?;
        
        // 创建MemoryManager实例
        let memory_manager = memory::create_memory_manager_for_real_evaluation(&self.config).await?;
        
        // 创建评估器
        let evaluator = RealEffectivenessEvaluator::new(self.config.effectiveness_config.clone(), memory_manager);
        
        // 运行评估
        let metrics = evaluator.evaluate(&dataset).await?;
        
        // 保存结果
        let result_path = self.output_dir.join("real_effectiveness_evaluation_result.json");
        let result_json = serde_json::to_string_pretty(&metrics)?;
        std::fs::write(result_path, result_json)?;
        
        info!("有效性评估完成");
        Ok(())
    }
    
    /// 运行真实评估
    async fn run_real_evaluation(&self) -> Result<()> {
        info!("开始真实评估...");
        
        // 检查数据集路径
        let recall_dataset_path = PathBuf::from(&self.config.recall_config.test_cases_path);
        let effectiveness_dataset_path = PathBuf::from(&self.config.effectiveness_config.test_cases_path);
        
        if !recall_dataset_path.exists() {
            anyhow::bail!("召回率测试数据集不存在: {:?}", recall_dataset_path);
        }
        
        if !effectiveness_dataset_path.exists() {
            anyhow::bail!("有效性测试数据集不存在: {:?}", effectiveness_dataset_path);
        }
        
        info!("数据集检查通过:");
        info!("  - 召回率数据集: {:?}", recall_dataset_path);
        info!("  - 有效性数据集: {:?}", effectiveness_dataset_path);
        
        // 创建 MemoryManager 实例
        info!("创建 MemoryManager 实例...");
        let memory_manager = memory::create_memory_manager_for_real_evaluation(&self.config).await?;
        info!("MemoryManager 实例创建成功");
        
        // 运行召回率评估
        info!("运行真实召回率评估...");
        self.run_real_recall_evaluation(&memory_manager, &recall_dataset_path).await?;
        
        // 运行有效性评估
        info!("运行真实有效性评估...");
        self.run_real_effectiveness_evaluation(&memory_manager, &effectiveness_dataset_path).await?;
        
        // 生成真实评估报告
        let real_report = self.generate_real_evaluation_report()?;
        let report_path = self.output_dir.join("real_evaluation_report.md");
        std::fs::write(report_path, real_report)?;
        
        info!("真实评估完成");
        info!("评估结果已保存到: {:?}", self.output_dir);
        
        Ok(())
    }
    
    /// 运行真实召回率评估
    async fn run_real_recall_evaluation(
        &self,
        memory_manager: &std::sync::Arc<cortex_mem_core::MemoryManager>,
        dataset_path: &PathBuf,
    ) -> Result<()> {
        info!("运行真实召回率评估...");
        
        // 加载数据集
        let dataset = DatasetLoader::load_recall_dataset(dataset_path)?;
        
        // 创建评估器
        let evaluator = RealRecallEvaluator::new(self.config.recall_config.clone(), memory_manager.clone());
        
        // 运行评估
        let metrics = evaluator.evaluate(&dataset).await?;
        
        // 保存结果
        let result_path = self.output_dir.join("real_recall_evaluation_result.json");
        let result_json = serde_json::to_string_pretty(&metrics)?;
        std::fs::write(result_path, result_json)?;
        
        info!("真实召回率评估完成");
        Ok(())
    }
    
    /// 运行真实有效性评估
    async fn run_real_effectiveness_evaluation(
        &self,
        memory_manager: &std::sync::Arc<cortex_mem_core::MemoryManager>,
        dataset_path: &PathBuf,
    ) -> Result<()> {
        info!("运行真实有效性评估...");
        
        // 加载数据集
        let dataset = DatasetLoader::load_effectiveness_dataset(dataset_path)?;
        
        // 创建评估器
        let evaluator = RealEffectivenessEvaluator::new(self.config.effectiveness_config.clone(), memory_manager.clone());
        
        // 运行评估
        let metrics = evaluator.evaluate(&dataset).await?;
        
        // 保存结果
        let result_path = self.output_dir.join("real_effectiveness_evaluation_result.json");
        let result_json = serde_json::to_string_pretty(&metrics)?;
        std::fs::write(result_path, result_json)?;
        
        info!("真实有效性评估完成");
        Ok(())
    }
    
    /// 生成真实评估报告
    fn generate_real_evaluation_report(&self) -> Result<String> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        let mut report = String::new();
        
        report.push_str("# Cortex-Mem 真实评估报告\n\n");
        report.push_str("## 概述\n\n");
        report.push_str("本报告展示了 Cortex-Mem 系统的真实评估结果。\n\n");
        
        report.push_str("## 评估配置\n\n");
        report.push_str(&format!("- **评估模式**: {}\n", self.config.mode));
        report.push_str(&format!("- **输出目录**: {:?}\n", self.output_dir));
        
        report.push_str("\n## 数据集状态\n\n");
        
        let recall_dataset_path = PathBuf::from(&self.config.recall_config.test_cases_path);
        let effectiveness_dataset_path = PathBuf::from(&self.config.effectiveness_config.test_cases_path);
        
        if recall_dataset_path.exists() {
            report.push_str("- **召回率数据集**: ✅ 存在\n");
            if let Ok(metadata) = std::fs::metadata(&recall_dataset_path) {
                report.push_str(&format!("  - 文件大小: {} 字节\n", metadata.len()));
            }
        } else {
            report.push_str("- **召回率数据集**: ❌ 不存在\n");
        }
        
        if effectiveness_dataset_path.exists() {
            report.push_str("- **有效性数据集**: ✅ 存在\n");
            if let Ok(metadata) = std::fs::metadata(&effectiveness_dataset_path) {
                report.push_str(&format!("  - 文件大小: {} 字节\n", metadata.len()));
            }
        } else {
            report.push_str("- **有效性数据集**: ❌ 不存在\n");
        }
        
        report.push_str("\n## 评估结果\n\n");
        report.push_str("评估结果已保存到以下文件：\n\n");
        report.push_str("- `real_recall_evaluation_result.json` - 召回率评估结果\n");
        report.push_str("- `real_effectiveness_evaluation_result.json` - 有效性评估结果\n");
        
        report.push_str("\n## 技术栈\n\n");
        report.push_str("- **向量存储**: Qdrant\n");
        report.push_str("- **LLM客户端**: 真实API客户端\n");
        report.push_str("- **评估框架**: 真实评估器（无模拟代码）\n");
        
        report.push_str("\n## 报告信息\n\n");
        report.push_str(&format!("- **生成时间**: {}\n", timestamp));
        report.push_str("- **评估类型**: 真实评估（无模拟代码）\n");
        
        Ok(report)
    }
}
