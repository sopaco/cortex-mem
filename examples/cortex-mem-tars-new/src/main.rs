mod agent;
mod app;
mod config;
mod infrastructure;
mod logger;
mod ui;

use anyhow::{Context, Result};
use app::{create_default_bots, App};
use config::ConfigManager;
use infrastructure::Infrastructure;
use logger::init_logger;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
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
    ).context("无法创建应用")?;
    log::info!("应用创建成功");

    // 检查服务可用性
    app.check_service_status().await.context("无法检查服务状态")?;

    // 加载用户基本信息
    app.load_user_info().await.context("无法加载用户信息")?;

    // 运行应用
    app.run().await.context("应用运行失败")?;

    // 退出时保存对话到记忆系统
    if let Some(inf) = infrastructure {
        log::info!("正在保存对话到记忆系统...");
        app.save_conversations_to_memory().await.context("保存对话失败")?;
        log::info!("对话保存完成");
    }

    Ok(())
}
