use clap::Parser;
use crossterm::{
    event, execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use memo_config::Config;
use memo_core::init_logging;
use memo_rig::{
    llm::OpenAILLMClient, memory::manager::MemoryManager, vector_store::qdrant::QdrantVectorStore,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, sync::Arc};
use tokio::sync::mpsc;

mod agent;
mod app;
mod events;
mod terminal;
mod ui;

use agent::{agent_reply_with_memory_retrieval, create_memory_agent, extract_user_basic_info};
use app::{redirect_log_to_ui, set_global_log_sender, App, AppMessage, FocusArea};
use events::{handle_key_event, handle_quit, process_user_input};
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
                    // 先设置shutting_down状态，这样UI会立即更新
                    app.is_shutting_down = true;
                    
                    // 如果当前焦点在输入框，切换到对话区域
                    if app.focus_area == FocusArea::Input {
                        app.focus_area = FocusArea::Conversation;
                    }
                    
                    // 刷新UI，立即显示说明文案而不是输入框
                    terminal.draw(|f| draw_ui(f, &mut app))?;
                    
                    // 记录退出命令
                    redirect_log_to_ui("INFO", "用户输入退出命令 /quit");
                    
                    // 同步执行handle_quit，确保记忆化操作完成
                    let conversations_snapshot: Vec<(String, String)> = app.conversations.iter().cloned().collect();
                    let memory_manager_clone = memory_manager.clone();
                    let user_id_string = user_id.to_string();
                    
                    // 先刷新一次UI显示开始退出
                    terminal.draw(|f| draw_ui(f, &mut app))?;
                    
                    match handle_quit(conversations_snapshot, memory_manager_clone, &user_id_string).await {
                        Ok(completed) => {
                            if completed {
                                // 手动设置记忆化完成状态
                                app.memory_iteration_completed = true;
                                app.should_quit = true;
                                redirect_log_to_ui("INFO", "记忆化完成，准备退出...");
                            } else {
                                redirect_log_to_ui("WARN", "记忆化未完成，但仍然退出");
                                app.should_quit = true;
                            }
                        }
                        Err(e) => {
                            redirect_log_to_ui("ERROR", &format!("退出流程出错: {}", e));
                            redirect_log_to_ui("INFO", "出现错误，仍然准备退出...");
                            app.should_quit = true;
                        }
                    }
                    
                    // 刷新最终UI
                    terminal.draw(|f| draw_ui(f, &mut app))?;
                    
                    // 短暂停留让用户看到最后的日志
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
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
                        app.conversations.iter().cloned().collect();

                    // 记录开始处理
                    redirect_log_to_ui("INFO", "开始处理用户请求...");

                    tokio::spawn(async move {
                        // 记录开始处理
                        redirect_log_to_ui("DEBUG", "正在检索相关记忆...");
                        
                        // Agent生成回复（带记忆检索和利用）
                        match agent_reply_with_memory_retrieval(
                            &agent_clone,
                            memory_manager_clone.clone(),
                            &config_clone,
                            &input,
                            &user_id_clone,
                            user_info_clone.as_deref(),
                            &current_conversations,
                        )
                        .await
                        {
                            Ok(response) => {
                                // 发送对话到主线程
                                if let Some(sender) = &msg_tx_clone {
                                    let _ = sender.send(AppMessage::Conversation {
                                        user: input.clone(),
                                        assistant: response.clone(),
                                    });
                                    redirect_log_to_ui("INFO", &format!("生成回复: {}", response));
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("抱歉，我遇到了一些技术问题: {}", e);
                                redirect_log_to_ui("ERROR", &error_msg);
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

    Ok(())
}
