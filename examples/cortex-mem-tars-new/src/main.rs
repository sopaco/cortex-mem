mod agent;
mod app;
mod config;
mod logger;
mod ui;

use anyhow::{Context, Result};
use app::{create_default_bots, App};
use config::ConfigManager;
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

    // 创建并运行应用
    let mut app = App::new(config_manager, log_manager).context("无法创建应用")?;
    log::info!("应用创建成功");

    app.run().await.context("应用运行失败")?;

    Ok(())
}
