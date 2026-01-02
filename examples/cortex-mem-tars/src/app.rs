use crate::agent::{ChatMessage, create_memory_agent, extract_user_basic_info, store_conversations_batch, agent_reply_with_memory_retrieval_streaming};
use crate::config::{BotConfig, ConfigManager};
use crate::infrastructure::Infrastructure;
use crate::logger::LogManager;
use crate::ui::{AppState, AppUi};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use rig::agent::Agent as RigAgent;
use rig::providers::openai::CompletionModel;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// 应用程序
pub struct App {
    #[allow(dead_code)]
    config_manager: ConfigManager,
    log_manager: Arc<LogManager>,
    ui: AppUi,
    current_bot: Option<BotConfig>,
    rig_agent: Option<RigAgent<CompletionModel>>,
    infrastructure: Option<Arc<Infrastructure>>,
    user_id: String,
    user_info: Option<String>,
    should_quit: bool,
    message_sender: mpsc::UnboundedSender<AppMessage>,
    message_receiver: mpsc::UnboundedReceiver<AppMessage>,
}

/// 应用消息类型
#[derive(Debug, Clone)]
pub enum AppMessage {
    #[allow(dead_code)]
    Log(String),
    StreamingChunk {
        #[allow(dead_code)]
        user: String,
        chunk: String,
    },
    StreamingComplete {
        #[allow(dead_code)]
        user: String,
        full_response: String,
    },
}

impl App {
    /// 创建新的应用
    pub fn new(config_manager: ConfigManager, log_manager: Arc<LogManager>, infrastructure: Option<Arc<Infrastructure>>) -> Result<Self> {
        let mut ui = AppUi::new();

        // 加载机器人列表
        let bots = config_manager.get_bots()?;
        ui.set_bot_list(bots);

        // 创建消息通道
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<AppMessage>();

        log::info!("应用程序初始化完成");

        Ok(Self {
            config_manager,
            log_manager,
            ui,
            current_bot: None,
            rig_agent: None,
            infrastructure,
            user_id: "tars_user".to_string(),
            user_info: None,
            should_quit: false,
            message_sender: msg_tx,
            message_receiver: msg_rx,
        })
    }

    /// 设置用户信息
    pub async fn load_user_info(&mut self) -> Result<()> {
        if let Some(infrastructure) = &self.infrastructure {
            let user_info = extract_user_basic_info(
                infrastructure.config(),
                infrastructure.memory_manager().clone(),
                &self.user_id,
            ).await.map_err(|e| anyhow::anyhow!("加载用户信息失败: {}", e))?;

            if let Some(info) = user_info {
                log::info!("已加载用户基本信息");
                self.user_info = Some(info);
            } else {
                log::info!("未找到用户基本信息");
            }
        }
        Ok(())
    }

    /// 检查服务可用性
    pub async fn check_service_status(&mut self) -> Result<()> {
        use reqwest::Method;

        if let Some(infrastructure) = &self.infrastructure {
            let api_base_url = &infrastructure.config().llm.api_base_url;
            // 拼接完整的 API 地址
            let check_url = format!("{}/chat/completions", api_base_url.trim_end_matches('/'));

            log::info!("检查服务可用性: {}", check_url);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .context("无法创建 HTTP 客户端")?;

            match client
                .request(Method::OPTIONS, &check_url)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() || response.status().as_u16() == 405 {
                        // 200 OK 或 405 Method Not Allowed 都表示服务可用
                        log::info!("服务可用，状态码: {}", response.status());
                        self.ui.service_status = crate::ui::ServiceStatus::Active;
                    } else {
                        log::warn!("服务不可用，状态码: {}", response.status());
                        self.ui.service_status = crate::ui::ServiceStatus::Inactive;
                    }
                }
                Err(e) => {
                    log::error!("服务检查失败: {}", e);
                    self.ui.service_status = crate::ui::ServiceStatus::Inactive;
                }
            }
        } else {
            log::warn!("基础设施未初始化，无法检查服务状态");
            self.ui.service_status = crate::ui::ServiceStatus::Inactive;
        }

        Ok(())
    }

    /// 运行应用
    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode().context("无法启用原始模式")?;

        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            crossterm::terminal::DisableLineWrap
        )
        .context("无法设置终端")?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = ratatui::Terminal::new(backend).context("无法创建终端")?;

        let mut last_log_update = Instant::now();
        let mut last_service_check = Instant::now();
        let tick_rate = Duration::from_millis(100);

        loop {
            // 更新日志
            if last_log_update.elapsed() > Duration::from_secs(1) {
                self.update_logs();
                last_log_update = Instant::now();
            }

            // 定期检查服务状态（每5秒）
            if last_service_check.elapsed() > Duration::from_secs(5) {
                // 在后台检查服务状态，不阻塞主循环
                let _ = self.check_service_status().await;
                last_service_check = Instant::now();
            }

            // 处理流式消息
            if let Ok(msg) = self.message_receiver.try_recv() {
                match msg {
                    AppMessage::StreamingChunk { user: _, chunk } => {
                        // 添加流式内容到当前正在生成的消息
                        if let Some(last_msg) = self.ui.messages.last_mut() {
                            if last_msg.role == crate::agent::MessageRole::Assistant {
                                last_msg.content.push_str(&chunk);
                            } else {
                                // 如果最后一条不是助手消息，创建新的助手消息
                                self.ui.messages.push(ChatMessage::assistant(chunk));
                            }
                        } else {
                            // 如果没有消息，创建新的助手消息
                            self.ui.messages.push(ChatMessage::assistant(chunk));
                        }
                        // 确保自动滚动启用
                        self.ui.auto_scroll = true;
                    }
                    AppMessage::StreamingComplete { user: _, full_response } => {
                        // 流式完成，确保完整响应已保存
                        if let Some(last_msg) = self.ui.messages.last_mut() {
                            if last_msg.role == crate::agent::MessageRole::Assistant {
                                last_msg.content = full_response;
                            } else {
                                self.ui.messages.push(ChatMessage::assistant(full_response));
                            }
                        } else {
                            self.ui.messages.push(ChatMessage::assistant(full_response));
                        }
                        // 确保自动滚动启用
                        self.ui.auto_scroll = true;
                    }
                    AppMessage::Log(_) => {
                        // 日志消息暂时忽略
                    }
                }
            }

            // 渲染 UI
            terminal.draw(|f| self.ui.render(f)).context("渲染失败")?;

            // 处理事件
            if event::poll(tick_rate).context("事件轮询失败")? {
                let event = event::read().context("读取事件失败")?;
                log::trace!("收到事件: {:?}", event);

                match event {
                    Event::Key(key) => {
                        let action = self.ui.handle_key_event(key);

                        match action {
                            crate::ui::KeyAction::Quit => {
                                self.should_quit = true;
                                break;
                            }
                            crate::ui::KeyAction::SendMessage => {
                                if self.ui.state == AppState::Chat {
                                    self.send_message().await?;
                                }
                            }
                            crate::ui::KeyAction::ClearChat => {
                                if self.ui.state == AppState::Chat {
                                    self.clear_chat();
                                }
                            }
                            crate::ui::KeyAction::ShowHelp => {
                                if self.ui.state == AppState::Chat {
                                    self.show_help();
                                }
                            }
                            crate::ui::KeyAction::ShowThemes => {
                                log::info!("收到 ShowThemes 动作，当前状态: {:?}", self.ui.state);
                                if self.ui.state == AppState::Chat {
                                    log::info!("调用 show_themes()");
                                    self.show_themes();
                                    log::info!("show_themes() 调用完成，theme_modal_visible: {}", self.ui.theme_modal_visible);
                                } else {
                                    log::warn!("不在 Chat 状态，无法显示主题");
                                }
                            }
                            crate::ui::KeyAction::DumpChats => {
                                if self.ui.state == AppState::Chat {
                                    self.dump_chats();
                                }
                            }
                            crate::ui::KeyAction::Continue => {}
                        }
                    }
                    Event::Mouse(mouse) => {
                        let size = terminal.size()?;
                        self.ui
                            .handle_mouse_event(mouse, Rect::new(0, 0, size.width, size.height));
                    }
                    _ => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        disable_raw_mode().context("无法禁用原始模式")?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .context("无法恢复终端")?;

        terminal.show_cursor().context("无法显示光标")?;

        log::info!("应用程序退出");
        Ok(())
    }

    /// 更新日志
    fn update_logs(&mut self) {
        match self.log_manager.read_logs(1000) {
            Ok(logs) => {
                self.ui.log_lines = logs;
            }
            Err(e) => {
                log::error!("读取日志失败: {}", e);
            }
        }
    }

    /// 发送消息
    async fn send_message(&mut self) -> Result<()> {
        let input_text = self.ui.get_input_text();
        let input_text = input_text.trim();

        log::debug!("准备发送消息，长度: {}", input_text.len());

        if input_text.is_empty() {
            log::debug!("消息为空，忽略");
            return Ok(());
        }

        // 检查是否是命令
        if let Some(command_action) = self.ui.parse_and_execute_command(input_text) {
            self.ui.clear_input();

            match command_action {
                crate::ui::KeyAction::Quit => {
                    self.should_quit = true;
                }
                crate::ui::KeyAction::ClearChat => {
                    self.clear_chat();
                }
                crate::ui::KeyAction::ShowHelp => {
                    self.show_help();
                }
                crate::ui::KeyAction::ShowThemes => {
                    self.show_themes();
                }
                crate::ui::KeyAction::DumpChats => {
                    self.dump_chats();
                }
                _ => {}
            }
            return Ok(());
        }

        // 检查是否刚进入聊天模式
        if self.current_bot.is_none() {
            if let Some(bot) = self.ui.selected_bot() {
                self.current_bot = Some(bot.clone());

                // 如果有基础设施，创建真实的带记忆的 Agent
                if let Some(infrastructure) = &self.infrastructure {
                    // 先提取用户基本信息
                    let user_info = match extract_user_basic_info(
                        infrastructure.config(),
                        infrastructure.memory_manager().clone(),
                        &self.user_id,
                    ).await {
                        Ok(info) => {
                            self.user_info = info.clone();
                            info
                        }
                        Err(e) => {
                            log::error!("提取用户基本信息失败: {}", e);
                            None
                        }
                    };

                    let memory_tool_config = cortex_mem_rig::tool::MemoryToolConfig {
                        default_user_id: Some(self.user_id.clone()),
                        ..Default::default()
                    };

                    match create_memory_agent(
                        infrastructure.memory_manager().clone(),
                        memory_tool_config,
                        infrastructure.config(),
                        user_info.as_deref(),
                    ).await {
                        Ok(rig_agent) => {
                            self.rig_agent = Some(rig_agent);
                            log::info!("已创建带记忆功能的真实 Agent");
                        }
                        Err(e) => {
                            log::error!("创建真实 Agent 失败，使用 Mock Agent: {}", e);
                        }
                    }
                }

                log::info!("选择机器人: {}", bot.name);
            } else {
                log::warn!("没有选中的机器人");
                return Ok(());
            }
        }

        // 添加用户消息
        let user_message = ChatMessage::user(input_text);
        self.ui.messages.push(user_message.clone());
        self.ui.clear_input();

        // 用户发送新消息，重新启用自动滚动
        self.ui.auto_scroll = true;

        log::info!("用户发送消息: {}", input_text);
        log::debug!("当前消息总数: {}", self.ui.messages.len());

        // 使用真实的带记忆的 Agent 或 Mock Agent
        if let Some(rig_agent) = &self.rig_agent {
            // 使用真实 Agent 进行流式响应
            // 构建历史对话（排除当前用户输入）
            let current_conversations: Vec<(String, String)> = {
                let mut conversations = Vec::new();
                let mut last_user_msg: Option<String> = None;

                // 遍历所有消息，但排除最后一条（当前用户输入）
                let messages_to_include = if self.ui.messages.len() > 1 {
                    &self.ui.messages[..self.ui.messages.len() - 1]
                } else {
                    &[]
                };

                for msg in messages_to_include {
                    match msg.role {
                        crate::agent::MessageRole::User => {
                            // 如果有未配对的 User 消息，先保存它（单独的 User 消息）
                            if let Some(user_msg) = last_user_msg.take() {
                                conversations.push((user_msg, String::new()));
                            }
                            last_user_msg = Some(msg.content.clone());
                        }
                        crate::agent::MessageRole::Assistant => {
                            // 将 Assistant 消息与最近的 User 消息配对
                            if let Some(user_msg) = last_user_msg.take() {
                                conversations.push((user_msg, msg.content.clone()));
                            }
                        }
                        _ => {}
                    }
                }

                // 如果最后一个消息是 User 消息，也加入对话历史
                if let Some(user_msg) = last_user_msg {
                    conversations.push((user_msg, String::new()));
                }

                conversations
            };

            let infrastructure_clone = self.infrastructure.clone();
            let rig_agent_clone = rig_agent.clone();
            let msg_tx = self.message_sender.clone();
            let user_input = input_text.to_string();
            let user_id = self.user_id.clone();
            let user_input_for_stream = user_input.clone();

            tokio::spawn(async move {
                let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();

                let generation_task = tokio::spawn(async move {
                    agent_reply_with_memory_retrieval_streaming(
                        &rig_agent_clone,
                        infrastructure_clone.unwrap().memory_manager().clone(),
                        &user_input,
                        &user_id,
                        &current_conversations,
                        stream_tx,
                    ).await
                });

                while let Some(chunk) = stream_rx.recv().await {
                    if let Err(_) = msg_tx.send(AppMessage::StreamingChunk {
                        user: user_input_for_stream.clone(),
                        chunk,
                    }) {
                        break;
                    }
                }

                match generation_task.await {
                    Ok(Ok(full_response)) => {
                        let _ = msg_tx.send(AppMessage::StreamingComplete {
                            user: user_input_for_stream.clone(),
                            full_response,
                        });
                    }
                    Ok(Err(e)) => {
                        log::error!("生成回复失败: {}", e);
                    }
                    Err(e) => {
                        log::error!("任务执行失败: {}", e);
                    }
                }
            });

        } else {
            log::warn!("Agent 未初始化");
        }

        // 滚动到底部 - 将在渲染时自动计算
        self.ui.auto_scroll = true;

        Ok(())
    }

    /// 清空会话
    fn clear_chat(&mut self) {
        log::info!("清空会话");
        self.ui.messages.clear();
        self.ui.scroll_offset = 0;
        self.ui.auto_scroll = true;
    }

    /// 显示帮助信息
    fn show_help(&mut self) {
        log::info!("显示帮助信息");
        self.ui.help_modal_visible = true;
        self.ui.help_scroll_offset = 0;
    }

    /// 显示主题选择
    fn show_themes(&mut self) {
        log::info!("显示主题选择");
        self.ui.theme_modal_visible = true;
        log::info!("主题弹窗可见性已设置为: {}", self.ui.theme_modal_visible);
    }

    /// 导出会话到剪贴板
    fn dump_chats(&mut self) {
        match self.ui.dump_chats_to_clipboard() {
            Ok(msg) => {
                log::info!("{}", msg);
                let success_message = ChatMessage::assistant(msg);
                self.ui.messages.push(success_message);
            }
            Err(e) => {
                log::error!("{}", e);
                let error_message = ChatMessage::assistant(format!("❌ {}", e));
                self.ui.messages.push(error_message);
            }
        }
        self.ui.auto_scroll = true;
    }

    /// 退出时保存对话到记忆系统
    pub async fn save_conversations_to_memory(&self) -> Result<()> {
        if let Some(infrastructure) = &self.infrastructure {
            let conversations: Vec<(String, String)> = {
                let mut conversations = Vec::new();
                let mut last_user_msg: Option<String> = None;

                for msg in &self.ui.messages {
                    match msg.role {
                        crate::agent::MessageRole::User => {
                            // 如果有未配对的 User 消息，先保存它（单独的 User 消息）
                            if let Some(user_msg) = last_user_msg.take() {
                                conversations.push((user_msg, String::new()));
                            }
                            last_user_msg = Some(msg.content.clone());
                        }
                        crate::agent::MessageRole::Assistant => {
                            // 将 Assistant 消息与最近的 User 消息配对
                            if let Some(user_msg) = last_user_msg.take() {
                                conversations.push((user_msg, msg.content.clone()));
                            }
                        }
                        _ => {}
                    }
                }

                // 如果最后一个消息是 User 消息，也加入对话历史
                if let Some(user_msg) = last_user_msg {
                    conversations.push((user_msg, String::new()));
                }

                conversations
            };

            // 只保存完整的对话对（用户和助手都有内容）
            let conversations: Vec<(String, String)> = conversations
                .into_iter()
                .filter(|(user, assistant)| !user.is_empty() && !assistant.is_empty())
                .collect();

            if !conversations.is_empty() {
                log::info!("正在保存 {} 条对话到记忆系统...", conversations.len());
                store_conversations_batch(
                    infrastructure.memory_manager().clone(),
                    &conversations,
                    &self.user_id,
                ).await.map_err(|e| anyhow::anyhow!("保存对话到记忆系统失败: {}", e))?;
                log::info!("对话保存完成");
            }
        }
        Ok(())
    }

    /// 获取所有对话
    pub fn get_conversations(&self) -> Vec<(String, String)> {
        self.ui.messages
            .iter()
            .filter_map(|msg| match msg.role {
                crate::agent::MessageRole::User => Some((msg.content.clone(), String::new())),
                crate::agent::MessageRole::Assistant => {
                    if let Some(last) = self.ui.messages.iter().rev().find(|m| m.role == crate::agent::MessageRole::User) {
                        Some((last.content.clone(), msg.content.clone()))
                    } else {
                        None
                    }
                },
                _ => None,
            })
            .collect()
    }

    /// 获取用户ID
    pub fn get_user_id(&self) -> String {
        self.user_id.clone()
    }
}

/// 创建默认机器人
pub fn create_default_bots(config_manager: &ConfigManager) -> Result<()> {
    let bots = config_manager.get_bots()?;

    if bots.is_empty() {
        // 创建默认机器人
        let default_bot = BotConfig::new(
            "助手",
            "你是一个有用的 AI 助手，能够回答各种问题并提供帮助。",
            "password",
        );
        config_manager.add_bot(default_bot)?;

        let coder_bot = BotConfig::new(
            "程序员",
            "你是一个经验丰富的程序员，精通多种编程语言，能够帮助解决编程问题。",
            "password",
        );
        config_manager.add_bot(coder_bot)?;

        log::info!("已创建默认机器人");
    }

    Ok(())
}
