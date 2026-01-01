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
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let enhance_memory_saver = args.contains(&"--enhance-memory-saver".to_string());

    if enhance_memory_saver {
        log::info!("已启用增强记忆保存功能");
    }

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

    // 退出时保存对话到记忆系统（仅在启用增强记忆保存功能时）
    if enhance_memory_saver {
        if let Some(_inf) = infrastructure {
            println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
            println!("║                            🧠 Cortex Memory - 退出流程                       ║");
            println!("╚══════════════════════════════════════════════════════════════════════════════╝");

            log::info!("🚀 开始退出流程，准备保存对话到记忆系统...");

            let conversations = app.get_conversations();
            let user_id = app.get_user_id();

            println!("📋 会话摘要:");
            println!("   • 对话轮次: {} 轮", conversations.len());
            println!("   • 用户ID: {}", user_id);

            if conversations.is_empty() {
                println!("⚠️ 没有需要存储的内容");
                println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
                println!("║                                    ✅ 退出流程完成                           ║");
                println!("╚══════════════════════════════════════════════════════════════════════════════╝");
                println!("👋 Cortex TARS powering down. Goodbye!");
                return Ok(());
            }

            println!("\n🧠 开始执行记忆化存储...");
            println!("📝 正在保存 {} 条对话记录到记忆库...", conversations.len());
            println!("🚀 开始存储对话到记忆系统...");

            match app.save_conversations_to_memory().await {
                Ok(_) => {
                    println!("✨ 记忆化完成！");
                    println!("✅ 所有对话已成功存储到记忆系统");
                    println!("🔍 存储详情:");
                    println!("   • 对话轮次: {} 轮", conversations.len());
                    println!("   • 用户消息: {} 条", conversations.len());
                    println!("   • 助手消息: {} 条", conversations.len());
                }
                Err(e) => {
                    println!("❌ 记忆存储失败: {}", e);
                    println!("⚠️ 虽然记忆化失败，但仍正常退出");
                }
            }

            println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
            println!("║                                  🎉 退出流程完成                             ║");
            println!("╚══════════════════════════════════════════════════════════════════════════════╝");
            println!("👋 Cortex TARS powering down. Goodbye!");
        } else {
            println!("\n⚠️ 基础设施未初始化，无法保存对话到记忆系统");
            println!("👋 Cortex TARS powering down. Goodbye!");
        }
    } else {
        log::info!("未启用增强记忆保存功能，跳过对话保存");
        println!("\n👋 Cortex TARS powering down. Goodbye!");
    }

    Ok(())
}
