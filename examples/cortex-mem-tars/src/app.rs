use crate::agent::{
    ChatMessage, MessageRole, create_system_prompt, extract_user_basic_info,
    handle_user_message, store_conversations_batch,
};
use crate::config::{BotConfig, ConfigManager, LLMConfig};
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
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Application message types
#[derive(Debug, Clone)]
pub enum AppMessage {
    AssistantResponse(String),
}

/// Main application
pub struct App {
    #[allow(dead_code)]
    config_manager: ConfigManager,
    log_manager: Arc<LogManager>,
    ui: AppUi,
    current_bot: Option<BotConfig>,
    infrastructure: Option<Arc<Infrastructure>>,
    thread_id: String,
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
    llm_config: LLMConfig,
    system_prompt: Option<String>,
}

impl App {
    /// Create new application
    pub fn new(
        config_manager: ConfigManager,
        log_manager: Arc<LogManager>,
        infrastructure: Option<Arc<Infrastructure>>,
        enable_audio_connect: bool,
        audio_connect_mode: String,
        _data_dir: String,
    ) -> Result<Self> {
        let mut ui = AppUi::new();

        // Load bot list
        let bots = config_manager.get_bots()?;
        ui.set_bot_list(bots);

        // Create message channels
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<AppMessage>();
        let (external_msg_tx, external_msg_rx) = mpsc::unbounded_channel::<String>();

        log::info!("Application initialized");

        let initial_state = ui.state;
        let llm_config = config_manager.config().llm.clone();

        Ok(Self {
            config_manager,
            log_manager,
            ui,
            current_bot: None,
            infrastructure,
            thread_id: "tars_thread".to_string(),
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
            llm_config,
            system_prompt: None,
        })
    }

    /// Check service availability
    pub async fn check_service_status(&mut self) -> Result<()> {
        use reqwest::Method;

        let api_base_url = &self.llm_config.api_base_url;
        let check_url = format!("{}/chat/completions", api_base_url.trim_end_matches('/'));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .context("Failed to create HTTP client")?;

        match client.request(Method::OPTIONS, &check_url).send().await {
            Ok(response) => {
                if response.status().is_success() || response.status().as_u16() == 405 {
                    log::debug!("Service available, status: {}", response.status());
                    self.ui.service_status = crate::ui::ServiceStatus::Active;
                } else {
                    log::warn!("Service unavailable, status: {}", response.status());
                    self.ui.service_status = crate::ui::ServiceStatus::Inactive;
                }
            }
            Err(e) => {
                log::error!("Service check failed: {}", e);
                self.ui.service_status = crate::ui::ServiceStatus::Inactive;
            }
        }

        Ok(())
    }

    /// Run application
    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode().context("Failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            crossterm::terminal::DisableLineWrap
        )
        .context("Failed to setup terminal")?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = ratatui::Terminal::new(backend).context("Failed to create terminal")?;

        // Clear event queue
        tokio::time::sleep(Duration::from_millis(100)).await;
        while event::poll(Duration::from_millis(10)).unwrap_or(false) {
            let _ = event::read();
        }

        let mut last_log_update = Instant::now();
        let mut last_service_check = Instant::now();
        let tick_rate = Duration::from_millis(100);

        loop {
            // Update logs
            if last_log_update.elapsed() > Duration::from_secs(3) {
                self.update_logs();
                last_log_update = Instant::now();
            }

            // Check service status
            if last_service_check.elapsed() > Duration::from_secs(5) {
                let _ = self.check_service_status().await;
                last_service_check = Instant::now();
            }

            // Process messages
            if let Ok(msg) = self.message_receiver.try_recv() {
                match msg {
                    AppMessage::AssistantResponse(response) => {
                        self.ui.messages.push(ChatMessage::assistant(response));
                        self.ui.invalidate_render_cache(None);
                        self.ui.auto_scroll = true;
                    }
                }
            }

            // Process external messages
            if let Ok(external_msg) = self.external_message_receiver.try_recv() {
                log::info!("Received external message: {}", external_msg);
                if let Err(e) = self.handle_external_message(external_msg).await {
                    log::error!("Failed to handle external message: {}", e);
                }
            }

            // Render UI
            terminal.draw(|f| self.ui.render(f)).context("Render failed")?;

            // Handle events
            if event::poll(tick_rate).context("Event poll failed")? {
                let event = event::read().context("Read event failed")?;
                log::trace!("Received event: {:?}", event);

                match event {
                    Event::Key(key) => {
                        let action = self.ui.handle_key_event(key);
                        log::debug!("Event processed, current state: {:?}", self.ui.state);

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
                                if self.ui.state == AppState::Chat {
                                    self.show_themes();
                                }
                            }
                            crate::ui::KeyAction::DumpChats => {
                                if self.ui.state == AppState::Chat {
                                    self.dump_chats();
                                }
                            }
                            crate::ui::KeyAction::CreateBot => {}
                            crate::ui::KeyAction::EditBot => {}
                            crate::ui::KeyAction::DeleteBot => {
                                self.delete_bot().await?;
                            }
                            crate::ui::KeyAction::SaveBot => {
                                self.save_bot().await?;
                            }
                            crate::ui::KeyAction::CancelBot => {}
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

            // State change detection
            if self.previous_state != Some(self.ui.state) {
                log::info!("State change: {:?} -> {:?}", self.previous_state, self.ui.state);

                if (self.previous_state == Some(crate::ui::AppState::BotSelection)
                    || self.previous_state == Some(crate::ui::AppState::PasswordInput))
                    && self.ui.state == crate::ui::AppState::Chat
                {
                    log::info!("Entering chat mode");
                    if let Some(bot) = self.ui.selected_bot().cloned() {
                        log::info!("Selected bot: {} (ID: {})", bot.name, bot.id);
                        self.on_enter_chat_mode(&bot);
                    }
                }

                self.previous_state = Some(self.ui.state);
            }

            if self.should_quit {
                break;
            }
        }

        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .context("Failed to restore terminal")?;

        terminal.show_cursor().context("Failed to show cursor")?;

        log::info!("Application exiting");
        Ok(())
    }

    /// Update logs
    fn update_logs(&mut self) {
        match self.log_manager.read_logs(1000) {
            Ok(logs) => {
                self.ui.log_lines = logs;
            }
            Err(e) => {
                log::error!("Failed to read logs: {}", e);
            }
        }
    }

    /// Send message
    async fn send_message(&mut self) -> Result<()> {
        let input_text = self.ui.get_input_text();
        let input_text = input_text.trim();

        if input_text.is_empty() {
            return Ok(());
        }

        // Check for commands
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

        // Check if we need to initialize chat
        if self.current_bot.is_none() {
            if let Some(bot) = self.ui.selected_bot() {
                self.current_bot = Some(bot.clone());

                if let Ok(mut bot_id) = self.current_bot_id.write() {
                    *bot_id = Some(bot.id.clone());
                    log::info!("Updated current bot ID: {}", bot.id);
                }

                if let Some(infrastructure) = &self.infrastructure {
                    // Extract user info
                    let user_info = match extract_user_basic_info(
                        infrastructure.operations().clone(),
                        &self.thread_id,
                    )
                    .await
                    {
                        Ok(info) => {
                            self.user_info = info.clone();
                            info
                        }
                        Err(e) => {
                            log::error!("Failed to extract user info: {}", e);
                            None
                        }
                    };

                    // Create system prompt
                    let system_prompt = create_system_prompt(
                        user_info.as_deref(),
                        Some(bot.system_prompt.as_str()),
                        &self.thread_id,
                    );
                    self.system_prompt = Some(system_prompt);
                    log::info!("Created system prompt");
                }

                log::info!("Selected bot: {}", bot.name);
            } else {
                log::warn!("No bot selected");
                return Ok(());
            }
        }

        // Add user message
        let user_message = ChatMessage::user(input_text);
        self.ui.messages.push(user_message.clone());
        self.ui.invalidate_render_cache(None);
        self.ui.clear_input();
        self.ui.auto_scroll = true;

        log::info!("User message: {}", input_text);

        // Generate response using handle_user_message
        if let Some(infrastructure) = &self.infrastructure {
            if let Some(system_prompt) = &self.system_prompt {
                let operations = infrastructure.operations().clone();
                let api_base_url = self.llm_config.api_base_url.clone();
                let api_key = self.llm_config.api_key.clone();
                let model = self.llm_config.model.clone();
                let system_prompt = system_prompt.clone();
                let user_input = input_text.to_string();
                let thread_id = self.thread_id.clone();
                let history: Vec<ChatMessage> = self.ui.messages.clone();
                let msg_tx = self.message_sender.clone();

                tokio::spawn(async move {
                    match handle_user_message(
                        operations,
                        &api_base_url,
                        &api_key,
                        &model,
                        &system_prompt,
                        &user_input,
                        &thread_id,
                        &history,
                    )
                    .await
                    {
                        Ok(response) => {
                            let _ = msg_tx.send(AppMessage::AssistantResponse(response));
                        }
                        Err(e) => {
                            log::error!("Failed to generate response: {}", e);
                            let _ = msg_tx.send(AppMessage::AssistantResponse(
                                format!("Error: {}", e)
                            ));
                        }
                    }
                });
            } else {
                log::warn!("System prompt not initialized");
            }
        } else {
            log::warn!("Infrastructure not initialized");
        }

        Ok(())
    }

    /// Clear chat
    fn clear_chat(&mut self) {
        log::info!("Clearing chat");
        self.ui.messages.clear();
        self.ui.invalidate_render_cache(None);
        self.ui.scroll_offset = 0;
        self.ui.auto_scroll = true;
    }

    /// Show help
    fn show_help(&mut self) {
        log::info!("Showing help");
        self.ui.help_modal_visible = true;
        self.ui.help_scroll_offset = 0;
    }

    /// Show themes
    fn show_themes(&mut self) {
        log::info!("Showing themes");
        self.ui.theme_modal_visible = true;
    }

    /// Dump chats to clipboard
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

    /// Save conversations to memory on exit
    pub async fn save_conversations_to_memory(&self) -> Result<()> {
        if let Some(infrastructure) = &self.infrastructure {
            let conversations: Vec<(String, String)> = {
                let mut conversations = Vec::new();
                let mut last_user_msg: Option<String> = None;

                for msg in &self.ui.messages {
                    match msg.role {
                        MessageRole::User => {
                            if let Some(user_msg) = last_user_msg.take() {
                                conversations.push((user_msg, String::new()));
                            }
                            last_user_msg = Some(msg.content.clone());
                        }
                        MessageRole::Assistant => {
                            if let Some(user_msg) = last_user_msg.take() {
                                conversations.push((user_msg, msg.content.clone()));
                            }
                        }
                    }
                }

                if let Some(user_msg) = last_user_msg {
                    conversations.push((user_msg, String::new()));
                }

                conversations
            };

            let conversations: Vec<(String, String)> = conversations
                .into_iter()
                .filter(|(user, assistant)| !user.is_empty() && !assistant.is_empty())
                .collect();

            if !conversations.is_empty() {
                log::info!("Saving {} conversations to memory...", conversations.len());
                store_conversations_batch(
                    infrastructure.operations().clone(),
                    &conversations,
                    &self.thread_id,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to save conversations: {}", e))?;
                log::info!("Conversations saved");
            }
        }
        Ok(())
    }

    /// Get all conversations
    pub fn get_conversations(&self) -> Vec<(String, String)> {
        let mut conversations = Vec::new();
        let mut last_user_msg: Option<String> = None;

        for msg in &self.ui.messages {
            match msg.role {
                MessageRole::User => {
                    if let Some(user_msg) = last_user_msg.take() {
                        conversations.push((user_msg, String::new()));
                    }
                    last_user_msg = Some(msg.content.clone());
                }
                MessageRole::Assistant => {
                    if let Some(user_msg) = last_user_msg.take() {
                        conversations.push((user_msg, msg.content.clone()));
                    }
                }
            }
        }

        if let Some(user_msg) = last_user_msg {
            conversations.push((user_msg, String::new()));
        }

        conversations
    }

    /// Get thread ID
    pub fn get_thread_id(&self) -> String {
        self.thread_id.clone()
    }

    /// Handle external message
    pub async fn handle_external_message(&mut self, content: String) -> Result<()> {
        log::info!("Received external message: {}", content);

        if self.current_bot.is_none() {
            if let Some(bot) = self.ui.selected_bot() {
                self.current_bot = Some(bot.clone());

                if let Ok(mut bot_id) = self.current_bot_id.write() {
                    *bot_id = Some(bot.id.clone());
                    log::info!("Updated current bot ID: {}", bot.id);
                }

                if let Some(infrastructure) = &self.infrastructure {
                    let user_info = match extract_user_basic_info(
                        infrastructure.operations().clone(),
                        &self.thread_id,
                    )
                    .await
                    {
                        Ok(info) => {
                            self.user_info = info.clone();
                            info
                        }
                        Err(e) => {
                            log::error!("Failed to extract user info: {}", e);
                            None
                        }
                    };

                    let system_prompt = create_system_prompt(
                        user_info.as_deref(),
                        Some(bot.system_prompt.as_str()),
                        &self.thread_id,
                    );
                    self.system_prompt = Some(system_prompt);
                    log::info!("Created system prompt");
                }

                log::info!("Selected bot: {}", bot.name);
            } else {
                log::warn!("No bot selected");
                return Ok(());
            }
        }

        let user_message = ChatMessage::user(content.clone());
        self.ui.messages.push(user_message.clone());
        self.ui.invalidate_render_cache(None);
        self.ui.auto_scroll = true;

        log::info!("External message added: {}", content);

        if let Some(infrastructure) = &self.infrastructure {
            if let Some(system_prompt) = &self.system_prompt {
                let operations = infrastructure.operations().clone();
                let api_base_url = self.llm_config.api_base_url.clone();
                let api_key = self.llm_config.api_key.clone();
                let model = self.llm_config.model.clone();
                let system_prompt = system_prompt.clone();
                let user_input = content.clone();
                let thread_id = self.thread_id.clone();
                let history: Vec<ChatMessage> = self.ui.messages.clone();
                let msg_tx = self.message_sender.clone();

                tokio::spawn(async move {
                    match handle_user_message(
                        operations,
                        &api_base_url,
                        &api_key,
                        &model,
                        &system_prompt,
                        &user_input,
                        &thread_id,
                        &history,
                    )
                    .await
                    {
                        Ok(response) => {
                            let _ = msg_tx.send(AppMessage::AssistantResponse(response));
                        }
                        Err(e) => {
                            log::error!("Failed to generate response: {}", e);
                            let _ = msg_tx.send(AppMessage::AssistantResponse(
                                format!("Error: {}", e)
                            ));
                        }
                    }
                });
            } else {
                log::warn!("System prompt not initialized");
            }
        } else {
            log::warn!("Infrastructure not initialized");
        }

        Ok(())
    }

    /// Save bot (create or update)
    async fn save_bot(&mut self) -> Result<()> {
        let (name, prompt, password) = self.ui.get_bot_input_data();

        if name.trim().is_empty() {
            log::warn!("Bot name cannot be empty");
            return Ok(());
        }

        if prompt.trim().is_empty() {
            log::warn!("System prompt cannot be empty");
            return Ok(());
        }

        match self.ui.bot_management_state {
            crate::ui::BotManagementState::Creating => {
                let bot_name = name.clone();
                let new_bot = crate::config::BotConfig::new(name, prompt, password);
                self.config_manager.add_bot(new_bot)?;
                log::info!("Created bot: {}", bot_name);
                self.refresh_bot_list()?;
            }
            crate::ui::BotManagementState::Editing => {
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
                        log::info!("Updated bot: {}", bot_name);
                        self.refresh_bot_list()?;
                    }
                }
            }
            _ => {}
        }

        self.ui.bot_management_state = crate::ui::BotManagementState::List;
        Ok(())
    }

    /// Delete bot
    async fn delete_bot(&mut self) -> Result<()> {
        if let Some(index) = self.ui.get_selected_bot_index() {
            if let Some(bot) = self.config_manager.get_bots()?.get(index) {
                let bot_id = bot.id.clone();
                let bot_name = bot.name.clone();

                if self.config_manager.remove_bot(&bot_id)? {
                    log::info!("Deleted bot: {}", bot_name);
                    self.refresh_bot_list()?;

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

        self.ui.bot_management_state = crate::ui::BotManagementState::List;
        Ok(())
    }

    /// Refresh bot list
    fn refresh_bot_list(&mut self) -> Result<()> {
        let bots = self.config_manager.get_bots()?;
        self.ui.set_bot_list(bots);
        Ok(())
    }

    /// Start API server
    fn start_api_server(&self) {
        log::info!("Starting API server...");

        if !self.enable_audio_connect {
            log::warn!("Audio connect not enabled");
            return;
        }

        if self
            .api_server_started
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            log::debug!("API server already started");
            return;
        }

        if let Some(infrastructure) = &self.infrastructure {
            let api_port = std::env::var("TARS_API_PORT")
                .unwrap_or_else(|_| "18199".to_string())
                .parse::<u16>()
                .unwrap_or(18199);

            let _current_bot_id = if let Ok(bot_id) = self.current_bot_id.read() {
                bot_id.clone()
            } else {
                None
            };

            let api_state = crate::api_server::ApiServerState {
                operations: infrastructure.operations().clone(),
                current_bot_id: self.current_bot_id.clone(),
                audio_connect_mode: self.audio_connect_mode.clone(),
                external_message_sender: Some(self.external_message_sender.clone()),
            };

            let api_server_started = self.api_server_started.clone();

            tokio::spawn(async move {
                log::info!("Starting API server task...");
                match crate::api_server::start_api_server(api_state, api_port).await {
                    Ok(_) => {
                        log::info!("API server task completed");
                    }
                    Err(e) => {
                        log::error!("API server error: {}", e);
                    }
                }
            });

            api_server_started.store(true, std::sync::atomic::Ordering::Relaxed);
            log::info!("API server started on port {}", api_port);

            // Health check
            let api_server_started_clone = api_server_started.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                let health_url = format!("http://localhost:{}/api/memory/health", api_port);
                match reqwest::get(&health_url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            log::info!("API server health check passed");
                        } else {
                            log::warn!("API server health check failed: {}", response.status());
                        }
                    }
                    Err(e) => {
                        log::error!("Cannot connect to API server: {}", e);
                        api_server_started_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });
        } else {
            log::warn!("Infrastructure not initialized");
        }
    }

    /// Called when entering chat mode
    pub fn on_enter_chat_mode(&mut self, bot: &BotConfig) {
        log::info!("Entering chat mode, bot: {} (ID: {})", bot.name, bot.id);

        if let Ok(mut bot_id) = self.current_bot_id.write() {
            *bot_id = Some(bot.id.clone());
            log::info!("Updated current bot ID: {}", bot.id);
        }

        self.start_api_server();
    }
}

/// Create default bots
pub fn create_default_bots(config_manager: &mut ConfigManager) -> Result<()> {
    let bots = config_manager.get_bots()?;

    if bots.is_empty() {
        let default_bot = BotConfig::new(
            "Assistant",
            "You are a helpful AI assistant. Answer questions and provide assistance.",
            "",
        );
        config_manager.add_bot(default_bot)?;

        let coder_bot = BotConfig::new(
            "Coder",
            "You are an experienced programmer. Help with coding questions and problems.",
            "",
        );
        config_manager.add_bot(coder_bot)?;

        log::info!("Created default bots");
    }

    Ok(())
}
