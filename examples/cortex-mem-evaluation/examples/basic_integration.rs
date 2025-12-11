//! 基本集成示例
//! 展示如何将实际的 MemoryManager 注入评估框架

use anyhow::Result;
use cortex_mem_core::{MemoryManager, Config};
use cortex_mem_evaluation::{
    evaluator::{RecallEvaluator, RecallEvaluationConfig},
    dataset::DatasetLoader,
};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Cortex-Mem 评估集成示例 ===");
    
    // 1. 创建实际的 MemoryManager
    println!("1. 创建 MemoryManager...");
    // 注意：这里需要根据你的实际配置来初始化
    // let config = Config::default();
    // let memory_manager = MemoryManager::new(config)?;
    
    // 由于缺少实际配置，我们暂时注释掉
    println!("   [提示] 需要提供实际的 MemoryManager 配置");
    
    // 2. 加载测试数据集
    println!("2. 加载测试数据集...");
    let dataset_path = Path::new("data/test_cases/recall_test_cases.json");
    
    if !dataset_path.exists() {
        println!("   数据集不存在，请先运行: ./scripts/run_evaluation.sh --generate");
        return Ok(());
    }
    
    let dataset = DatasetLoader::load_recall_dataset(dataset_path)?;
    println!("   加载完成: {}个测试用例, {}个记忆", 
        dataset.test_cases.len(), dataset.memories.len());
    
    // 3. 配置评估器
    println!("3. 配置召回率评估器...");
    let eval_config = RecallEvaluationConfig {
        k_values: vec![1, 3, 5, 10],
        similarity_thresholds: vec![0.7, 0.8, 0.9],
        max_results_per_query: 20,
        save_detailed_results: true,
    };
    
    let evaluator = RecallEvaluator::new(eval_config);
    
    // 4. 运行评估（需要实际的 MemoryManager）
    println!("4. 运行评估...");
    println!("   [提示] 需要取消注释下面的代码并提供 MemoryManager 实例");
    
    /*
    let metrics = evaluator.evaluate(&memory_manager, &dataset).await?;
    
    println!("5. 保存评估结果...");
    evaluator.save_results(&metrics, "results/real_recall_metrics.json")?;
    
    println!("=== 评估完成 ===");
    println!("结果保存在: results/real_recall_metrics.json");
    */
    
    println!("=== 集成示例结束 ===");
    println!("下一步: 提供实际的 MemoryManager 实例并取消注释评估代码");
    
    Ok(())
}