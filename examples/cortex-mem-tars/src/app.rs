use crate::agent::{AgentChatHandler, ChatMessage, create_memory_agent, extract_user_basic_info};
use crate::config::{BotConfig, ConfigManager};
use crate::infrastructure::Infrastructure;
use crate::logger::LogManager;
use crate::ui::{AppState, AppUi};
use anyhow::{Context, Result};
use cortex_mem_tools::MemoryOperations;
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

// 音频相关导入
use crate::audio_input;
use crate::audio_transcription::{self, WhisperTranscriber};

/// 应用程序
#[allow(dead_code)]
pub struct App {
    config_manager: ConfigManager,
    log_manager: Arc<LogManager>,
    ui: AppUi,
    current_bot: Option<BotConfig>,
    rig_agent: Option<RigAgent<CompletionModel>>,
    tenant_operations: Option<Arc<MemoryOperations>>,
    current_session_id: Option<String>,
    infrastructure: Option<Arc<Infrastructure>>,
    user_id: String,
    user_info: Option<String>,
    should_quit: bool,
    message_sender: mpsc::UnboundedSender<AppMessage>,
    message_receiver: mpsc::UnboundedReceiver<AppMessage>,
    pub current_bot_id: Arc<std::sync::RwLock<Option<String>>>,
    previous_state: Option<crate::ui::AppState>,

    // 🎙️ 音频输入相关
    audio_input_enabled: bool, // 是否启用语音输入
    audio_task_handle: Option<tokio::task::JoinHandle<()>>, // 音频处理任务句柄
    audio_text_receiver: Option<mpsc::UnboundedReceiver<String>>, // 接收转录文本
    audio_transcriber: Option<Arc<WhisperTranscriber>>, // Whisper转录器
    agent_handler: Option<AgentChatHandler>, // Agent处理器
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
    ) -> Result<Self> {
        let mut ui = AppUi::new();

        // 加载机器人列表
        let bots = config_manager.get_bots()?;
        ui.set_bot_list(bots);

        // 创建消息通道
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<AppMessage>();

        log::info!("应用程序初始化完成");

        let initial_state = ui.state;

        Ok(Self {
            config_manager,
            log_manager,
            ui,
            current_bot: None,
            rig_agent: None,
            tenant_operations: None,
            current_session_id: None,
            infrastructure,
            user_id: "tars_user".to_string(),
            user_info: None,
            should_quit: false,
            message_sender: msg_tx,
            message_receiver: msg_rx,
            current_bot_id: Arc::new(std::sync::RwLock::new(None)),
            previous_state: Some(initial_state),

            // 🎙️ 音频输入初始化
            audio_input_enabled: false,
            audio_task_handle: None,
            audio_text_receiver: None,
            audio_transcriber: None,
            agent_handler: None,
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
                        log::debug!("服务不可用，状态码: {}", response.status());
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

            // 定期检查服务状态
            if last_service_check.elapsed() > Duration::from_secs(15) {
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
                                last_msg.content = full_response.clone();
                                // 只清除当前正在更新的消息的缓存
                                let last_idx = self.ui.messages.len() - 1;
                                self.ui.invalidate_render_cache(Some(last_idx));
                            } else {
                                self.ui.messages.push(ChatMessage::assistant(full_response.clone()));
                                self.ui.invalidate_render_cache(None);
                            }
                        } else {
                            self.ui.messages.push(ChatMessage::assistant(full_response.clone()));
                            self.ui.invalidate_render_cache(None);
                        }
                        
                        // 🔧 更新 agent_handler 的历史记录，确保下一轮对话能获取完整上下文
                        if let Some(ref mut handler) = self.agent_handler {
                            handler.add_assistant_response(full_response);
                            log::debug!("✅ 已将助手响应添加到对话历史");
                        }
                        
                        // 确保自动滚动启用
                        self.ui.auto_scroll = true;
                    }
                    AppMessage::Log(_) => {
                        // 日志消息暂时忽略
                    }
                }
            }

            // 🎙️ 处理语音转录结果
            let mut texts_to_process = Vec::new();
            if let Some(ref mut rx) = self.audio_text_receiver {
                while let Ok(text) = rx.try_recv() {
                    texts_to_process.push(text);
                }
            }

            for text in texts_to_process {
                if let Err(e) = self.handle_audio_transcription(text).await {
                    log::error!("处理语音转录失败: {}", e);
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
                            crate::ui::KeyAction::EnableAudioInput => {
                                if self.ui.state == AppState::Chat {
                                    if let Err(e) = self.enable_audio_input().await {
                                        log::error!("启用语音输入失败: {}", e);
                                        self.ui.messages.push(ChatMessage::system(format!(
                                            "❌ 启用语音输入失败: {}",
                                            e
                                        )));
                                    }
                                }
                            }
                            crate::ui::KeyAction::DisableAudioInput => {
                                if self.ui.state == AppState::Chat {
                                    if let Err(e) = self.disable_audio_input().await {
                                        log::error!("禁用语音输入失败: {}", e);
                                        self.ui.messages.push(ChatMessage::system(format!(
                                            "❌ 禁用语音输入失败: {}",
                                            e
                                        )));
                                    }
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

            // 检测状态变化（在事件处理之后）

            log::trace!(
                "状态检查: previous_state={:?}, current_state={:?}",
                self.previous_state,
                self.ui.state
            );

            if self.previous_state != Some(self.ui.state) {
                log::info!(
                    "🔄 状态变化: {:?} -> {:?}",
                    self.previous_state,
                    self.ui.state
                );

                // 如果从 BotSelection 或 PasswordInput 切换到 Chat，启动 API 服务器

                log::info!(
                    "检查条件: previous_state == BotSelection: {}",
                    self.previous_state == Some(crate::ui::AppState::BotSelection)
                );

                log::info!(
                    "检查条件: previous_state == PasswordInput: {}",
                    self.previous_state == Some(crate::ui::AppState::PasswordInput)
                );

                log::info!(
                    "检查条件: current_state == Chat: {}",
                    self.ui.state == crate::ui::AppState::Chat
                );

                if (self.previous_state == Some(crate::ui::AppState::BotSelection)
                    || self.previous_state == Some(crate::ui::AppState::PasswordInput))
                    && self.ui.state == crate::ui::AppState::Chat
                {
                    log::info!("✨ 检测到进入聊天模式");

                    if let Some(bot) = self.ui.selected_bot().cloned() {
                        log::info!("🤖 选中的机器人: {} (ID: {})", bot.name, bot.id);

                        log::info!("即将调用 on_enter_chat_mode...");

                        self.on_enter_chat_mode(&bot).await;

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
                crate::ui::KeyAction::EnableAudioInput => {
                    if let Err(e) = self.enable_audio_input().await {
                        self.ui
                            .messages
                            .push(ChatMessage::system(format!("❌ 启用语音输入失败: {}", e)));
                    }
                }
                crate::ui::KeyAction::DisableAudioInput => {
                    if let Err(e) = self.disable_audio_input().await {
                        self.ui
                            .messages
                            .push(ChatMessage::system(format!("❌ 禁用语音输入失败: {}", e)));
                    }
                }
                _ => {}
            }
            return Ok(());
        }

        // 检查是否有选中的机器人（防护性检查，正常情况下 on_enter_chat_mode 已经初始化）
        if self.current_bot.is_none() {
            if let Some(bot) = self.ui.selected_bot() {
                // 如果 on_enter_chat_mode 未被调用（例如直接跳转到 Chat），在此兜底初始化
                let bot_cloned = bot.clone();
                log::warn!("⚠️ 检测到 current_bot 未设置，执行兜底初始化...");
                self.on_enter_chat_mode(&bot_cloned).await;
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
        if let Some(_rig_agent) = &self.rig_agent {
            // 🔧 使用App持久化的agent_handler而不是每次创建新的
            if self.agent_handler.is_none() {
                log::error!("Agent handler 未初始化,请先初始化");
                return Ok(());
            }

            let msg_tx = self.message_sender.clone();
            let user_input = input_text.to_string();
            let user_input_for_stream = user_input.clone();

            // 🔧 获取agent_handler的引用来调用chat_stream
            let agent_handler = self
                .agent_handler
                .as_mut()
                .expect("Agent handler should exist");

            // 🔧 chat_stream 返回 (stream_rx, completion_rx)
            match agent_handler.chat_stream(&user_input).await {
                Ok((mut stream_rx, mut completion_rx)) => {
                    // 在主线程中spawn接收流式响应的任务
                    tokio::spawn(async move {
                        let mut full_response = String::new();

                        while let Some(chunk) = stream_rx.recv().await {
                            full_response.push_str(&chunk);
                            if let Err(_) = msg_tx.send(AppMessage::StreamingChunk {
                                user: user_input_for_stream.clone(),
                                chunk,
                            }) {
                                break;
                            }
                        }

                        // 🔧 从 completion_rx 获取完整响应（确保一致性）
                        if let Ok(response) = completion_rx.try_recv() {
                            full_response = response;
                        }

                        let _ = msg_tx.send(AppMessage::StreamingComplete {
                            user: user_input_for_stream.clone(),
                            full_response,
                        });
                    });
                }
                Err(e) => {
                    log::error!("生成回复失败: {}", e);
                    let error_msg = format!("生成回复失败: {}", e);
                    let _ = msg_tx.send(AppMessage::StreamingChunk {
                        user: user_input_for_stream.clone(),
                        chunk: error_msg,
                    });
                }
            }
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

    /// 进入聊天模式时初始化 Agent 和 AgentHandler
    /// 当从 BotSelection 或 PasswordInput 切换到 Chat 状态时调用此方法
    pub async fn on_enter_chat_mode(&mut self, bot: &BotConfig) {
        log::info!("🎯 进入聊天模式，机器人: {} (ID: {})", bot.name, bot.id);

        // 更新 current_bot_id 和 current_bot
        if let Ok(mut bot_id) = self.current_bot_id.write() {
            *bot_id = Some(bot.id.clone());
            log::info!("✅ 已更新当前机器人 ID: {}", bot.id);
        } else {
            log::error!("❌ 无法更新 current_bot_id");
        }
        self.current_bot = Some(bot.clone());

        // 如果 rig_agent 还未初始化，在此异步初始化
        if self.rig_agent.is_none() {
            if let Some(infrastructure) = &self.infrastructure {
                log::info!("🤖 开始初始化 AI Agent...");
                let config = infrastructure.config();
                match create_memory_agent(
                    config.cortex.data_dir(),
                    config,
                    None, // user_info 稍后从租户 operations 提取
                    Some(bot.system_prompt.as_str()),
                    &bot.id,
                    &self.user_id,
                )
                .await
                {
                    Ok((rig_agent, tenant_ops)) => {
                        self.tenant_operations = Some(tenant_ops.clone());

                        // 从租户 operations 提取用户基本信息
                        let user_info = match extract_user_basic_info(
                            tenant_ops.clone(),
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

                        // 如果有用户信息，需要重新创建带用户信息的 Agent
                        if user_info.is_some() {
                            let config = infrastructure.config();
                            match create_memory_agent(
                                config.cortex.data_dir(),
                                config,
                                user_info.as_deref(),
                                Some(bot.system_prompt.as_str()),
                                &bot.id,
                                &self.user_id,
                            )
                            .await
                            {
                                Ok((agent_with_userinfo, _)) => {
                                    self.rig_agent = Some(agent_with_userinfo);
                                    log::info!("✅ 已创建带用户信息的 Agent");
                                }
                                Err(e) => {
                                    log::error!("重新创建带用户信息的 Agent 失败: {}", e);
                                    self.rig_agent = Some(rig_agent);
                                }
                            }
                        } else {
                            self.rig_agent = Some(rig_agent);
                            log::info!("✅ 已创建 Agent（无用户历史记忆）");
                        }
                    }
                    Err(e) => {
                        log::error!("❌ 初始化 AI Agent 失败: {}", e);
                        return;
                    }
                }
            } else {
                log::error!("❌ 基础设施未初始化，无法创建 Agent");
                return;
            }
        }

        // 初始化 agent_handler
        if let Some(rig_agent) = &self.rig_agent {
            let session_id = self
                .current_session_id
                .get_or_insert_with(|| uuid::Uuid::new_v4().to_string())
                .clone();

            if let Some(tenant_ops) = &self.tenant_operations {
                self.agent_handler = Some(AgentChatHandler::with_memory(
                    rig_agent.clone(),
                    tenant_ops.clone(),
                    session_id,
                ));
                log::info!(
                    "✅ 已初始化 agent_handler（含记忆）session_id: {}",
                    self.current_session_id.as_ref().unwrap()
                );
            } else {
                self.agent_handler = Some(AgentChatHandler::new(rig_agent.clone()));
                log::info!("✅ 已初始化 agent_handler（无记忆）");
            }
        } else {
            log::error!("❌ rig_agent 初始化后仍为 None，无法创建 agent_handler");
        }
    }

    /// 启用语音输入
    async fn enable_audio_input(&mut self) -> Result<()> {
        if self.audio_input_enabled {
            self.ui
                .messages
                .push(ChatMessage::system("ℹ️ 语音输入已经开启"));
            return Ok(());
        }

        // 🔧 确保已选择Bot并初始化Agent
        // 如果 current_bot 为 None，尝试从 UI 获取选中的 Bot
        if self.current_bot.is_none() {
            if let Some(bot) = self.ui.selected_bot() {
                self.current_bot = Some(bot.clone());

                // 更新 current_bot_id
                if let Ok(mut bot_id) = self.current_bot_id.write() {
                    *bot_id = Some(bot.id.clone());
                    log::info!("已更新当前机器人 ID: {}", bot.id);
                }
            } else {
                self.ui.messages.push(ChatMessage::system(
                    "⚠️ 请先选择一个Bot（按Tab键切换到Bot选择界面并选择Bot）",
                ));
                return Ok(());
            }
        }

        // 如果已选择Bot但Agent未初始化，先初始化Agent
        if self.rig_agent.is_none() {
            if let Some(bot) = &self.current_bot {
                if let Some(infrastructure) = &self.infrastructure {
                    self.ui
                        .messages
                        .push(ChatMessage::system("🤖 正在初始化AI Agent..."));

                    let config = infrastructure.config();
                    match create_memory_agent(
                        config.cortex.data_dir(),
                        config,
                        None,
                        Some(bot.system_prompt.as_str()),
                        &bot.id,
                        &self.user_id,
                    )
                    .await
                    {
                        Ok((rig_agent, tenant_ops)) => {
                            self.tenant_operations = Some(tenant_ops.clone());
                            self.rig_agent = Some(rig_agent);
                            log::info!("✅ Agent已初始化");
                        }
                        Err(e) => {
                            self.ui
                                .messages
                                .push(ChatMessage::system(format!("❌ Agent初始化失败: {}", e)));
                            return Ok(());
                        }
                    }
                }
            }
        }

        // 1. 加载 Whisper 模型（如果未加载）
        if self.audio_transcriber.is_none() {
            self.ui
                .messages
                .push(ChatMessage::system("📥 正在加载 Whisper 模型..."));

            let config = audio_transcription::TranscriptionConfig::default();
            let transcriber = audio_transcription::WhisperTranscriber::new(config)
                .context("无法加载 Whisper 模型")?;

            self.audio_transcriber = Some(Arc::new(transcriber));
        }

        // 2. 创建通道
        let (tx, rx) = mpsc::unbounded_channel();
        self.audio_text_receiver = Some(rx);

        // 3. 启动音频处理任务
        let transcriber = self.audio_transcriber.as_ref().unwrap().clone();
        let handle = tokio::spawn(audio_processing_task(tx, transcriber));
        self.audio_task_handle = Some(handle);

        self.audio_input_enabled = true;
        self.ui.audio_input_enabled = true; // 🎙️ 同步UI状态

        self.ui.messages.push(ChatMessage::system(
            "✅ 语音输入已开启，每60秒自动转录一次...",
        ));

        log::info!("🎙️ 语音输入已启用");
        Ok(())
    }

    /// 禁用语音输入
    async fn disable_audio_input(&mut self) -> Result<()> {
        if !self.audio_input_enabled {
            return Ok(());
        }

        // 🔇 在关闭音频时临时重定向stderr，避免清理操作破坏TUI
        #[cfg(unix)]
        let _null_file = std::fs::File::create("/dev/null").ok();
        #[cfg(windows)]
        let _null_file = std::fs::File::create("NUL").ok();

        #[cfg(unix)]
        let _temp_stderr_guard = _null_file.as_ref().and_then(|f| {
            use std::os::unix::io::AsRawFd;
            unsafe {
                let saved = libc::dup(2);
                if saved >= 0 {
                    libc::dup2(f.as_raw_fd(), 2);
                    Some(TempStderrGuard { saved })
                } else {
                    None
                }
            }
        });

        // 1. 停止音频任务
        if let Some(handle) = self.audio_task_handle.take() {
            handle.abort();

            // 等待一小段时间，确保任务清理完成
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 2. 清理接收器
        self.audio_text_receiver = None;

        self.audio_input_enabled = false;
        self.ui.audio_input_enabled = false; // 🎙️ 同步UI状态

        self.ui
            .messages
            .push(ChatMessage::system("🔇 语音输入已关闭"));

        log::info!("🔇 语音输入已禁用");

        // _temp_stderr_guard 会在函数结束时恢复 stderr
        Ok(())
    }

    /// 处理语音转录结果
    async fn handle_audio_transcription(&mut self, text: String) -> Result<()> {
        log::info!("🎙️ 语音识别: {}", text);

        // 1. 添加系统消息显示识别结果
        self.ui
            .messages
            .push(ChatMessage::system(format!("🎙️ 识别: {}", text)));

        // 2. 添加用户消息
        self.ui.messages.push(ChatMessage::user(text.clone()));

        // 3. 触发 AI 回复
        // 🔧 复用已有的 agent_handler，保持对话历史上下文
        if let Some(ref mut agent_handler) = self.agent_handler {
            let msg_sender = self.message_sender.clone();
            let text_clone = text.clone();

            match agent_handler.chat_stream(&text).await {
                Ok((mut stream_rx, mut completion_rx)) => {
                    tokio::spawn(async move {
                        let mut full_response = String::new();

                        while let Some(chunk) = stream_rx.recv().await {
                            full_response.push_str(&chunk);
                            let _ = msg_sender.send(AppMessage::StreamingChunk {
                                user: text_clone.clone(),
                                chunk,
                            });
                        }

                        // 从 completion_rx 获取完整响应
                        if let Ok(response) = completion_rx.try_recv() {
                            full_response = response;
                        }

                        let _ = msg_sender.send(AppMessage::StreamingComplete {
                            user: text_clone,
                            full_response,
                        });
                    });
                }
                Err(e) => {
                    log::error!("语音消息生成回复失败: {}", e);
                    self.ui
                        .messages
                        .push(ChatMessage::system(format!("❌ 生成回复失败: {}", e)));
                }
            }
        } else {
            log::error!("⚠️ agent_handler 未初始化");
            self.ui.messages.push(ChatMessage::system(
                "⚠️ 内部错误：Agent状态异常，请重新选择机器人",
            ));
        }

        Ok(())
    }

    /// 退出时的清理工作，触发记忆提取
    pub async fn on_exit(&mut self) -> Result<()> {
        log::info!("🚪 开始退出流程...");

        // 🎙️ 清理音频输入资源
        if self.audio_input_enabled {
            log::info!("🔇 关闭语音输入...");
            let _ = self.disable_audio_input().await;
        }

        if let (Some(tenant_ops), Some(session_id)) =
            (&self.tenant_operations, &self.current_session_id)
        {
            // 同步关闭会话：等待记忆提取 + user/agent 文件写入 + L0/L1 生成全部完成后才返回
            log::info!("🧠 同步关闭会话，等待记忆提取与层级文件生成完成...");
            match tenant_ops.close_session_sync(session_id).await {
                Ok(_) => {
                    log::info!("✅ 会话已关闭，记忆提取与 L0/L1 生成完成");
                }
                Err(e) => {
                    log::warn!("⚠️ 会话关闭失败: {}", e);
                }
            }

            // 索引所有文件到向量数据库（此时 L0/L1 已全部生成，索引数据完整）
            log::info!("📊 开始索引所有文件到向量数据库...");
            let index_timeout = tokio::time::Duration::from_secs(360);
            match tokio::time::timeout(index_timeout, tenant_ops.index_all_files()).await {
                Ok(Ok(stats)) => {
                    log::info!(
                        "✅ 索引完成: 总计 {} 个文件, {} 个已索引, {} 个跳过",
                        stats.total_files,
                        stats.indexed_files,
                        stats.skipped_files
                    );
                }
                Ok(Err(e)) => {
                    log::warn!("⚠️ 索引失败: {}", e);
                }
                Err(_) => {
                    log::warn!("⚠️ 索引超时（120秒），跳过索引以完成退出");
                }
            }
        } else {
            log::info!("ℹ️ 无需处理会话（未配置租户或无会话）");
        }

        log::info!("👋 退出流程完成");
        Ok(())
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

/// 音频处理任务 - 简化版：每60秒录制一段音频并转录
async fn audio_processing_task(
    text_sender: mpsc::UnboundedSender<String>,
    transcriber: Arc<WhisperTranscriber>,
) {
    // 🔇 在整个音频处理任务中重定向stderr到/dev/null，避免底层库输出破坏TUI
    #[cfg(unix)]
    let null_path = "/dev/null";
    #[cfg(windows)]
    let null_path = "NUL";

    let _null_file = std::fs::File::create(null_path).ok();

    #[cfg(unix)]
    let _stderr_guard = _null_file.as_ref().and_then(|f| {
        use std::os::unix::io::AsRawFd;
        unsafe {
            let saved_stderr = libc::dup(2);
            if saved_stderr >= 0 {
                libc::dup2(f.as_raw_fd(), 2);
                Some(StderrGuard { saved_stderr })
            } else {
                None
            }
        }
    });

    log::info!("🎙️ 音频处理任务启动（每60秒转录一次）");

    // 1. 启动音频流
    let (audio_tx, mut audio_rx) = mpsc::channel(100000);

    let audio_manager = match audio_input::AudioStreamManager::start(audio_tx) {
        Ok(manager) => {
            log::info!("✅ 音频流启动成功");
            manager
        }
        Err(e) => {
            log::error!("❌ 音频流启动失败: {}", e);
            return;
        }
    };

    let audio_config = audio_manager.config();
    let input_sample_rate = audio_config.sample_rate;
    let input_channels = audio_config.channels as usize;

    // 2. 设置60秒定时器
    const RECORDING_INTERVAL_SECS: u64 = 60;
    let mut interval = tokio::time::interval(Duration::from_secs(RECORDING_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // 跳过第一个tick（立即触发）
    interval.tick().await;

    // 3. 音频处理循环
    let mut audio_buffer = Vec::new();
    let mut segment_count = 0;

    loop {
        tokio::select! {
            // 定时器触发：转录当前缓冲区的音频
            _ = interval.tick() => {
                if audio_buffer.is_empty() {
                    log::debug!("⏱️ {}秒已过，但没有录制到音频数据", RECORDING_INTERVAL_SECS);
                    continue;
                }

                segment_count += 1;
                let sample_count = audio_buffer.len();
                let duration_secs = sample_count as f32 / input_sample_rate as f32 / input_channels as f32;

                log::info!(
                    "⏱️ 录音段 #{}: {:.1}秒, {} 采样",
                    segment_count,
                    duration_secs,
                    sample_count
                );

                // 计算音量（RMS）
                let rms = (audio_buffer.iter().map(|&x| x * x).sum::<f32>() / audio_buffer.len() as f32).sqrt();

                // 如果音量太低，跳过转录
                // 🔧 提高阈值，避免安静环境下噪音误触发
                if rms < 0.02 {
                    log::info!("⚠️ 音量过低 (RMS: {:.4})，跳过转录", rms);
                    audio_buffer.clear();
                    continue;
                }

                // 转换为单声道
                let mono_samples = audio_transcription::convert_to_mono(
                    &audio_buffer,
                    input_channels,
                );

                // 清空缓冲区，准备下一个60秒
                audio_buffer.clear();

                // 异步转录（不阻塞音频采集）
                let transcriber_clone = Arc::clone(&transcriber);
                let text_sender_clone = text_sender.clone();

                tokio::spawn(async move {
                    match transcriber_clone.transcribe(&mono_samples, input_sample_rate).await {
                        Ok(transcribed_text) => {
                            let text = transcribed_text.trim().to_string();

                            // 检查是否为有意义的文本
                            if audio_transcription::is_meaningful_text(&text, rms) {
                                log::info!("✅ 转录成功: {}", text);

                                if let Err(e) = text_sender_clone.send(text) {
                                    log::error!("发送转录文本失败: {}", e);
                                }
                            } else {
                                log::info!("⚠️ 转录结果无意义，已丢弃: {}", text);
                            }
                        }
                        Err(e) => {
                            log::error!("❌ 转录失败: {}", e);
                        }
                    }
                });
            }

            // 接收音频数据并累积到缓冲区
            Some(samples) = audio_rx.recv() => {
                audio_buffer.extend_from_slice(&samples);
            }

            // 音频流结束
            else => {
                log::info!("🛑 音频流已关闭");
                break;
            }
        }
    }

    log::info!("🛑 音频处理任务结束");
    // _stderr_guard 会在函数结束时自动恢复 stderr
}

// RAII守卫：在drop时恢复stderr
#[cfg(unix)]
struct StderrGuard {
    saved_stderr: i32,
}

#[cfg(unix)]
impl Drop for StderrGuard {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.saved_stderr, 2);
            libc::close(self.saved_stderr);
        }
    }
}

// 临时stderr守卫（用于disable_audio_input）
#[cfg(unix)]
struct TempStderrGuard {
    saved: i32,
}

#[cfg(unix)]
impl Drop for TempStderrGuard {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.saved, 2);
            libc::close(self.saved);
        }
    }
}
