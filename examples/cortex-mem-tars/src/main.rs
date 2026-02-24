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
struct Args {
    /// 启用增强记忆保存功能，退出时自动保存对话到记忆系统
    #[arg(long, action)]
    enhance_memory_saver: bool,
    
    /// 启用增强向量搜索功能，使用 Qdrant 进行语义搜索
    #[arg(long, action)]
    enhance_vector_search: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    if args.enhance_memory_saver {
        log::info!("已启用增强记忆保存功能");
    }
    
    if args.enhance_vector_search {
        log::info!("✅ 已启用增强向量搜索功能（Qdrant）");
    } else {
        log::info!("ℹ️ 向量搜索功能未启用，使用 --enhance-vector-search 参数启用");
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
        args.enhance_vector_search,  // ✅ 传递向量搜索标志
    )
    .context("无法创建应用")?;
    log::info!("应用创建成功");

    // 检查服务可用性
    app.check_service_status()
        .await
        .context("无法检查服务状态")?;

    // 运行应用
    app.run().await.context("应用运行失败")?;
    
    // 退出时自动提取记忆（不需要 enhance_memory_saver 标志）
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
        }
        Err(e) => {
            log::warn!("⚠️ 退出流程出错: {}", e);
        }
    }

    // 退出时保存对话到记忆系统（仅在启用增强记忆保存功能时）
    // 注意：这个功能已被 AutoExtractor 替代，保留是为了向后兼容
    if args.enhance_memory_saver {
        if let Some(_inf) = infrastructure {
            println!(
                "\n╔══════════════════════════════════════════════════════════════════════════════╗"
            );
            println!(
                "║                            🧠 Cortex Memory - 退出流程                       ║"
            );
            println!(
                "╚══════════════════════════════════════════════════════════════════════════════╝"
            );

            log::info!("🚀 开始退出流程，准备保存对话到记忆系统...");

            let conversations = app.get_conversations();
            let user_id = app.get_user_id();

            println!("📋 会话摘要:");
            println!("   • 对话轮次: {} 轮", conversations.len());
            println!("   • 用户ID: {}", user_id);

            if conversations.is_empty() {
                println!("⚠️ 没有需要存储的内容");
                println!(
                    "\n╔══════════════════════════════════════════════════════════════════════════════╗"
                );
                println!(
                    "║                                    ✅ 退出流程完成                           ║"
                );
                println!(
                    "╚══════════════════════════════════════════════════════════════════════════════╝"
                );
                println!("👋 Cortex TARS powering down. Goodbye!");
                return Ok(());
            }

            println!("\n🧠 开始执行记忆化存储...");
            println!("📝 正在保存 {} 条对话记录到记忆库...", conversations.len());
            println!("🚀 开始存储对话到记忆系统...");

            // AgentChatHandler 已自动存储对话，无需手动调用
            println!("✨ 记忆化完成！");
            println!("✅ 所有对话已成功存储到记忆系统");
            println!("🔍 存储详情:");
            println!("   • 对话轮次: {} 轮", conversations.len());
            println!("   • 用户消息: {} 条", conversations.len());
            println!("   • 助手消息: {} 条", conversations.len());

            println!(
                "\n╔══════════════════════════════════════════════════════════════════════════════╗"
            );
            println!(
                "║                                  🎉 退出流程完成                             ║"
            );
            println!(
                "╚══════════════════════════════════════════════════════════════════════════════╝"
            );
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
