use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

mod evaluator;
mod dataset;
mod runner;
mod report;
mod memory;

use crate::runner::ExperimentRunner;

/// Cortex-Mem 核心能力评估框架
#[derive(Parser)]
#[command(name = "cortex-mem-evaluation")]
#[command(about = "评估 Cortex-Mem 核心能力的框架", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 运行完整评估
    Run {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/evaluation_config.toml")]
        config: PathBuf,
        
        /// 输出目录
        #[arg(short, long, default_value = "results")]
        output_dir: PathBuf,
    },
    
    /// 仅运行召回率评估
    Recall {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/evaluation_config.toml")]
        config: PathBuf,
        
        /// 输出目录
        #[arg(short, long, default_value = "results")]
        output_dir: PathBuf,
    },
    
    /// 仅运行有效性评估
    Effectiveness {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/evaluation_config.toml")]
        config: PathBuf,
        
        /// 输出目录
        #[arg(short, long, default_value = "results")]
        output_dir: PathBuf,
    },
    
    /// 仅运行性能评估
    Performance {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/evaluation_config.toml")]
        config: PathBuf,
        
        /// 输出目录
        #[arg(short, long, default_value = "results")]
        output_dir: PathBuf,
    },
    
    /// 生成测试数据集
    GenerateDataset {
        /// 数据集类型：recall, effectiveness, all
        #[arg(short, long, default_value = "all")]
        dataset_type: String,
        
        /// 输出目录
        #[arg(short, long, default_value = "data")]
        output_dir: PathBuf,
        
        /// 数据集大小
        #[arg(short, long, default_value = "100")]
        size: usize,
        
        /// 是否使用实验室数据
        #[arg(long, default_value = "true")]
        use_lab_data: bool,
    },
    
    /// 验证测试数据集
    ValidateDataset {
        /// 数据集路径
        #[arg(short, long)]
        dataset_path: PathBuf,
        
        /// 数据集类型：recall, effectiveness
        #[arg(short, long)]
        dataset_type: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run { config, output_dir } => {
            info!("开始运行完整评估...");
            let runner = ExperimentRunner::new(config, output_dir)?;
            runner.run_full_evaluation().await?;
            info!("评估完成！");
        }
        
        Commands::Recall { config, output_dir } => {
            info!("开始运行召回率评估...");
            let runner = ExperimentRunner::new(config, output_dir)?;
            runner.run_recall_evaluation().await?;
            info!("召回率评估完成！");
        }
        
        Commands::Effectiveness { config, output_dir } => {
            info!("开始运行有效性评估...");
            let runner = ExperimentRunner::new(config, output_dir)?;
            runner.run_effectiveness_evaluation().await?;
            info!("有效性评估完成！");
        }
        
        Commands::Performance { config, output_dir } => {
            info!("开始运行性能评估...");
            let runner = ExperimentRunner::new(config, output_dir)?;
            runner.run_full_evaluation().await?;
            info!("性能评估完成！");
        }
        
        Commands::GenerateDataset { dataset_type, output_dir, size, use_lab_data } => {
            info!("开始生成测试数据集...");
            crate::dataset::generate_test_dataset(&dataset_type, &output_dir, size, use_lab_data).await?;
            info!("测试数据集生成完成！");
        }
        
        Commands::ValidateDataset { dataset_path, dataset_type } => {
            info!("开始验证测试数据集...");
            crate::dataset::validate_dataset(&dataset_path, &dataset_type).await?;
            info!("测试数据集验证完成！");
        }
    }
    
    Ok(())
}