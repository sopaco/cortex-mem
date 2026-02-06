use crate::agent::{
    AgentChatHandler, ChatMessage, create_memory_agent, extract_user_basic_info,
};
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
    pub current_bot_id: Arc<std::sync::RwLock<Option<String>>>,
    enable_audio_connect: bool,
    audio_connect_mode: String,
    api_server_started: std::sync::Arc<std::sync::atomic::AtomicBool>,
    previous_state: Option<crate::ui::AppState>,
    external_message_sender: mpsc::UnboundedSender<String>,
    external_message_receiver: mpsc::UnboundedReceiver<String>,
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
    pub fn new(
        config_manager: ConfigManager,
        log_manager: Arc<LogManager>,
        infrastructure: Option<Arc<Infrastructure>>,
        enable_audio_connect: bool,
        audio_connect_mode: String,
    ) -> Result<Self> {
        let mut ui = AppUi::new();

        // 加载机器人列表
        let bots = config_manager.get_bots()?;
        ui.set_bot_list(bots);

        // 创建消息通道
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<AppMessage>();
        let (external_msg_tx, external_msg_rx) = mpsc::unbounded_channel::<String>();

        log::info!("应用程序初始化完成");

        let initial_state = ui.state;

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
            current_bot_id: Arc::new(std::sync::RwLock::new(None)),
            enable_audio_connect,
            audio_connect_mode,
            api_server_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            previous_state: Some(initial_state),
            external_message_sender: external_msg_tx,
            external_message_receiver: external_msg_rx,
        })
    }

    /// 检查服务可用性
    pub async fn check_service_status(&mut self) -> Result<()> {
        use reqwest::Method;

        // 重新启用 API 服务器
        if let Some(infrastructure) = &self.infrastructure {
            let api_base_url = &infrastructure.config().llm.api_base_url;
            // 拼接完整的 API 地址
            let check_url = format!("{}/chat/completions", api_base_url.trim_end_matches('/'));

            // log::info!("检查服务可用性: {}", check_url);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .context("无法创建 HTTP 客户端")?;

            match client.request(Method::OPTIONS, &check_url).send().await {
                Ok(response) => {
                    if response.status().is_success() || response.status().as_u16() == 405 {
                        // 200 OK 或 405 Method Not Allowed 都表示服务可用
                        log::debug!("服务可用，状态码: {}", response.status());
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

        // 添加短暂的延迟，确保任何自动发送的事件都被处理掉
        // 特别是在 Windows 上，某些终端可能会在启动时自动发送 Enter 键事件
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 清空事件队列，忽略启动时的任何自动事件
        while event::poll(Duration::from_millis(10)).unwrap_or(false) {
            let _ = event::read();
        }

        let mut last_log_update = Instant::now();
        let mut last_service_check = Instant::now();
        let tick_rate = Duration::from_millis(100);

        loop {
            // 更新日志（降低频率到每3秒一次，减少不必要的UI刷新）
            if last_log_update.elapsed() > Duration::from_secs(3) {
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
                                // 只清除当前正在更新的消息的缓存
                                let last_idx = self.ui.messages.len() - 1;
                                self.ui.invalidate_render_cache(Some(last_idx));
                            } else {
                                // 如果最后一条不是助手消息，创建新的助手消息
                                self.ui.messages.push(ChatMessage::assistant(chunk));
                                // 新消息，清除所有缓存（因为索引会变化）
                                self.ui.invalidate_render_cache(None);
                            }
                        } else {
                            // 如果没有消息，创建新的助手消息
                            self.ui.messages.push(ChatMessage::assistant(chunk));
                            self.ui.invalidate_render_cache(None);
                        }
                        // 确保自动滚动启用
                        self.ui.auto_scroll = true;
                    }
                    AppMessage::StreamingComplete {
                        user: _,
                        full_response,
                    } => {
                        // 流式完成，确保完整响应已保存
                        if let Some(last_msg) = self.ui.messages.last_mut() {
                            if last_msg.role == crate::agent::MessageRole::Assistant {
                                last_msg.content = full_response;
                                // 只清除当前正在更新的消息的缓存
                                let last_idx = self.ui.messages.len() - 1;
                                self.ui.invalidate_render_cache(Some(last_idx));
                            } else {
                                self.ui.messages.push(ChatMessage::assistant(full_response));
                                self.ui.invalidate_render_cache(None);
                            }
                        } else {
                            self.ui.messages.push(ChatMessage::assistant(full_response));
                            self.ui.invalidate_render_cache(None);
                        }
                        // 确保自动滚动启用
                        self.ui.auto_scroll = true;
                    }
                    AppMessage::Log(_) => {
                        // 日志消息暂时忽略
                    }
                }
            }

            // 处理外部消息（来自 API 的 chat 模式）
            if let Ok(external_msg) = self.external_message_receiver.try_recv() {
                log::info!("收到外部消息: {}", external_msg);
                // 调用 handle_external_message 处理外部消息
                if let Err(e) = self.handle_external_message(external_msg).await {
                    log::error!("处理外部消息失败: {}", e);
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

                        log::debug!("事件处理完成，当前状态: {:?}", self.ui.state);

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
                                    log::info!(
                                        "show_themes() 调用完成，theme_modal_visible: {}",
                                        self.ui.theme_modal_visible
                                    );
                                } else {
                                    log::warn!("不在 Chat 状态，无法显示主题");
                                }
                            }
                            crate::ui::KeyAction::DumpChats => {
                                if self.ui.state == AppState::Chat {
                                    self.dump_chats();
                                }
                            }
                            crate::ui::KeyAction::CreateBot => {
                                // 创建机器人的逻辑在 UI 中处理
                            }
                            crate::ui::KeyAction::EditBot => {
                                // 编辑机器人的逻辑在 UI 中处理
                            }
                            crate::ui::KeyAction::DeleteBot => {
                                self.delete_bot().await?;
                            }
                            crate::ui::KeyAction::SaveBot => {
                                self.save_bot().await?;
                            }
                            crate::ui::KeyAction::CancelBot => {
                                // 取消操作由 UI 处理
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

            // 检测状态变化（在事件处理之后）

                        log::trace!("状态检查: previous_state={:?}, current_state={:?}", self.previous_state, self.ui.state);



                        if self.previous_state != Some(self.ui.state) {

                            log::info!("🔄 状态变化: {:?} -> {:?}", self.previous_state, self.ui.state);



                            // 如果从 BotSelection 或 PasswordInput 切换到 Chat，启动 API 服务器

                            log::info!("检查条件: previous_state == BotSelection: {}",

                                self.previous_state == Some(crate::ui::AppState::BotSelection));

                            log::info!("检查条件: previous_state == PasswordInput: {}",

                                self.previous_state == Some(crate::ui::AppState::PasswordInput));

                            log::info!("检查条件: current_state == Chat: {}",

                                self.ui.state == crate::ui::AppState::Chat);



                            if (self.previous_state == Some(crate::ui::AppState::BotSelection)

                                || self.previous_state == Some(crate::ui::AppState::PasswordInput))

                                && self.ui.state == crate::ui::AppState::Chat

                            {

                                log::info!("✨ 检测到进入聊天模式");

                                if let Some(bot) = self.ui.selected_bot().cloned() {

                                    log::info!("🤖 选中的机器人: {} (ID: {})", bot.name, bot.id);

                                    log::info!("即将调用 on_enter_chat_mode...");

                                    self.on_enter_chat_mode(&bot);

                                    log::info!("on_enter_chat_mode 调用完成");

                                } else {

                                    log::warn!("⚠️  没有选中的机器人");

                                }

                            } else {

                                log::info!("⏭️  状态变化不符合启动 API 服务器的条件");

                            }

                            self.previous_state = Some(self.ui.state);

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

                // 更新 current_bot_id
                if let Ok(mut bot_id) = self.current_bot_id.write() {
                    *bot_id = Some(bot.id.clone());
                    log::info!("已更新当前机器人 ID: {}", bot.id);
                }

                // 如果有基础设施，创建真实的带记忆的 Agent
                if let Some(infrastructure) = &self.infrastructure {
                    // 先提取用户基本信息（使用 bot.id 作为 agent_id）
                    let user_info = match extract_user_basic_info(
                        infrastructure.operations().clone(),
                        &self.user_id,
                        &bot.id,
                    )
                    .await
                    {
                        Ok(info) => {
                            self.user_info = info.clone();
                            info
                        }
                        Err(e) => {
                            log::error!("提取用户基本信息失败: {}", e);
                            None
                        }
                    };

                    match create_memory_agent(
                        infrastructure.operations().clone(),
                        &infrastructure.config().llm.api_base_url,
                        &infrastructure.config().llm.api_key,
                        &infrastructure.config().llm.model_efficient,
                        user_info.as_deref(),
                        Some(bot.system_prompt.as_str()),
                        &bot.id,
                        &self.user_id,
                    )
                    .await
                    {
                        Ok(rig_agent) => {
                            self.rig_agent = Some(rig_agent);
                            log::info!("已创建带记忆功能的真实 Agent");
                        }
                        Err(e) => {
                            log::error!("创建真实 Agent 失败 {}", e);
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
        self.ui.invalidate_render_cache(None);
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
            let mut agent_handler = AgentChatHandler::new(rig_agent.clone());
            let msg_tx = self.message_sender.clone();
            let user_input = input_text.to_string();
            let user_input_for_stream = user_input.clone();

            tokio::spawn(async move {
                match agent_handler.chat(&user_input).await {
                    Ok(response) => {
                        let _ = msg_tx.send(AppMessage::StreamingComplete {
                            user: user_input_for_stream.clone(),
                            full_response: response,
                        });
                    }
                    Err(e) => {
                        log::error!("生成回复失败: {}", e);
                    }
                }
            });
        }
        
        if self.infrastructure.is_none() {
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
        self.ui.invalidate_render_cache(None);
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
                self.ui.invalidate_render_cache(None);
            }
            Err(e) => {
                log::error!("{}", e);
                let error_message = ChatMessage::assistant(format!("❌ {}", e));
                self.ui.messages.push(error_message);
                self.ui.invalidate_render_cache(None);
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
                
                // 使用 current_bot 的 id 作为 thread_id
                let thread_id = if let Some(bot) = &self.current_bot {
                    bot.id.clone()
                } else {
                    "default".to_string()
                };
                
                // 批量存储对话（使用新的 store API）
                for (user_msg, assistant_msg) in &conversations {
                    if !user_msg.is_empty() {
                        let store_args = cortex_mem_tools::StoreArgs {
                            content: user_msg.clone(),
                            thread_id: thread_id.clone(),
                            metadata: None,
                            auto_generate_layers: Some(true),
                        };
                        infrastructure.operations()
                            .store(store_args)
                            .await
                            .map_err(|e| anyhow::anyhow!("存储用户消息失败: {}", e))?;
                    }
                    
                    if !assistant_msg.is_empty() {
                        let store_args = cortex_mem_tools::StoreArgs {
                            content: assistant_msg.clone(),
                            thread_id: thread_id.clone(),
                            metadata: None,
                            auto_generate_layers: Some(true),
                        };
                        infrastructure.operations()
                            .store(store_args)
                            .await
                            .map_err(|e| anyhow::anyhow!("存储助手消息失败: {}", e))?;
                    }
                }
                log::info!("对话保存完成");
            }
        }
        Ok(())
    }

    /// 获取所有对话
    pub fn get_conversations(&self) -> Vec<(String, String)> {
        self.ui
            .messages
            .iter()
            .filter_map(|msg| match msg.role {
                crate::agent::MessageRole::User => Some((msg.content.clone(), String::new())),
                crate::agent::MessageRole::Assistant => {
                    if let Some(last) = self
                        .ui
                        .messages
                        .iter()
                        .rev()
                        .find(|m| m.role == crate::agent::MessageRole::User)
                    {
                        Some((last.content.clone(), msg.content.clone()))
                    } else {
                        None
                    }
                }
            })
            .collect()
    }

    /// 获取用户ID
    pub fn get_user_id(&self) -> String {
        self.user_id.clone()
    }

    /// 处理来自 API 的外部消息（模拟用户输入）
    pub async fn handle_external_message(&mut self, content: String) -> Result<()> {
        log::info!("收到外部消息: {}", content);

        // 检查是否选择了机器人
        if self.current_bot.is_none() {
            if let Some(bot) = self.ui.selected_bot() {
                self.current_bot = Some(bot.clone());

                // 更新 current_bot_id
                if let Ok(mut bot_id) = self.current_bot_id.write() {
                    *bot_id = Some(bot.id.clone());
                    log::info!("已更新当前机器人 ID: {}", bot.id);
                }

                // 如果有基础设施，创建真实的带记忆的 Agent
                if let Some(infrastructure) = &self.infrastructure {
                    // 先提取用户基本信息（使用 bot.id 作为 agent_id）
                    let user_info = match extract_user_basic_info(
                        infrastructure.operations().clone(),
                        &self.user_id,
                        &bot.id,
                    )
                    .await
                    {
                        Ok(info) => {
                            self.user_info = info.clone();
                            info
                        }
                        Err(e) => {
                            log::error!("提取用户基本信息失败: {}", e);
                            None
                        }
                    };

                    match create_memory_agent(
                        infrastructure.operations().clone(),
                        &infrastructure.config().llm.api_base_url,
                        &infrastructure.config().llm.api_key,
                        &infrastructure.config().llm.model_efficient,
                        user_info.as_deref(),
                        Some(bot.system_prompt.as_str()),
                        &bot.id,
                        &self.user_id,
                    )
                    .await
                    {
                        Ok(rig_agent) => {
                            self.rig_agent = Some(rig_agent);
                            log::info!("已创建带记忆功能的真实 Agent");
                        }
                        Err(e) => {
                            log::error!("创建真实 Agent 失败 {}", e);
                        }
                    }
                }

                log::info!("选择机器人: {}", bot.name);
            } else {
                log::warn!("没有选中的机器人");
                return Ok(());
            }
        }

        // 添加用户消息到 UI
        let user_message = ChatMessage::user(content.clone());
        self.ui.messages.push(user_message.clone());
        self.ui.invalidate_render_cache(None);

        // 用户发送新消息，重新启用自动滚动
        self.ui.auto_scroll = true;

        log::info!("外部消息已添加到对话: {}", content);
        log::debug!("当前消息总数: {}", self.ui.messages.len());

        // 使用真实的带记忆的 Agent 进行流式响应
        if let Some(rig_agent) = &self.rig_agent {
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

            let mut agent_handler = AgentChatHandler::new(rig_agent.clone());
            let msg_tx = self.message_sender.clone();
            let user_input = content.clone();
            let user_input_for_stream = user_input.clone();

            tokio::spawn(async move {
                match agent_handler.chat(&user_input).await {
                    Ok(response) => {
                        let _ = msg_tx.send(AppMessage::StreamingComplete {
                            user: user_input_for_stream.clone(),
                            full_response: response,
                        });
                    }
                    Err(e) => {
                        log::error!("生成回复失败: {}", e);
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

    /// 保存机器人（创建或更新）
    async fn save_bot(&mut self) -> Result<()> {
        let (name, prompt, password) = self.ui.get_bot_input_data();

        if name.trim().is_empty() {
            log::warn!("机器人名称不能为空");
            return Ok(());
        }

        if prompt.trim().is_empty() {
            log::warn!("系统提示词不能为空");
            return Ok(());
        }

        match self.ui.bot_management_state {
            crate::ui::BotManagementState::Creating => {
                // 创建新机器人
                let bot_name = name.clone();
                let new_bot = crate::config::BotConfig::new(name, prompt, password);
                self.config_manager.add_bot(new_bot)?;
                log::info!("成功创建机器人: {}", bot_name);

                // 刷新机器人列表
                self.refresh_bot_list()?;
            }
            crate::ui::BotManagementState::Editing => {
                // 更新现有机器人
                if let Some(index) = self.ui.get_selected_bot_index() {
                    if let Some(existing_bot) = self.config_manager.get_bots()?.get(index) {
                        let bot_name = name.clone();
                        let updated_bot = crate::config::BotConfig {
                            id: existing_bot.id.clone(),
                            name: name.clone(),
                            system_prompt: prompt,
                            access_password: password,
                            created_at: existing_bot.created_at,
                        };
                        self.config_manager
                            .update_bot(&existing_bot.id, updated_bot)?;
                        log::info!("成功更新机器人: {}", bot_name);

                        // 刷新机器人列表
                        self.refresh_bot_list()?;
                    }
                }
            }
            _ => {}
        }

        // 返回列表状态
        self.ui.bot_management_state = crate::ui::BotManagementState::List;
        Ok(())
    }

    /// 删除机器人
    async fn delete_bot(&mut self) -> Result<()> {
        if let Some(index) = self.ui.get_selected_bot_index() {
            if let Some(bot) = self.config_manager.get_bots()?.get(index) {
                let bot_id = bot.id.clone();
                let bot_name = bot.name.clone();

                if self.config_manager.remove_bot(&bot_id)? {
                    log::info!("成功删除机器人: {}", bot_name);

                    // 刷新机器人列表
                    self.refresh_bot_list()?;

                    // 如果删除的是当前选中的机器人，重置选择
                    if let Some(selected) = self.ui.bot_list_state.selected() {
                        if selected >= self.ui.bot_list.len() && !self.ui.bot_list.is_empty() {
                            self.ui
                                .bot_list_state
                                .select(Some(self.ui.bot_list.len() - 1));
                        }
                    }
                }
            }
        }

        // 返回列表状态
        self.ui.bot_management_state = crate::ui::BotManagementState::List;
        Ok(())
    }

    /// 刷新机器人列表
    fn refresh_bot_list(&mut self) -> Result<()> {
        let bots = self.config_manager.get_bots()?;
        self.ui.set_bot_list(bots);
        Ok(())
    }

    /// 启动 API 服务器
    fn start_api_server(&self) {
        log::info!("🚀 尝试启动 API 服务器...");
        log::info!("   - enable_audio_connect: {}", self.enable_audio_connect);
        log::info!("   - api_server_started: {}",
            self.api_server_started.load(std::sync::atomic::Ordering::Relaxed));
        log::info!("   - infrastructure: {}", self.infrastructure.is_some());

        if !self.enable_audio_connect {
            log::warn!("❌ 音频连接功能未启用，跳过 API 服务器启动");
            log::warn!("   提示：请使用 --enable-audio-connect 参数启动应用");
            return;
        }

        // 检查是否已经启动
        if self
            .api_server_started
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            log::debug!("API 服务器已经启动，跳过");
            return;
        }

        // API 服务器已适配 V2 架构
        if let Some(infrastructure) = &self.infrastructure {
            let api_port = std::env::var("TARS_API_PORT")
                .unwrap_or_else(|_| "18199".to_string())
                .parse::<u16>()
                .unwrap_or(8080);

            log::info!("   - API 端口: {}", api_port);

            // 获取当前机器人 ID
            let current_bot_id = if let Ok(bot_id) = self.current_bot_id.read() {
                bot_id.clone()
            } else {
                None
            };
            log::info!("   - 当前机器人 ID: {:?}", current_bot_id);

            let api_state = crate::api_server::ApiServerState {
                operations: infrastructure.operations().clone(),
                current_bot_id: self.current_bot_id.clone(),
                audio_connect_mode: self.audio_connect_mode.clone(),
                external_message_sender: Some(self.external_message_sender.clone()),
            };

            let api_server_started = self.api_server_started.clone();

            // 在后台启动 API 服务器
            let handle = tokio::spawn(async move {
                log::info!("🔄 正在启动 API 服务器任务...");
                match crate::api_server::start_api_server(api_state, api_port).await {
                    Ok(_) => {
                        log::info!("✅ API 服务器任务完成");
                    }
                    Err(e) => {
                        log::error!("❌ API 服务器错误: {}", e);
                        log::error!("   错误详情: {:?}", e);
                    }
                }
            });

            // 立即检查任务是否启动成功
            log::info!("📋 API 服务器任务句柄: {:?}", handle.id());

            // 标记为已启动
            api_server_started.store(true, std::sync::atomic::Ordering::Relaxed);

            log::info!("✅ API 服务器已在后台启动，监听端口 {}", api_port);
            log::info!("💡 请稍等几秒钟，让服务器完全启动...");

            // 添加一个异步任务来验证服务器是否真正启动
            let api_server_started_clone = api_server_started.clone();
            tokio::spawn(async move {
                // 等待 2 秒让服务器启动
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // 尝试连接服务器
                let health_url = format!("http://localhost:{}/api/memory/health", api_port);
                match reqwest::get(&health_url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            log::info!("✅ API 服务器健康检查成功！服务器已就绪");
                        } else {
                            log::warn!("⚠️  API 服务器健康检查失败，状态码: {}", response.status());
                        }
                    }
                    Err(e) => {
                        log::error!("❌ 无法连接到 API 服务器: {}", e);
                        // 如果连接失败，重置启动标志
                        api_server_started_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });
        } else {
            log::warn!("❌ 未启用音频连接：基础设施未初始化");
        }
    }

    /// 当切换到聊天状态时调用此方法
    pub fn on_enter_chat_mode(&mut self, bot: &BotConfig) {
        log::info!("🎯 进入聊天模式，机器人: {} (ID: {})", bot.name, bot.id);

        // 更新 current_bot_id
        if let Ok(mut bot_id) = self.current_bot_id.write() {
            *bot_id = Some(bot.id.clone());
            log::info!("✅ 已更新当前机器人 ID: {}", bot.id);
        } else {
            log::error!("❌ 无法更新 current_bot_id");
        }

        // 启动 API 服务器（如果启用了音频连接）
        log::info!("📡 准备启动 API 服务器...");
        self.start_api_server();
    }
}

/// 创建默认机器人
pub fn create_default_bots(config_manager: &ConfigManager) -> Result<()> {
    let bots = config_manager.get_bots()?;

    if bots.is_empty() {
        // 创建默认机器人（密码为空，不需要验证）
        let default_bot = BotConfig::new(
            "助手",
            "你是一个有用的 AI 助手，能够回答各种问题并提供帮助。",
            "",
        );
        config_manager.add_bot(default_bot)?;

        let coder_bot = BotConfig::new(
            "程序员",
            "你是一个经验丰富的程序员，精通多种编程语言，能够帮助解决编程问题。",
            "",
        );
        config_manager.add_bot(coder_bot)?;

        log::info!("已创建默认机器人");
    }

    Ok(())
}
