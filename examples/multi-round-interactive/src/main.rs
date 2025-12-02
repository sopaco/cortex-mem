use clap::Parser;
use crossterm::{
    event, execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use memo_config::Config;
use memo_core::init_logging;
use memo_rig::{
    llm::OpenAILLMClient, memory::manager::MemoryManager, vector_store::qdrant::QdrantVectorStore,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, path::PathBuf, sync::Arc};
use tokio::sync::mpsc;
use tokio::time::Duration;

mod agent;
mod app;
mod events;
mod log_monitor;
mod terminal;
mod ui;

use agent::{
    agent_reply_with_memory_retrieval_streaming, create_memory_agent, extract_user_basic_info,
    store_conversations_batch,
};
use app::{App, AppMessage, redirect_log_to_ui, set_global_log_sender};
use events::{handle_key_event, process_user_input};
use log_monitor::start_log_monitoring_task;
use terminal::cleanup_terminal_final;
use ui::draw_ui;

#[derive(Parser)]
#[command(name = "multi-round-interactive")]
#[command(about = "Multi-round interactive conversation with a memory-enabled agent")]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载基本配置以获取日志设置
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    // 初始化日志系统
    init_logging(&config.logging)?;

    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_application(&mut terminal).await;

    // 最终清理 - 使用最彻底的方法
    cleanup_terminal_final(&mut terminal);

    result
}

/// 主应用逻辑
async fn run_application(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建消息通道
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<AppMessage>();

    // 使用我们的自定义日志系统，禁用tracing
    // tracing_subscriber::fmt::init();

    // 设置全局日志发送器以便我们的日志系统正常工作
    set_global_log_sender(msg_tx.clone());

    // 初始化组件
    // 配置加载已经在main函数中完成，这里只获取文件路径
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    let llm_client = OpenAILLMClient::new(&config.llm, &config.embedding)?;
    let vector_store = QdrantVectorStore::new(&config.qdrant)
        .await
        .expect("无法连接到Qdrant");

    let memory_config = config.memory.clone();
    let memory_manager = Arc::new(MemoryManager::new(
        Box::new(vector_store),
        Box::new(llm_client.clone()),
        memory_config,
    ));

    // 创建带记忆的Agent
    let memory_tool_config = memo_rig::tool::MemoryToolConfig {
        default_user_id: Some("demo_user".to_string()),
        ..Default::default()
    };

    let agent = create_memory_agent(memory_manager.clone(), memory_tool_config, &config).await?;

    // 初始化用户信息
    let user_id = "demo_user";
    let user_info = extract_user_basic_info(&config, memory_manager.clone(), user_id).await?;

    // 创建应用状态
    let mut app = App::new(msg_tx);

    if let Some(info) = user_info {
        app.user_info = Some(info.clone());
        app.log_info("已加载用户基本信息");
    } else {
        app.log_info("未找到用户基本信息");
    }

    app.log_info("初始化完成，开始对话...");

    // 主事件循环
    loop {
        // 更新消息（包括在quit过程中收到的所有消息）
        while let Ok(msg) = msg_rx.try_recv() {
            match msg {
                AppMessage::Log(log_msg) => {
                    app.add_log(log_msg);
                }
                AppMessage::Conversation { user, assistant } => {
                    app.add_conversation(user, assistant);
                }
                AppMessage::StreamingChunk { user, chunk } => {
                    // 如果是新的用户输入，开始新的流式回复
                    if app.current_streaming_response.is_none() || 
                       app.current_streaming_response.as_ref().map(|(u, _)| u != &user).unwrap_or(false) {
                        app.start_streaming_response(user);
                    }
                    app.add_streaming_chunk(chunk);
                }
                AppMessage::StreamingComplete { user: _, full_response: _ } => {
                    app.complete_streaming_response();
                }
                AppMessage::MemoryIterationCompleted => {
                    app.memory_iteration_completed = true;
                    app.should_quit = true;
                }
            }
        }

        // 绘制UI
        terminal.draw(|f| draw_ui(f, &mut app))?;

        // 处理事件
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Some(input) = handle_key_event(event::read()?, &mut app) {
                // 先检查是否是quit命令
                let is_quit = process_user_input(input.clone(), &mut app);

                // 如果是quit命令，先添加到对话历史
                if is_quit {
                    app.add_conversation(input.clone(), "正在执行退出命令...".to_string());
                }

                if is_quit {
                    // 立即退出到terminal，后台执行记忆化任务
                    let conversations_vec: Vec<(String, String)> =
                        app.conversations.iter().map(|(user, assistant, _)| (user.clone(), assistant.clone())).collect();
                    handle_quit_async(
                        terminal,
                        &mut app,
                        &conversations_vec,
                        &memory_manager,
                        user_id,
                    )
                    .await?;

                    // 退出主循环
                    break;
                } else {
                    // 记录用户输入
                    redirect_log_to_ui("INFO", &format!("接收用户输入: {}", input));

                    // 处理用户输入
                    let agent_clone = agent.clone();
                    let memory_manager_clone = memory_manager.clone();
                    let config_clone = config.clone();
                    let user_info_clone = app.user_info.clone();
                    let user_id_clone = user_id.to_string();
                    let msg_tx_clone = app.message_sender.clone();

                    // 获取当前对话历史的引用（转换为slice）
                    let current_conversations: Vec<(String, String)> =
                        app.conversations.iter().map(|(user, assistant, _)| (user.clone(), assistant.clone())).collect();

                    // 记录开始处理
                    redirect_log_to_ui("INFO", "开始处理用户请求...");

                    tokio::spawn(async move {
                        // 创建流式通道
                        let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();
                        
                        // 启动流式处理任务
                        let agent_clone2 = agent_clone.clone();
                        let memory_manager_clone2 = memory_manager_clone.clone();
                        let config_clone2 = config_clone.clone();
                        let user_info_clone2 = user_info_clone.clone();
                        let user_id_clone2 = user_id_clone.clone();
                        let input_clone = input.clone();
                        let current_conversations_clone = current_conversations.clone();
                        
                        let generation_task = tokio::spawn(async move {
                            agent_reply_with_memory_retrieval_streaming(
                                &agent_clone2,
                                memory_manager_clone2,
                                &input_clone,
                                &user_id_clone2,
                                user_info_clone2.as_deref(),
                                &current_conversations_clone,
                                stream_tx,
                            )
                            .await
                        });

                        // 处理流式内容
                        while let Some(chunk) = stream_rx.recv().await {
                            if let Some(sender) = &msg_tx_clone {
                                let _ = sender.send(AppMessage::StreamingChunk {
                                    user: input.clone(),
                                    chunk,
                                });
                            }
                        }

                        // 等待生成任务完成
                        match generation_task.await {
                            Ok(Ok(full_response)) => {
                                // 发送完成消息
                                if let Some(sender) = &msg_tx_clone {
                                    let _ = sender.send(AppMessage::StreamingComplete {
                                        user: input.clone(),
                                        full_response: full_response.clone(),
                                    });
                                    redirect_log_to_ui("INFO", &format!("生成回复完成: {}", full_response));
                                }
                            }
                            Ok(Err(e)) => {
                                let error_msg = format!("抱歉，我遇到了一些技术问题: {}", e);
                                redirect_log_to_ui("ERROR", &error_msg);
                                // 完成流式回复（即使出错也要清理状态）
                                if let Some(sender) = &msg_tx_clone {
                                    let _ = sender.send(AppMessage::StreamingComplete {
                                        user: input.clone(),
                                        full_response: error_msg,
                                    });
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("任务执行失败: {}", e);
                                redirect_log_to_ui("ERROR", &error_msg);
                                // 完成流式回复（即使出错也要清理状态）
                                if let Some(sender) = &msg_tx_clone {
                                    let _ = sender.send(AppMessage::StreamingComplete {
                                        user: input.clone(),
                                        full_response: error_msg,
                                    });
                                }
                            }
                        }
                    });
                }
            }
        }

        // 检查是否有新的对话结果
        app.is_processing = false;

        // 只有在没有在shutting down状态或者记忆化已完成时才能退出
        if app.should_quit && app.memory_iteration_completed {
            break;
        }

        // **在quit过程中处理剩余的日志消息但不退出**
        if app.is_shutting_down && !app.memory_iteration_completed {
            // **立即处理所有待处理的日志消息**
            while let Ok(msg) = msg_rx.try_recv() {
                match msg {
                    AppMessage::Log(log_msg) => {
                        app.add_log(log_msg);
                    }
                    AppMessage::Conversation { user, assistant } => {
                        app.add_conversation(user, assistant);
                    }
                    AppMessage::StreamingChunk { user, chunk } => {
                        // 如果是新的用户输入，开始新的流式回复
                        if app.current_streaming_response.is_none() || 
                           app.current_streaming_response.as_ref().map(|(u, _)| u != &user).unwrap_or(false) {
                            app.start_streaming_response(user);
                        }
                        app.add_streaming_chunk(chunk);
                    }
                    AppMessage::StreamingComplete { user: _, full_response: _ } => {
                        app.complete_streaming_response();
                    }
                    AppMessage::MemoryIterationCompleted => {
                        app.memory_iteration_completed = true;
                        app.should_quit = true;
                        break;
                    }
                }
            }

            // 在shutting down期间立即刷新UI显示最新日志
            if let Err(e) = terminal.draw(|f| draw_ui(f, &mut app)) {
                eprintln!("UI绘制错误: {}", e);
            }

            // 在shutting down期间添加短暂延迟，让用户能看到日志更新
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    println!("Cortex TARS powering down. Goodbye!");
    Ok(())
}

/// 异步处理退出逻辑，立即退出TUI到terminal
async fn handle_quit_async(
    _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    conversations: &Vec<(String, String)>,
    memory_manager: &Arc<MemoryManager>,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::cursor::{MoveTo, Show};
    use crossterm::style::{
        Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    };
    use crossterm::{
        event::DisableMouseCapture,
        execute,
        terminal::{Clear, ClearType, LeaveAlternateScreen},
    };
    use std::io::{Write, stdout};

    // 记录退出命令到UI
    redirect_log_to_ui("INFO", "🚀 用户输入退出命令 /quit，开始后台记忆化...");

    // 先获取所有日志内容
    let all_logs: Vec<String> = app.logs.iter().cloned().collect();

    // 彻底清理terminal状态
    let mut stdout = stdout();

    // 执行完整的terminal重置序列
    execute!(&mut stdout, ResetColor)?;
    execute!(&mut stdout, Clear(ClearType::All))?;
    execute!(&mut stdout, MoveTo(0, 0))?;
    execute!(&mut stdout, Show)?;
    execute!(&mut stdout, LeaveAlternateScreen)?;
    execute!(&mut stdout, DisableMouseCapture)?;
    execute!(&mut stdout, SetAttribute(Attribute::Reset))?;
    execute!(&mut stdout, SetForegroundColor(Color::Reset))?;
    execute!(&mut stdout, SetBackgroundColor(Color::Reset))?;

    // 禁用原始模式
    let _ = crossterm::terminal::disable_raw_mode();

    // 刷新输出确保清理完成
    stdout.flush()?;

    // 输出分隔线
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                            🧠 Cortex Memory - 退出流程                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");

    // 显示会话摘要
    println!("📋 会话摘要:");
    println!("   • 对话轮次: {} 轮", conversations.len());
    println!("   • 用户ID: {}", user_id);

    // 显示最近的日志（如果有）
    if !all_logs.is_empty() {
        println!("\n📜 最近的操作日志:");
        let recent_logs = if all_logs.len() > 10 {
            &all_logs[all_logs.len() - 10..]
        } else {
            &all_logs[..]
        };

        println!("   {}", "─".repeat(70));
        for (i, log) in recent_logs.iter().enumerate() {
            let beautified_content = beautify_log_content(log);

            // 添加日志条目编号
            if i > 0 {
                println!("   {}", "─".repeat(70));
            }

            // 显示美化后的内容，支持多行显示
            let lines: Vec<&str> = beautified_content.split('\n').collect();
            for (line_i, line) in lines.iter().enumerate() {
                if line_i == 0 {
                    // 第一行显示编号和完整内容
                    let colored_line = get_log_level_color(log, line);
                    println!("   {}", colored_line);
                } else {
                    // 后续行添加缩进
                    println!("   │ {}", line);
                }
            }
        }
        if all_logs.len() > 10 {
            println!("   {}", "─".repeat(70));
            println!("   ... (显示最近10条，共{}条)", all_logs.len());
        }
    }

    println!("\n🧠 开始执行记忆化存储...");

    // 准备对话数据（过滤quit命令）
    let mut valid_conversations = Vec::new();
    for (user_msg, assistant_msg) in conversations {
        let user_msg_trimmed = user_msg.trim().to_lowercase();
        if user_msg_trimmed == "quit"
            || user_msg_trimmed == "exit"
            || user_msg_trimmed == "/quit"
            || user_msg_trimmed == "/exit"
        {
            continue;
        }
        valid_conversations.push((user_msg.clone(), assistant_msg.clone()));
    }

    if valid_conversations.is_empty() {
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
        println!("👋 感谢使用Cortex Memory！");
        return Ok(());
    }

    // 只有在有内容需要存储时才启动日志监听任务
    let log_dir = "logs".to_string();
    let log_monitoring_handle = tokio::spawn(async move {
        if let Err(e) = start_log_monitoring_task(log_dir).await {
            eprintln!("日志监听任务失败: {}", e);
        }
    });

    println!(
        "📝 正在保存 {} 条对话记录到记忆库...",
        valid_conversations.len()
    );
    println!("🚀 开始存储对话到记忆系统...");

    // 执行批量记忆化
    match store_conversations_batch(memory_manager.clone(), &valid_conversations, user_id).await {
        Ok(_) => {
            println!("✨ 记忆化完成！");
            println!("✅ 所有对话已成功存储到记忆系统");
            println!("🔍 存储详情:");
            println!("   • 对话轮次: {} 轮", valid_conversations.len());
            println!("   • 用户消息: {} 条", valid_conversations.len());
            println!("   • 助手消息: {} 条", valid_conversations.len());
        }
        Err(e) => {
            println!("❌ 记忆存储失败: {}", e);
            println!("⚠️ 虽然记忆化失败，但仍正常退出");
        }
    }

    // 停止日志监听任务
    log_monitoring_handle.abort();

    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                  🎉 退出流程完成                             ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!("👋 感谢使用Cortex Memory！");

    Ok(())
}

/// 美化日志内容显示
fn beautify_log_content(log_line: &str) -> String {
    // 过滤掉时间戳前缀，保持简洁
    let content = if let Some(content_start) = log_line.find("] ") {
        &log_line[content_start + 2..]
    } else {
        log_line
    };

    // 判断是否为JSON内容
    let trimmed_content = content.trim();
    let is_json = trimmed_content.starts_with('{') && trimmed_content.ends_with('}');

    if is_json {
        // 尝试美化JSON，保留完整内容
        match prettify_json(trimmed_content) {
            Ok(formatted_json) => {
                // 如果格式化成功，返回完整的带缩进的JSON
                formatted_json
            }
            Err(_) => {
                // 如果JSON格式化失败，返回原始内容
                content.to_string()
            }
        }
    } else {
        // 非JSON内容，保持原样
        content.to_string()
    }
}

/// 美化JSON内容
fn prettify_json(json_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    use serde_json::Value;

    let value: Value = serde_json::from_str(json_str)?;
    Ok(serde_json::to_string_pretty(&value)?)
}

/// 根据日志级别返回带颜色的文本
fn get_log_level_color(log_line: &str, text: &str) -> String {
    let log_level = if let Some(level_start) = log_line.find("[") {
        if let Some(level_end) = log_line[level_start..].find("]") {
            &log_line[level_start + 1..level_start + level_end]
        } else {
            "UNKNOWN"
        }
    } else {
        "UNKNOWN"
    };

    // ANSI颜色代码
    let (color_code, reset_code) = match log_level.to_uppercase().as_str() {
        "ERROR" => ("\x1b[91m", "\x1b[0m"),            // 亮红色
        "WARN" | "WARNING" => ("\x1b[93m", "\x1b[0m"), // 亮黄色
        "INFO" => ("\x1b[36m", "\x1b[0m"),             // 亮青色
        "DEBUG" => ("\x1b[94m", "\x1b[0m"),            // 亮蓝色
        "TRACE" => ("\x1b[95m", "\x1b[0m"),            // 亮紫色
        _ => ("\x1b[0m", "\x1b[0m"),                   // 白色
    };

    format!("{}{}{}", color_code, text, reset_code)
}
