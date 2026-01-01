use crate::agent::{Agent, AgentFactory, ChatMessage};
use crate::config::{BotConfig, ConfigManager};
use crate::logger::LogManager;
use crate::ui::{AppState, AppUi};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 应用程序
pub struct App {
    config_manager: ConfigManager,
    log_manager: Arc<LogManager>,
    ui: AppUi,
    current_bot: Option<BotConfig>,
    agent: Option<Box<dyn Agent>>,
    should_quit: bool,
}

impl App {
    /// 创建新的应用
    pub fn new(config_manager: ConfigManager, log_manager: Arc<LogManager>) -> Result<Self> {
        let mut ui = AppUi::new();

        // 加载机器人列表
        let bots = config_manager.get_bots()?;
        ui.set_bot_list(bots);

        log::info!("应用程序初始化完成");

        Ok(Self {
            config_manager,
            log_manager,
            ui,
            current_bot: None,
            agent: None,
            should_quit: false,
        })
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
        let tick_rate = Duration::from_millis(100);

        loop {
            // 更新日志
            if last_log_update.elapsed() > Duration::from_secs(1) {
                self.update_logs();
                last_log_update = Instant::now();
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
                let log_count = logs.len();
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
                self.agent = Some(AgentFactory::create_mock_agent(
                    &bot.name,
                    &bot.system_prompt,
                ));
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

        log::info!("用户发送消息: {}", input_text);
        log::debug!("当前消息总数: {}", self.ui.messages.len());

        // 获取 AI 响应
        if let Some(agent) = &self.agent {
            let mut messages = vec![];
            if let Some(bot) = &self.current_bot {
                messages.push(ChatMessage::system(&bot.system_prompt));
                log::debug!("添加系统提示词");
            }
            messages.extend(self.ui.messages.iter().cloned());
            log::debug!("准备调用 Agent，消息数: {}", messages.len());

            match agent.chat(&messages).await {
                Ok(response) => {
                    log::info!("AI 响应成功，长度: {}", response.len());
                    let assistant_message = ChatMessage::assistant(response);
                    self.ui.messages.push(assistant_message);
                }
                Err(e) => {
                    log::error!("AI 响应失败: {}", e);
                    let error_message = ChatMessage::assistant(format!("错误: {}", e));
                    self.ui.messages.push(error_message);
                }
            }
        } else {
            log::warn!("Agent 未初始化");
        }

        // 滚动到底部 - 将在渲染时自动计算
        self.ui.scroll_offset = usize::MAX;

        Ok(())
    }

    /// 清空会话
    fn clear_chat(&mut self) {
        log::info!("清空会话");
        self.ui.messages.clear();
        self.ui.scroll_offset = 0;
    }

    /// 显示帮助信息
    fn show_help(&mut self) {
        log::info!("显示帮助信息");
        let help_message = ChatMessage::assistant(AppUi::get_help_message());
        self.ui.messages.push(help_message);
        self.ui.scroll_offset = usize::MAX;
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
        self.ui.scroll_offset = usize::MAX;
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

