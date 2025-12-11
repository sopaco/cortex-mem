//! 实验运行器
//! 
//! 负责协调和运行完整的评估实验

use anyhow::{Result, Context};
use config::Config;
use std::path::PathBuf;
use tracing::{info, error, warn};

use crate::{
    evaluator::{
        RecallEvaluator, RecallEvaluationConfig, RecallTestDataset,
        EffectivenessEvaluator, EffectivenessEvaluationConfig, EffectivenessTestDataset,
    },
    dataset::DatasetGenerator,
    report::ReportGenerator,
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
    pub recall_config: RecallEvaluationConfig,
    /// 有效性评估配置
    pub effectiveness_config: EffectivenessEvaluationConfig,
    /// 是否生成数据集
    pub generate_dataset: bool,
    /// 数据集大小
    pub dataset_size: usize,
    /// 是否保存详细结果
    pub save_detailed_results: bool,
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
        
        let recall_config = RecallEvaluationConfig {
            k_values: config_builder.get_array("recall_evaluation.k_values")
                .unwrap_or_else(|_| vec![1.into(), 3.into(), 5.into(), 10.into()])
                .into_iter()
                .filter_map(|v| v.into_int().ok().map(|n| n as usize))
                .collect(),
            similarity_thresholds: config_builder.get_array("recall_evaluation.similarity_thresholds")
                .unwrap_or_else(|_| vec![0.7.into(), 0.8.into(), 0.9.into()])
                .into_iter()
                .filter_map(|v| v.into_float().ok().map(|f| f as f64))
                .collect(),
            max_results_per_query: config_builder.get_int("recall_evaluation.max_results_per_query")
                .unwrap_or(20) as usize,
            save_detailed_results: config_builder.get_bool("recall_evaluation.save_detailed_results")
                .unwrap_or(true),
        };
        
        let effectiveness_config = EffectivenessEvaluationConfig {
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
            llm_evaluation_enabled: config_builder.get_bool("effectiveness_evaluation.llm_evaluation_enabled")
                .unwrap_or(false),
        };
        
        let config = ExperimentConfig {
            mode,
            recall_config,
            effectiveness_config,
            generate_dataset: false, // 默认不生成，由命令行参数控制
            dataset_size: config_builder.get_int("general.dataset_size")
                .unwrap_or(100) as usize,
            save_detailed_results: config_builder.get_bool("general.save_detailed_results")
                .unwrap_or(true),
        };
        
        info!("实验配置加载完成: mode={}", config.mode);
        Ok(config)
    }
    
    /// 运行完整评估
    pub async fn run_full_evaluation(&self) -> Result<()> {
        info!("开始完整评估...");
        
        // 这里需要实际的 MemoryManager 实例
        // 由于这是一个框架，我们暂时模拟评估过程
        
        warn!("注意: 这是一个评估框架，需要实际的 MemoryManager 实例才能运行完整评估");
        warn!("当前运行的是模拟评估，用于验证框架功能");
        
        // 模拟评估过程
        self.run_simulation().await?;
        
        info!("完整评估完成（模拟模式）");
        Ok(())
    }
    
    /// 运行召回率评估
    pub async fn run_recall_evaluation(&self) -> Result<()> {
        info!("开始召回率评估...");
        
        // 检查数据集
        let dataset_path = PathBuf::from("data/test_cases/recall_test_cases.json");
        if !dataset_path.exists() {
            if self.config.generate_dataset {
                info!("生成召回率测试数据集...");
                self.generate_recall_dataset()?;
            } else {
                anyhow::bail!("召回率测试数据集不存在，请使用 --generate 参数生成数据集");
            }
        }
        
        // 加载数据集
        let dataset = RecallEvaluator::load_dataset(dataset_path.to_str().unwrap())?;
        
        // 创建评估器
        let evaluator = RecallEvaluator::new(self.config.recall_config.clone());
        
        // 这里需要实际的 MemoryManager 实例
        // 由于缺少实际实例，我们运行模拟评估
        warn!("注意: 这是一个评估框架，需要实际的 MemoryManager 实例才能运行真实评估");
        warn!("当前运行的是模拟评估，用于验证框架功能");
        
        info!("召回率评估框架就绪");
        info!("测试数据集: {}个查询, {}个记忆", 
            dataset.test_cases.len(), dataset.memories.len());
        
        // 保存数据集信息
        let dataset_info = serde_json::to_string_pretty(&dataset.metadata)?;
        let info_path = self.output_dir.join("recall_dataset_info.json");
        std::fs::write(info_path, dataset_info)?;
        
        // 运行模拟评估
        self.run_simulation_for_recall().await?;
        
        info!("召回率评估完成（模拟模式）");
        Ok(())
    }
    
    /// 运行有效性评估
    pub async fn run_effectiveness_evaluation(&self) -> Result<()> {
        info!("开始有效性评估...");
        
        // 检查数据集
        let dataset_path = PathBuf::from("data/test_cases/effectiveness_test_cases.json");
        if !dataset_path.exists() {
            if self.config.generate_dataset {
                info!("生成有效性测试数据集...");
                self.generate_effectiveness_dataset()?;
            } else {
                anyhow::bail!("有效性测试数据集不存在，请使用 --generate 参数生成数据集");
            }
        }
        
        // 加载数据集
        let dataset = EffectivenessEvaluator::load_dataset(dataset_path.to_str().unwrap())?;
        
        // 创建评估器
        let evaluator = EffectivenessEvaluator::new(self.config.effectiveness_config.clone());
        
        // 这里需要实际的 MemoryManager 实例
        info!("有效性评估框架就绪");
        info!("需要注入 MemoryManager 实例以运行实际评估");
        info!("测试数据集: {}个测试用例, {}个现有记忆", 
            dataset.test_cases.len(), dataset.existing_memories.len());
        
        // 保存数据集信息
        let dataset_info = serde_json::to_string_pretty(&dataset.metadata)?;
        let info_path = self.output_dir.join("effectiveness_dataset_info.json");
        std::fs::write(info_path, dataset_info)?;
        
        info!("有效性评估框架验证完成");
        Ok(())
    }
    
    /// 运行性能评估
    pub async fn run_performance_evaluation(&self) -> Result<()> {
        info!("开始性能评估...");
        
        // 性能评估需要实际的 MemoryManager 实例
        info!("性能评估框架就绪");
        info!("需要注入 MemoryManager 实例以运行实际评估");
        info!("支持以下性能测试:");
        info!("  - 基准测试: 测量基本操作性能");
        info!("  - 负载测试: 模拟不同并发用户");
        info!("  - 压力测试: 测试系统极限");
        info!("  - 可扩展性测试: 验证不同规模下的性能");
        
        // 创建性能评估目录
        let perf_dir = self.output_dir.join("performance");
        std::fs::create_dir_all(&perf_dir)?;
        
        // 保存性能测试配置
        let perf_config = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(perf_dir.join("performance_config.json"), perf_config)?;
        
        info!("性能评估框架验证完成");
        Ok(())
    }
    
    /// 生成召回率数据集
    fn generate_recall_dataset(&self) -> Result<()> {
        info!("生成召回率测试数据集...");
        
        let mut generator = DatasetGenerator::new(Default::default());
        let dataset = generator.generate_recall_dataset()?;
        
        // 保存数据集
        let output_path = PathBuf::from("data/test_cases/recall_test_cases.json");
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        generator.save_dataset(&dataset, output_path.to_str().unwrap())?;
        
        info!("召回率数据集生成完成: {}个测试用例", dataset.test_cases.len());
        Ok(())
    }
    
    /// 生成有效性数据集
    fn generate_effectiveness_dataset(&self) -> Result<()> {
        info!("生成有效性测试数据集...");
        
        let mut generator = DatasetGenerator::new(Default::default());
        let dataset = generator.generate_effectiveness_dataset()?;
        
        // 保存数据集
        let output_path = PathBuf::from("data/test_cases/effectiveness_test_cases.json");
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        generator.save_dataset(&dataset, output_path.to_str().unwrap())?;
        
        info!("有效性数据集生成完成: {}个测试用例", dataset.test_cases.len());
        Ok(())
    }
    
    /// 运行模拟评估
    async fn run_simulation(&self) -> Result<()> {
        info!("运行模拟评估...");
        
        // 生成示例数据
        let mut generator = DatasetGenerator::new(Default::default());
        
        // 生成并保存数据集
        if self.config.mode == "all" || self.config.mode == "recall" {
            info!("模拟召回率评估...");
            let recall_dataset = generator.generate_recall_dataset()?;
            let recall_path = self.output_dir.join("simulated_recall_dataset.json");
            generator.save_dataset(&recall_dataset, recall_path.to_str().unwrap())?;
            
            // 模拟评估结果
            let recall_metrics = serde_json::json!({
                "precision_at_k": { "1": 0.85, "3": 0.78, "5": 0.72, "10": 0.65 },
                "recall_at_k": { "1": 0.45, "3": 0.68, "5": 0.82, "10": 0.95 },
                "mean_average_precision": 0.75,
                "normalized_discounted_cumulative_gain": 0.82,
                "simulation_note": "这是模拟数据，实际评估需要 MemoryManager 实例"
            });
            
            let metrics_path = self.output_dir.join("simulated_recall_metrics.json");
            std::fs::write(metrics_path, serde_json::to_string_pretty(&recall_metrics)?)?;
        }
        
        if self.config.mode == "all" || self.config.mode == "effectiveness" {
            info!("模拟有效性评估...");
            let effectiveness_dataset = generator.generate_effectiveness_dataset()?;
            let effectiveness_path = self.output_dir.join("simulated_effectiveness_dataset.json");
            generator.save_dataset(&effectiveness_dataset, effectiveness_path.to_str().unwrap())?;
            
            // 模拟评估结果
            let effectiveness_metrics = serde_json::json!({
                "fact_extraction_accuracy": {
                    "precision": 0.88,
                    "recall": 0.82,
                    "f1_score": 0.85,
                    "facts_extracted": 150,
                    "correct_facts": 132
                },
                "classification_accuracy": {
                    "accuracy": 0.92,
                    "precision_by_class": {
                        "Conversational": 0.94,
                        "Procedural": 0.89,
                        "Factual": 0.96
                    }
                },
                "overall_score": 0.87,
                "simulation_note": "这是模拟数据，实际评估需要 MemoryManager 实例"
            });
            
            let metrics_path = self.output_dir.join("simulated_effectiveness_metrics.json");
            std::fs::write(metrics_path, serde_json::to_string_pretty(&effectiveness_metrics)?)?;
        }
        
        // 生成综合报告
        let report = self.generate_simulation_report()?;
        let report_path = self.output_dir.join("simulation_report.md");
        std::fs::write(report_path, report)?;
        
        // 生成报告目录的内容
        self.generate_report_directories()?;
        
        info!("模拟评估完成，结果保存在: {:?}", self.output_dir);
        Ok(())
    }
    
    /// 运行召回率模拟评估
    async fn run_simulation_for_recall(&self) -> Result<()> {
        info!("运行召回率模拟评估...");
        
        // 生成示例数据
        let mut generator = DatasetGenerator::new(Default::default());
        
        // 模拟召回率评估结果
        let recall_metrics = serde_json::json!({
            "precision_at_k": { "1": 0.85, "3": 0.78, "5": 0.72, "10": 0.65 },
            "recall_at_k": { "1": 0.45, "3": 0.68, "5": 0.82, "10": 0.95 },
            "mean_average_precision": 0.75,
            "normalized_discounted_cumulative_gain": 0.82,
            "metrics_by_threshold": {
                "0.7": {
                    "threshold": 0.7,
                    "precision": 0.82,
                    "recall": 0.88,
                    "f1_score": 0.85,
                    "avg_results_returned": 8.5
                },
                "0.8": {
                    "threshold": 0.8,
                    "precision": 0.85,
                    "recall": 0.78,
                    "f1_score": 0.81,
                    "avg_results_returned": 6.2
                },
                "0.9": {
                    "threshold": 0.9,
                    "precision": 0.92,
                    "recall": 0.65,
                    "f1_score": 0.76,
                    "avg_results_returned": 3.8
                }
            },
            "simulation_note": "这是模拟数据，实际评估需要 MemoryManager 实例",
            "evaluation_timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        // 保存评估结果
        let metrics_path = self.output_dir.join("recall_metrics.json");
        std::fs::write(&metrics_path, serde_json::to_string_pretty(&recall_metrics)?)?;
        
        // 生成召回率评估报告
        let recall_report = self.generate_recall_report(&recall_metrics)?;
        let report_path = self.output_dir.join("recall_report.md");
        std::fs::write(report_path, recall_report)?;
        
        // 生成报告目录的内容
        self.generate_report_directories()?;
        
        info!("召回率模拟评估完成，结果已保存");
        Ok(())
    }
    
    /// 生成召回率评估报告
    fn generate_recall_report(&self, metrics: &serde_json::Value) -> Result<String> {
        let mut report = String::new();
        
        report.push_str("# 召回率评估报告\n\n");
        report.push_str("## 概述\n\n");
        report.push_str("本报告展示了 Cortex-Mem 系统的召回率评估结果。\n\n");
        
        report.push_str("## 评估配置\n\n");
        report.push_str("- **评估模式**: 召回率评估\n");
        report.push_str("- **数据集**: 50个测试查询，150个记忆\n");
        report.push_str("- **评估时间**: ");
        report.push_str(&chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        report.push_str("\n\n");
        
        report.push_str("## 评估结果\n\n");
        
        // 精确率@K
        report.push_str("### 精确率@K (Precision@K)\n\n");
        if let Some(precision_at_k) = metrics.get("precision_at_k") {
            if let Some(obj) = precision_at_k.as_object() {
                report.push_str("| K值 | 精确率 |\n");
                report.push_str("|-----|--------|\n");
                for (k, precision) in obj {
                    report.push_str(&format!("| {} | {:.3} |\n", k, precision.as_f64().unwrap_or(0.0)));
                }
                report.push_str("\n");
            }
        }
        
        // 召回率@K
        report.push_str("### 召回率@K (Recall@K)\n\n");
        if let Some(recall_at_k) = metrics.get("recall_at_k") {
            if let Some(obj) = recall_at_k.as_object() {
                report.push_str("| K值 | 召回率 |\n");
                report.push_str("|-----|--------|\n");
                for (k, recall) in obj {
                    report.push_str(&format!("| {} | {:.3} |\n", k, recall.as_f64().unwrap_or(0.0)));
                }
                report.push_str("\n");
            }
        }
        
        // 其他指标
        report.push_str("### 其他指标\n\n");
        report.push_str("| 指标 | 值 |\n");
        report.push_str("|------|----|\n");
        
        if let Some(map) = metrics.get("mean_average_precision") {
            report.push_str(&format!("| 平均精确率均值 (MAP) | {:.3} |\n", map.as_f64().unwrap_or(0.0)));
        }
        
        if let Some(ndcg) = metrics.get("normalized_discounted_cumulative_gain") {
            report.push_str(&format!("| 归一化折损累计增益 (NDCG) | {:.3} |\n", ndcg.as_f64().unwrap_or(0.0)));
        }
        
        report.push_str("\n");
        
        // 不同阈值下的指标
        report.push_str("### 不同相似度阈值下的表现\n\n");
        if let Some(metrics_by_threshold) = metrics.get("metrics_by_threshold") {
            if let Some(obj) = metrics_by_threshold.as_object() {
                report.push_str("| 阈值 | 精确率 | 召回率 | F1分数 | 平均返回结果数 |\n");
                report.push_str("|------|--------|--------|--------|----------------|\n");
                for (threshold, metrics_obj) in obj {
                    if let Some(m) = metrics_obj.as_object() {
                        let precision = m.get("precision").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let recall = m.get("recall").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let f1 = m.get("f1_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let avg_results = m.get("avg_results_returned").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        
                        report.push_str(&format!("| {} | {:.3} | {:.3} | {:.3} | {:.1} |\n", 
                            threshold, precision, recall, f1, avg_results));
                    }
                }
                report.push_str("\n");
            }
        }
        
        report.push_str("## 说明\n\n");
        report.push_str("> **注意**: 这是模拟评估数据，用于验证评估框架的功能。\n");
        report.push_str("> 要获取真实评估结果，需要提供实际的 MemoryManager 实例。\n");
        report.push_str("> 请参考 `examples/basic_integration.rs` 了解如何集成实际系统。\n");
        
        Ok(report)
    }
    
    /// 生成报告目录的内容
    fn generate_report_directories(&self) -> Result<()> {
        let reports_dir = self.output_dir.join("reports");
        let visualizations_dir = self.output_dir.join("visualizations");
        
        // 确保目录存在
        std::fs::create_dir_all(&reports_dir)?;
        std::fs::create_dir_all(&visualizations_dir)?;
        
        // 在reports目录中创建说明文件
        let reports_readme = r#"# 评估报告目录

本目录存放评估生成的各种报告文件。

## 文件说明

### 模拟模式
当运行模拟评估时，会生成以下文件：
- `simulation_report.md` - 模拟评估报告
- `simulated_recall_metrics.json` - 模拟召回率指标
- `simulated_effectiveness_metrics.json` - 模拟有效性指标

### 真实评估模式
当提供实际的 MemoryManager 实例后，会生成：
- `comprehensive_report.md` - 综合评估报告
- `comprehensive_report.html` - HTML格式报告
- `comprehensive_report.json` - JSON格式详细数据
- `recall_evaluation.json` - 召回率评估结果
- `effectiveness_evaluation.json` - 有效性评估结果
- `performance_evaluation.json` - 性能评估结果

## 如何运行真实评估
1. 修改代码，提供实际的 MemoryManager 实例
2. 运行: `cargo run -- run --config config/evaluation_config.toml`
3. 查看本目录中的报告文件

---
*当前处于模拟模式，等待实际系统集成*"#;
        
        std::fs::write(reports_dir.join("README.md"), reports_readme)?;
        
        // 在visualizations目录中创建说明文件
        let viz_readme = r#"# 可视化图表目录

本目录存放评估结果的可视化图表。

## 图表类型

### 召回率评估图表
- Precision-Recall 曲线
- Precision@K 折线图
- Recall@K 折线图

### 有效性评估图表
- 事实提取准确性雷达图
- 记忆分类混淆矩阵
- 重要性评分分布图

### 性能评估图表
- 延迟分布箱线图
- 吞吐量趋势图
- 资源使用监控图

## 生成图表
运行真实评估后，图表将自动生成在此目录中。

## 当前状态
✅ 图表生成框架就绪
⚠️ 需要实际评估数据
📊 支持 PNG、SVG、PDF 格式

---
*等待实际评估数据以生成图表*"#;
        
        std::fs::write(visualizations_dir.join("README.md"), viz_readme)?;
        
        // 创建一些示例图表数据
        let example_data = serde_json::json!({
            "note": "这是示例数据，实际图表需要运行真实评估",
            "supported_charts": [
                "precision_recall_curve",
                "confusion_matrix",
                "latency_distribution",
                "throughput_trend"
            ],
            "chart_formats": ["png", "svg", "pdf"],
            "framework_ready": true
        });
        
        std::fs::write(
            visualizations_dir.join("example_chart_data.json"),
            serde_json::to_string_pretty(&example_data)?
        )?;
        
        Ok(())
    }
    
    /// 生成模拟报告
    fn generate_simulation_report(&self) -> Result<String> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        let report = format!(
            "# Cortex-Mem 评估框架模拟报告

## 报告信息
- **生成时间**: {}
- **评估模式**: {}
- **输出目录**: {:?}

## 框架概述
这是一个用于评估 Cortex-Mem 核心能力的框架，包含以下功能：

### 1. 召回率评估
- 验证向量检索的准确性和完整性
- 支持 Precision@K、Recall@K、MAP、NDCG 等指标
- 可配置的相似度阈值和K值

### 2. 记忆有效性评估
- 测试事实提取准确性
- 验证记忆分类正确性
- 评估重要性评分合理性
- 检查去重和更新逻辑

### 3. 性能评估
- 基准测试：基本操作性能
- 负载测试：并发用户性能
- 压力测试：系统极限性能
- 可扩展性测试：不同规模性能

## 当前状态
✅ **框架结构完整**：所有核心模块已实现
✅ **配置系统就绪**：支持 TOML 配置文件和环境变量
✅ **数据集生成器**：可生成测试数据集
✅ **评估器接口**：定义了完整的评估接口
⚠️ **需要实际集成**：需要注入 MemoryManager 实例

## 下一步
1. 将 MemoryManager 实例注入评估框架
2. 运行实际评估获取真实数据
3. 根据评估结果优化 Cortex-Mem 实现
4. 定期运行评估确保质量

## 文件结构
```
评估框架/
├── config/evaluation_config.toml    # 评估配置
├── src/evaluator/                   # 评估器模块
├── src/dataset/                     # 数据集模块
├── src/runner/                      # 运行器模块
├── scripts/run_evaluation.sh        # 评估脚本
└── results/                         # 评估结果
```

## 使用方法
```bash
# 生成测试数据集
./scripts/run_evaluation.sh --generate --size 100

# 运行完整评估
./scripts/run_evaluation.sh --mode all

# 仅运行召回率评估
./scripts/run_evaluation.sh --mode recall
```

## 注意事项
- 当前报告基于模拟数据
- 实际评估需要提供 MemoryManager 实例
- 配置参数可根据需要调整
- 评估结果可用于指导优化方向

---
*报告生成时间: {}*
", timestamp, self.config.mode, self.output_dir, timestamp);
        
        Ok(report)
    }
}