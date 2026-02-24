mod agent;
mod app;
mod config;
mod infrastructure;
mod logger;
mod ui;

// 音频相关模块
mod audio_input;
mod audio_transcription;

use anyhow::{Context, Result};
use app::{App, create_default_bots};
use clap::Parser;
use config::ConfigManager;
use infrastructure::Infrastructure;
use logger::init_logger;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "cortex-mem-tars")]
#[command(about = "TARS, An Interactive Demonstration Program Based on Cortex Memory")]
#[command(author = "Sopaco")]
#[command(version)]
struct Args;

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数（目前无自定义参数）
    let _args = Args::parse();

    // 初始化配置管理器
    let config_manager = ConfigManager::new().context("无法初始化配置管理器")?;
    log::info!("配置管理器初始化成功");

    // 初始化日志系统
    let log_manager = init_logger(config_manager.config_dir()).context("无法初始化日志系统")?;
    log::info!("日志系统初始化成功");

    // 创建默认机器人
    create_default_bots(&config_manager).context("无法创建默认机器人")?;

    // 初始化基础设施（LLM 客户端、向量存储、记忆管理器）
    let infrastructure = match Infrastructure::new(config_manager.cortex_config().clone()).await {
        Ok(inf) => {
            log::info!("基础设施初始化成功");
            Some(Arc::new(inf))
        }
        Err(e) => {
            log::warn!("基础设施初始化失败，将使用 Mock Agent: {}", e);
            None
        }
    };

    // 创建并运行应用
    let mut app = App::new(
        config_manager,
        log_manager,
        infrastructure.clone(),
    )
    .context("无法创建应用")?;
    log::info!("应用创建成功");

    // 检查服务可用性
    app.check_service_status()
        .await
        .context("无法检查服务状态")?;

    // 运行应用
    app.run().await.context("应用运行失败")?;
    
    // 退出时自动提取记忆
    println!(
        "\n╔══════════════════════════════════════════════════════════════════════════════╗"
    );
    println!(
        "║                            🧠 Cortex Memory - 退出流程                       ║"
    );
    println!(
        "╚══════════════════════════════════════════════════════════════════════════════╝"
    );
    
    log::info!("🚀 开始退出流程，准备自动提取会话记忆...");
    
    match app.on_exit().await {
        Ok(_) => {
            log::info!("✅ 退出流程完成");
            println!(
                "\n╔══════════════════════════════════════════════════════════════════════════════╗"
            );
            println!(
                "║                                  🎉 退出流程完成                             ║"
            );
            println!(
                "╚══════════════════════════════════════════════════════════════════════════════╝"
            );
        }
        Err(e) => {
            log::warn!("⚠️ 退出流程出错: {}", e);
        }
    }

    println!("👋 Cortex TARS powering down. Goodbye!");

    Ok(())
}