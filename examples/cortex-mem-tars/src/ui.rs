use crate::agent::ChatMessage;
use crate::config::BotConfig;
use clipboard::ClipboardProvider;
use ratatui::{
    crossterm::event::{KeyEvent, MouseEvent, MouseEventKind},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use tui_markdown::from_str;
use tui_textarea::TextArea;

/// 应用状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    BotSelection,
    PasswordInput,  // 密码输入状态
    Chat,
}

/// 服务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Initing,   // 初始化中
    Active,    // 服务可用
    Inactive,  // 服务不可用
}

/// 聊天界面状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatState {
    Normal,
    #[allow(dead_code)]
    LogPanel,
    #[allow(dead_code)]
    Selection,
}

/// 主题定义
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub primary_color: Color,
    pub secondary_color: Color,
    pub accent_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub border_color: Color,
}

/// 预设主题
impl Theme {
    pub const DEFAULT: Theme = Theme {
        name: "默认",
        primary_color: Color::Cyan,
        secondary_color: Color::Blue,
        accent_color: Color::Green,
        background_color: Color::Rgb(20, 30, 40),
        text_color: Color::White,
        border_color: Color::Cyan,
    };

    pub const DARK: Theme = Theme {
        name: "暗黑",
        primary_color: Color::Gray,
        secondary_color: Color::DarkGray,
        accent_color: Color::LightCyan,
        background_color: Color::Rgb(10, 10, 15),
        text_color: Color::Rgb(220, 220, 220),
        border_color: Color::Gray,
    };

    pub const FOREST: Theme = Theme {
        name: "森林",
        primary_color: Color::Green,
        secondary_color: Color::Rgb(0, 100, 0),
        accent_color: Color::LightGreen,
        background_color: Color::Rgb(20, 40, 20),
        text_color: Color::Rgb(200, 255, 200),
        border_color: Color::Green,
    };

    pub const OCEAN: Theme = Theme {
        name: "海洋",
        primary_color: Color::Blue,
        secondary_color: Color::Rgb(0, 0, 100),
        accent_color: Color::LightBlue,
        background_color: Color::Rgb(20, 30, 50),
        text_color: Color::Rgb(200, 220, 255),
        border_color: Color::Blue,
    };

    pub const SUNSET: Theme = Theme {
        name: "日落",
        primary_color: Color::Rgb(255, 165, 0),
        secondary_color: Color::Rgb(200, 100, 0),
        accent_color: Color::Rgb(255, 200, 100),
        background_color: Color::Rgb(40, 20, 10),
        text_color: Color::Rgb(255, 240, 200),
        border_color: Color::Rgb(255, 165, 0),
    };

    pub fn all() -> &'static [Theme; 5] {
        &[Self::DEFAULT, Self::DARK, Self::FOREST, Self::OCEAN, Self::SUNSET]
    }
}

/// 应用 UI 状态
pub struct AppUi {
    pub state: AppState,
    pub service_status: ServiceStatus,
    #[allow(dead_code)]
    pub chat_state: ChatState,
    pub bot_list_state: ListState,
    pub bot_list: Vec<BotConfig>,
    pub messages: Vec<ChatMessage>,
    pub input_textarea: TextArea<'static>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,  // 是否自动滚动到底部
    pub log_panel_visible: bool,
    pub log_lines: Vec<String>,
    pub log_scroll_offset: usize,
    pub input_area_width: u16,
    last_key_event: Option<KeyEvent>,
    // 选择模式相关字段
    pub selection_active: bool,
    pub selection_start: Option<(usize, usize)>, // (line_index, char_index)
    pub selection_end: Option<(usize, usize)>,   // (line_index, char_index)
    #[allow(dead_code)]
    pub cursor_position: (usize, usize),         // 当前光标位置 (line_index, char_index)
    // 消息显示区域位置
    pub messages_area: Option<Rect>,
    // Markdown 渲染缓存：存储每条消息的渲染行，避免重复解析
    // Vec 的索引对应 messages 的索引
    pub message_render_cache: Vec<Option<Vec<String>>>,
    // 帮助弹窗相关字段
    pub help_modal_visible: bool,
    pub help_content: Vec<Line<'static>>,
    pub help_scroll_offset: usize,
    // 主题相关字段
    pub current_theme: Theme,
    pub theme_modal_visible: bool,
    pub theme_list_state: ListState,
    // 机器人管理弹窗相关字段
    pub bot_management_modal_visible: bool,
    pub bot_management_state: BotManagementState,
    pub bot_name_input: TextArea<'static>,
    pub bot_prompt_input: TextArea<'static>,
    pub bot_password_input: TextArea<'static>,
    pub bot_management_list_state: ListState,
    pub active_input_field: BotInputField, // 当前活动的输入框
    // 密码验证相关字段
    pub password_input: TextArea<'static>,
    pub pending_bot: Option<BotConfig>, // 等待密码验证的机器人
    // 🎙️ 语音输入状态
    pub audio_input_enabled: bool,
}

/// 机器人管理弹窗状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotManagementState {
    List,      // 机器人列表
    Creating,  // 创建机器人
    Editing,   // 编辑机器人
    ConfirmDelete, // 确认删除
}

/// 机器人输入框类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotInputField {
    Name,     // 机器人名称
    Prompt,   // 系统提示词
    Password, // 访问密码
}

/// 键盘事件处理结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Continue,         // 继续运行
    Quit,             // 退出程序
    SendMessage,      // 发送消息
    ClearChat,        // 清空会话
    ShowHelp,         // 显示帮助
    ShowThemes,       // 显示主题选择
    DumpChats,        // 导出会话到剪贴板
    CreateBot,        // 创建机器人
    EditBot,          // 编辑机器人
    DeleteBot,        // 删除机器人
    SaveBot,          // 保存机器人
    CancelBot,        // 取消机器人操作
    EnableAudioInput, // 启用语音输入
    DisableAudioInput,// 禁用语音输入
}

impl AppUi {
    pub fn new() -> Self {
        let mut bot_list_state = ListState::default();
        bot_list_state.select(Some(0));

        let mut input_textarea = TextArea::default();
        let _ = input_textarea.set_block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::DEFAULT.border_color))
            .title("输入消息或命令 (Enter 发送, 输入 /help 查看命令)"));
        let _ = input_textarea.set_cursor_line_style(Style::default());

        let help_content = Self::parse_help_content();

        let mut theme_list_state = ListState::default();
        theme_list_state.select(Some(0));

        // 初始化机器人管理输入框
        let mut bot_name_input = TextArea::default();
        let _ = bot_name_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("机器人名称"));

        let mut bot_prompt_input = TextArea::default();
        let _ = bot_prompt_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("系统提示词"));

        let mut bot_password_input = TextArea::default();
        let _ = bot_password_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("访问密码"));

        let mut bot_management_list_state = ListState::default();
        bot_management_list_state.select(Some(0));

        // 初始化密码输入框
        let mut password_input = TextArea::default();
        let _ = password_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("请输入密码"));

        Self {
            state: AppState::BotSelection,
            service_status: ServiceStatus::Initing,
            chat_state: ChatState::Normal,
            bot_list_state,
            bot_list: vec![],
            messages: vec![],
            input_textarea,
            scroll_offset: 0,
            auto_scroll: true,
            log_panel_visible: false,
            log_lines: vec![],
            log_scroll_offset: 0,
            input_area_width: 0,
            last_key_event: None,
            selection_active: false,
            selection_start: None,
            selection_end: None,
            cursor_position: (0, 0),
            messages_area: None,
            message_render_cache: vec![],
            help_modal_visible: false,
            help_content,
            help_scroll_offset: 0,
            current_theme: Theme::DEFAULT,
            theme_modal_visible: false,
            theme_list_state,
            bot_management_modal_visible: false,
            bot_management_state: BotManagementState::List,
            bot_name_input,
            bot_prompt_input,
            bot_password_input,
            bot_management_list_state,
            active_input_field: BotInputField::Name,
            password_input,
            pending_bot: None,
            audio_input_enabled: false, // 🎙️ 默认关闭语音输入
        }
    }

    /// 设置机器人列表
    pub fn set_bot_list(&mut self, bots: Vec<BotConfig>) {
        self.bot_list = bots;
        if !self.bot_list.is_empty() {
            self.bot_list_state.select(Some(0));
        } else {
            self.bot_list_state.select(None);
        }
    }

    /// 使渲染缓存失效（在消息变化时调用）
    /// 如果指定了 index，只清除该条消息的缓存；否则清除所有缓存
    pub fn invalidate_render_cache(&mut self, index: Option<usize>) {
        if let Some(idx) = index {
            if idx < self.message_render_cache.len() {
                self.message_render_cache[idx] = None;
            }
        } else {
            self.message_render_cache.clear();
        }
    }

    /// 获取选中的机器人
    pub fn selected_bot(&self) -> Option<&BotConfig> {
        if let Some(index) = self.bot_list_state.selected() {
            self.bot_list.get(index)
        } else {
            None
        }
    }

    /// 处理键盘事件
    pub fn handle_key_event(&mut self, key: KeyEvent) -> KeyAction {
        // 事件去重：如果和上一次事件完全相同，则忽略
        if let Some(last_key) = self.last_key_event {
            if last_key.code == key.code && last_key.modifiers == key.modifiers {
                // log::debug!("忽略重复事件: {:?}", key);
                self.last_key_event = None;
                return KeyAction::Continue;
            }
        }

        self.last_key_event = Some(key);

        // 优先级：密码输入 > 机器人管理弹窗 > 正常状态
        if self.state == AppState::PasswordInput {
            return self.handle_password_input_key(key);
        }

        if self.bot_management_modal_visible {
            return self.handle_bot_management_key(key);
        }

        match self.state {
            AppState::BotSelection => {
                if self.handle_bot_selection_key(key) {
                    KeyAction::Continue
                } else {
                    KeyAction::Quit
                }
            }
            AppState::Chat => self.handle_chat_key(key),
            _ => KeyAction::Continue,
        }
    }

    /// 处理机器人选择界面的键盘事件
    fn handle_bot_selection_key(&mut self, key: KeyEvent) -> bool {
        use ratatui::crossterm::event::KeyCode;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.bot_list_state.selected() {
                    if selected > 0 {
                        self.bot_list_state.select(Some(selected - 1));
                        log::debug!("选择上一个机器人");
                    }
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.bot_list_state.selected() {
                    if selected < self.bot_list.len().saturating_sub(1) {
                        self.bot_list_state.select(Some(selected + 1));
                        log::debug!("选择下一个机器人");
                    }
                }
                true
            }
            KeyCode::Enter => {
                if let Some(bot) = self.selected_bot() {
                    log::info!("选择机器人: {}", bot.name);
                    // 检查是否需要密码验证
                    if bot.access_password.trim().is_empty() {
                        // 密码为空，直接进入聊天
                        self.state = AppState::Chat;
                    } else {
                        // 需要密码验证
                        self.pending_bot = Some(bot.clone());
                        self.password_input = TextArea::default();
                        let _ = self.password_input.set_block(Block::default()
                            .borders(Borders::ALL)
                            .title("请输入密码"));
                        self.state = AppState::PasswordInput;
                    }
                }
                true
            }
            KeyCode::Char('m') => {
                // 打开机器人管理
                log::info!("打开机器人管理");
                self.bot_management_modal_visible = true;
                self.bot_management_state = BotManagementState::List;
                self.bot_management_list_state.select(Some(0));
                true
            }
            KeyCode::Char('q') => {
                log::info!("用户按 q 退出");
                false
            }
            KeyCode::Char('c') if key.modifiers.contains(ratatui::crossterm::event::KeyModifiers::CONTROL) => {
                log::info!("用户按 Ctrl-C 退出");
                false
            }
            _ => true,
        }
    }

    /// 处理密码输入界面的键盘事件
    fn handle_password_input_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                // 取消密码输入，返回机器人选择界面
                self.state = AppState::BotSelection;
                self.pending_bot = None;
                self.password_input = TextArea::default();
                KeyAction::Continue
            }
            KeyCode::Enter => {
                // 验证密码
                let input_password = self.password_input.lines().first().map(|s| s.trim()).unwrap_or("");
                if let Some(bot) = &self.pending_bot {
                    if input_password == bot.access_password.trim() {
                        // 密码正确，进入聊天
                        log::info!("密码验证成功");
                        self.state = AppState::Chat;
                        self.pending_bot = None;
                        self.password_input = TextArea::default();
                        KeyAction::Continue
                    } else {
                        // 密码错误
                        log::warn!("密码错误");
                        self.password_input = TextArea::default();
                        let _ = self.password_input.set_block(Block::default()
                            .borders(Borders::ALL)
                            .title("密码错误，请重新输入"));
                        KeyAction::Continue
                    }
                } else {
                    self.state = AppState::BotSelection;
                    KeyAction::Continue
                }
            }
            _ => {
                // 让密码输入框处理按键
                self.password_input.input(key);
                KeyAction::Continue
            }
        }
    }

    /// 处理聊天界面的键盘事件
    fn handle_chat_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

        // 如果帮助弹窗打开，只处理弹窗相关的按键
        if self.help_modal_visible {
            return self.handle_help_modal_key(key);
        }

        // 如果主题弹窗打开，只处理主题弹窗相关的按键
        if self.theme_modal_visible {
            return self.handle_theme_modal_key(key);
        }

        if self.log_panel_visible {
            log::debug!("日志面板打开，处理日志面板键盘事件");
            if self.handle_log_panel_key(key) {
                KeyAction::Continue
            } else {
                KeyAction::Quit
            }
        } else {
            match key.code {
                KeyCode::Enter => {
                    if key.modifiers.is_empty() {
                        // Enter 发送消息
                        log::debug!("Enter: 准备发送消息");
                        let text = self.get_input_text();
                        if !text.trim().is_empty() {
                            KeyAction::SendMessage
                        } else {
                            KeyAction::Continue
                        }
                    } else {
                        // Shift+Enter 换行
                        self.input_textarea.input(key);
                        KeyAction::Continue
                    }
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    log::debug!("Ctrl-C: 退出");
                    KeyAction::Quit
                }
                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.log_panel_visible = !self.log_panel_visible;
                    KeyAction::Continue
                }
                KeyCode::Esc => {
                    self.log_panel_visible = false;
                    // 清除选择
                    self.selection_active = false;
                    self.selection_start = None;
                    self.selection_end = None;
                    KeyAction::Continue
                }
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => {
                    // 先让 tui-textarea 处理输入
                    self.input_textarea.input(key);
                    // 尝试自动换行
                    self.handle_auto_wrap();
                    KeyAction::Continue
                }
                _ => {
                    self.input_textarea.input(key);
                    KeyAction::Continue
                }
            }
        }
    }

    /// 处理自动换行 - 检查所有行的显示宽度，必要时插入换行
    fn handle_auto_wrap(&mut self) {
        // 使用实际的输入框宽度（减去边框的2个字符）
        let max_display_width = if self.input_area_width > 2 {
            (self.input_area_width as usize).saturating_sub(2)
        } else {
            74 // 默认宽度
        };

        loop {
            let lines = self.input_textarea.lines().to_vec();
            let (cursor_row, cursor_col) = self.input_textarea.cursor();

            // 检查所有行，找到第一行超过宽度的行
            let mut wrap_line_idx = None;
            let mut wrap_pos = 0;

            for (line_idx, line) in lines.iter().enumerate() {
                let line_width: usize = line.chars()
                    .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                    .sum();

                if line_width > max_display_width {
                    // 找到这一行中需要换行的位置
                    let chars: Vec<char> = line.chars().collect();
                    let mut current_width = 0usize;

                    for (char_idx, c) in chars.iter().enumerate() {
                        let char_width = unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
                        current_width += char_width;

                        if current_width > max_display_width {
                            // 找到前一个空格的位置
                            wrap_pos = char_idx;
                            for j in (0..char_idx).rev() {
                                if chars[j].is_whitespace() {
                                    wrap_pos = j;
                                    break;
                                }
                            }
                            if wrap_pos == 0 {
                                wrap_pos = 1;
                            }
                            wrap_line_idx = Some(line_idx);
                            break;
                        }
                    }
                    break;
                }
            }

            // 如果没有需要换行的行，退出
            if wrap_line_idx.is_none() {
                return;
            }

            let line_idx = wrap_line_idx.unwrap();
            let line = &lines[line_idx];
            let chars: Vec<char> = line.chars().collect();

            log::debug!("[AUTO_WRAP] line {} needs wrap, pos {}", line_idx, wrap_pos);

            // 分割这一行
            let prefix: String = chars[..wrap_pos].iter().collect();
            let suffix: String = chars[wrap_pos..].iter().collect();

            // 构建新的行列表
            let mut new_lines = lines[..line_idx].to_vec();
            new_lines.push(prefix.trim_end().to_string());
            new_lines.push(suffix.trim_start().to_string());
            if line_idx + 1 < lines.len() {
                new_lines.extend_from_slice(&lines[line_idx + 1..]);
            }

            // 重新创建 TextArea
            let mut new_textarea = TextArea::from(new_lines.iter().cloned());
            let _ = new_textarea.set_block(Block::default()
                .borders(Borders::ALL)
                .title("输入消息或命令 (Enter 发送, 输入 /help 查看命令)"));
            let _ = new_textarea.set_cursor_line_style(Style::default());

            // 重新计算光标位置
            let new_cursor_row = if line_idx < cursor_row {
                cursor_row + 1
            } else if line_idx == cursor_row {
                if cursor_col > wrap_pos {
                    line_idx + 1
                } else {
                    line_idx
                }
            } else {
                cursor_row
            };

            let new_cursor_col = if cursor_row == line_idx && cursor_col > wrap_pos {
                let suffix_prefix: String = chars[wrap_pos..cursor_col].iter().collect();
                suffix_prefix.chars()
                    .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                    .sum()
            } else if cursor_row == line_idx {
                cursor_col
            } else {
                cursor_col
            };

            // 移动光标到正确位置
            for _ in 0..new_cursor_row {
                new_textarea.move_cursor(tui_textarea::CursorMove::Down);
            }
            for _ in 0..new_cursor_col {
                new_textarea.move_cursor(tui_textarea::CursorMove::Forward);
            }

            self.input_textarea = new_textarea;
        }
    }

    /// 处理日志面板的键盘事件
    fn handle_log_panel_key(&mut self, key: KeyEvent) -> bool {
        use ratatui::crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                self.log_panel_visible = false;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.log_scroll_offset > 0 {
                    self.log_scroll_offset -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.log_scroll_offset < self.log_lines.len().saturating_sub(1) {
                    self.log_scroll_offset += 1;
                }
                true
            }
            KeyCode::PageUp => {
                self.log_scroll_offset = self.log_scroll_offset.saturating_sub(10);
                true
            }
            KeyCode::PageDown => {
                self.log_scroll_offset = self
                    .log_scroll_offset
                    .saturating_add(10)
                    .min(self.log_lines.len().saturating_sub(1));
                true
            }
            KeyCode::Home => {
                self.log_scroll_offset = 0;
                true
            }
            KeyCode::End => {
                self.log_scroll_offset = self.log_lines.len().saturating_sub(1);
                true
            }
            KeyCode::Char('l') => {
                self.log_panel_visible = false;
                true
            }
            _ => true,
        }
    }

    /// 处理帮助弹窗的键盘事件
    fn handle_help_modal_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                log::debug!("关闭帮助弹窗");
                self.help_modal_visible = false;
                KeyAction::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.help_scroll_offset > 0 {
                    self.help_scroll_offset -= 1;
                }
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let visible_lines = 20; // 弹窗可见行数
                if self.help_scroll_offset < self.help_content.len().saturating_sub(visible_lines) {
                    self.help_scroll_offset += 1;
                }
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_sub(10);
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                let visible_lines = 20; // 弹窗可见行数
                self.help_scroll_offset = self.help_scroll_offset
                    .saturating_add(10)
                    .min(self.help_content.len().saturating_sub(visible_lines));
                KeyAction::Continue
            }
            KeyCode::Home => {
                self.help_scroll_offset = 0;
                KeyAction::Continue
            }
            KeyCode::End => {
                let visible_lines = 20; // 弹窗可见行数
                self.help_scroll_offset = self.help_content.len().saturating_sub(visible_lines);
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    /// 处理主题弹窗的键盘事件
    fn handle_theme_modal_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                log::debug!("关闭主题弹窗");
                self.theme_modal_visible = false;
                KeyAction::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.theme_list_state.selected() {
                    if selected > 0 {
                        self.theme_list_state.select(Some(selected - 1));
                    }
                }
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.theme_list_state.selected() {
                    if selected < Theme::all().len().saturating_sub(1) {
                        self.theme_list_state.select(Some(selected + 1));
                    }
                }
                KeyAction::Continue
            }
            KeyCode::Enter => {
                if let Some(index) = self.theme_list_state.selected() {
                    if let Some(theme) = Theme::all().get(index) {
                        self.current_theme = *theme;
                        log::info!("切换主题: {}", theme.name);

                        // 更新输入框样式
                        let _ = self.input_textarea.set_block(Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(self.current_theme.border_color))
                            .title("输入消息或命令 (Enter 发送, 输入 /help 查看命令)"));

                        self.theme_modal_visible = false;
                    }
                }
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    /// 复制选中的内容到剪贴板
    fn copy_selection(&mut self) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let selected_text = self.get_selected_text(start, end);

            if !selected_text.is_empty() {
                match clipboard::ClipboardContext::new() {
                    Ok(mut ctx) => {
                        match ctx.set_contents(selected_text.clone()) {
                            Ok(_) => {
                                log::info!("已复制 {} 个字符到剪贴板", selected_text.len());
                            }
                            Err(e) => {
                                log::error!("复制到剪贴板失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("无法访问剪贴板: {}", e);
                    }
                }
            }
        }
    }

    /// 获取选中的文本
    fn get_selected_text(&self, start: (usize, usize), end: (usize, usize)) -> String {
        let mut result = String::new();
        let all_lines = self.get_all_rendered_lines();

        let start_line = start.0.min(end.0);
        let end_line = end.0.max(start.0);
        let start_col = if start.0 <= end.0 { start.1 } else { end.1 };
        let end_col = if start.0 <= end.0 { end.1 } else { start.1 };

        for line_idx in start_line..=end_line {
            if line_idx < all_lines.len() {
                let line = &all_lines[line_idx];
                let chars: Vec<char> = line.chars().collect();
                let char_len = chars.len();

                if line_idx == start_line && line_idx == end_line {
                    // 单行选择
                    if start_col < char_len && end_col <= char_len && start_col < end_col {
                        let selected: String = chars[start_col..end_col].iter().collect();
                        result.push_str(&selected);
                    }
                } else if line_idx == start_line {
                    // 起始行
                    if start_col < char_len {
                        let selected: String = chars[start_col..].iter().collect();
                        result.push_str(&selected);
                    }
                    result.push('\n');
                } else if line_idx == end_line {
                    // 结束行
                    if end_col <= char_len {
                        let selected: String = chars[..end_col].iter().collect();
                        result.push_str(&selected);
                    }
                } else {
                    // 中间行
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }

        result
    }

    /// 获取所有渲染的行文本
    fn get_all_rendered_lines(&self) -> Vec<String> {
        let mut all_lines: Vec<String> = vec![];

        for message in &self.messages {
            // 角色标签行
            let role_label = match message.role {
                crate::agent::MessageRole::User => "[You]",
                crate::agent::MessageRole::Assistant => "[AI]",
                crate::agent::MessageRole::System => "[System]",
            };
            all_lines.push(role_label.to_string());

            // 渲染 Markdown 内容（与 render_messages 保持一致）
            let markdown_text = from_str(&message.content);
            for line in markdown_text.lines {
                let line_text: String = line.spans.iter().map(|s| s.content.clone()).collect();
                all_lines.push(line_text);
            }

            // 空行分隔
            all_lines.push(String::new());
        }

        all_lines
    }

    /// 处理鼠标事件
    pub fn handle_mouse_event(&mut self, event: MouseEvent, _area: Rect) -> bool {
        if self.state != AppState::Chat {
            return true;
        }

        // 如果帮助弹窗打开，处理帮助弹窗的滚轮事件
        if self.help_modal_visible {
            // 动态计算弹窗高度和可见行数（与 render_help_modal 保持一致）
            let modal_height = _area.height.saturating_sub(10).min(25);
            let visible_lines = modal_height.saturating_sub(4) as usize;
            let max_scroll = self.help_content.len().saturating_sub(visible_lines);

            match event.kind {
                MouseEventKind::ScrollUp => {
                    if self.help_scroll_offset > 0 {
                        self.help_scroll_offset = self.help_scroll_offset.saturating_sub(3);
                    }
                }
                MouseEventKind::ScrollDown => {
                    if self.help_scroll_offset < max_scroll {
                        self.help_scroll_offset = self.help_scroll_offset.saturating_add(3).min(max_scroll);
                    }
                }
                _ => {}
            }
            return true;
        }

        // 使用保存的消息区域
        let messages_area = match self.messages_area {
            Some(area) => area,
            None => return true,
        };

        match event.kind {
            MouseEventKind::ScrollUp => {
                if self.log_panel_visible {
                    if self.log_scroll_offset > 0 {
                        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(3);
                    }
                } else if self.scroll_offset > 0 {
                    self.scroll_offset = self.scroll_offset.saturating_sub(3);
                    // 用户手动滚动，禁用自动滚动
                    self.auto_scroll = false;
                }
                true
            }
            MouseEventKind::ScrollDown => {
                if self.log_panel_visible {
                    self.log_scroll_offset = self.log_scroll_offset.saturating_add(3);
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_add(3);
                    // 用户手动滚动，禁用自动滚动
                    self.auto_scroll = false;
                }
                true
            }
            MouseEventKind::Down(but) if but == ratatui::crossterm::event::MouseButton::Left => {
                // 鼠标左键按下，开始选择
                let (line_idx, col_idx) = self.mouse_to_text_position(event, messages_area);
                self.selection_active = true;
                self.selection_start = Some((line_idx, col_idx));
                self.selection_end = Some((line_idx, col_idx));
                true
            }
            MouseEventKind::Drag(but) if but == ratatui::crossterm::event::MouseButton::Left => {
                // 鼠标拖拽，更新选择
                let (line_idx, col_idx) = self.mouse_to_text_position(event, messages_area);
                self.selection_end = Some((line_idx, col_idx));
                true
            }
            MouseEventKind::Up(but) if but == ratatui::crossterm::event::MouseButton::Left => {

                // 保持选择状态，用户可以继续操作
                true
            }
            MouseEventKind::Up(but) if but == ratatui::crossterm::event::MouseButton::Right => {
                // 鼠标右键释放，复制选中的文本
                log::debug!("鼠标右键，复制选中的文本");
                if self.selection_active {
                    self.copy_selection();
                }
                true
            }
            _ => true,
        }
    }

    /// 将鼠标坐标转换为文本位置 (line_index, char_index)
    fn mouse_to_text_position(&self, event: MouseEvent, area: Rect) -> (usize, usize) {
        // 计算相对于消息区域的坐标
        let content_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        // 检查鼠标是否在消息区域内
        if event.row < content_area.top() || event.row >= content_area.bottom() ||
           event.column < content_area.left() || event.column >= content_area.right() {
            log::debug!("鼠标不在消息区域内");
            return (self.scroll_offset, 0);
        }

        // 计算行索引（考虑滚动偏移）
        let relative_row = event.row.saturating_sub(content_area.top());
        let line_idx = self.scroll_offset + relative_row as usize;

        // 计算列索引
        let relative_col = event.column.saturating_sub(content_area.left());
        let col_idx = relative_col as usize;

        // 获取实际行的文本，确保列索引不超出范围
        let all_lines = self.get_all_rendered_lines();
        if line_idx < all_lines.len() {
            let line_len = all_lines[line_idx].len();
            (line_idx, col_idx.min(line_len))
        } else {
            log::debug!("行索引超出范围: {} >= {}", line_idx, all_lines.len());
            (line_idx, 0)
        }
    }

    /// 渲染 UI
    pub fn render(&mut self, frame: &mut Frame) {
        // 如果机器人管理弹窗可见，只渲染弹窗，不渲染主界面
        if self.bot_management_modal_visible {
            self.render_bot_management_modal(frame);
            return;
        }

        match self.state {
            AppState::BotSelection => self.render_bot_selection(frame),
            AppState::PasswordInput => self.render_password_input(frame),
            AppState::Chat => self.render_chat(frame),
        }
    }

    /// 渲染机器人选择界面
    fn render_bot_selection(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // 标题
        let title = Paragraph::new("选择机器人")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(title, chunks[0]);

        // 机器人列表
        let items: Vec<ListItem> = self
            .bot_list
            .iter()
            .map(|bot| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        bot.name.clone(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" - "),
                    Span::styled(
                        format!("{}...", &bot.system_prompt.chars().take(40).collect::<String>()),
                        Style::default().fg(Color::Gray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("可用机器人"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::REVERSED),
            );

        frame.render_stateful_widget(list, chunks[1], &mut self.bot_list_state);

        // 帮助提示
        let help = Paragraph::new("↑/↓ 或 j/k: 选择 | Enter: 进入 | m: 管理机器人 | q 或 Ctrl-C: 退出")
            .alignment(Alignment::Center);

        frame.render_widget(help, chunks[2]);
    }

    /// 渲染密码输入界面
    fn render_password_input(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // 标题
        let bot_name = self.pending_bot.as_ref().map(|b| b.name.as_str()).unwrap_or("未知");
        let title = Paragraph::new(format!("访问机器人: {}", bot_name))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(title, chunks[0]);

        // 密码输入框
        frame.render_widget(&self.password_input, chunks[1]);

        // 帮助提示
        let help = Paragraph::new("Enter: 确认 | Esc: 取消")
            .alignment(Alignment::Center);

        frame.render_widget(help, chunks[2]);
    }

    /// 渲染聊天界面
    fn render_chat(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if self.log_panel_visible {
            self.render_chat_with_log_panel(frame, area);
        } else {
            self.render_chat_normal(frame, area);
        }

        // 如果帮助弹窗可见，渲染弹窗
        if self.help_modal_visible {
            self.render_help_modal(frame);
        }

        // 如果主题弹窗可见，渲染弹窗
        if self.theme_modal_visible {
            self.render_theme_modal(frame);
        }
    }

    /// 渲染普通聊天界面
    fn render_chat_normal(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(8),
            ])
            .split(area);

        // 创建简洁的标题文字
        let bot_name = self.selected_bot().map(|b| b.name.as_str()).unwrap_or("未知");
        
        // 🎙️ 添加语音输入状态指示
        let audio_status = if self.audio_input_enabled {
            Span::styled(
                " [🎙️ 语音输入开启]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " [🔇 语音输入关闭]",
                Style::default()
                    .fg(Color::DarkGray),
            )
        };
        
        let title_line = Line::from(vec![
            Span::styled(
                "Cortex TARS AI Program",
                Style::default()
                    .fg(self.current_theme.primary_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" (当前角色: {})", bot_name),
                Style::default()
                    .fg(self.current_theme.accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            audio_status, // 🎙️ 添加语音状态
        ]);

        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(self.current_theme.primary_color)
                            .add_modifier(Modifier::BOLD)
                    )
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(
                        Style::default()
                            .fg(match self.service_status {
                                ServiceStatus::Initing => Color::Blue,
                                ServiceStatus::Active => Color::Green,
                                ServiceStatus::Inactive => Color::Red,
                            })
                            .add_modifier(Modifier::BOLD)
                    )
                    .title(match self.service_status {
                        ServiceStatus::Initing => " [ SYSTEM INITING ] ",
                        ServiceStatus::Active => " [ SYSTEM ACTIVE ] ",
                        ServiceStatus::Inactive => " [ SYSTEM INACTIVE ] ",
                    })
            )
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(self.current_theme.text_color)
                    .bg(self.current_theme.background_color)
            );

        frame.render_widget(title, chunks[0]);

        // 消息显示区域
        let messages_area = chunks[1];
        self.messages_area = Some(messages_area);
        self.render_messages(frame, messages_area);

        // 输入区域
        self.render_input(frame, chunks[2]);
    }

    /// 渲染带日志面板的聊天界面
    fn render_chat_with_log_panel(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // 创建简洁的标题文字
        let bot_name = self.selected_bot().map(|b| b.name.as_str()).unwrap_or("未知");
        let title_line = Line::from(vec![
            Span::styled(
                "Cortex TARS AI Program",
                Style::default()
                    .fg(self.current_theme.primary_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" (当前角色: {})", bot_name),
                Style::default()
                    .fg(self.current_theme.accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(self.current_theme.primary_color)
                            .add_modifier(Modifier::BOLD)
                    )
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(
                        Style::default()
                            .fg(match self.service_status {
                                ServiceStatus::Initing => Color::Blue,
                                ServiceStatus::Active => Color::Green,
                                ServiceStatus::Inactive => Color::Red,
                            })
                            .add_modifier(Modifier::BOLD)
                    )
                    .title(match self.service_status {
                        ServiceStatus::Initing => " [ SYSTEM INITING ] ",
                        ServiceStatus::Active => " [ SYSTEM ACTIVE ] ",
                        ServiceStatus::Inactive => " [ SYSTEM INACTIVE ] ",
                    })
            )
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(self.current_theme.text_color)
                    .bg(self.current_theme.background_color)
            );

        frame.render_widget(title, chunks[0]);

        // 消息显示区域
        let messages_area = chunks[1];
        self.messages_area = Some(messages_area);
        self.render_messages(frame, messages_area);

        // 日志面板
        self.render_log_panel(frame, chunks[2]);
    }

    /// 渲染消息
    fn render_messages(&mut self, frame: &mut Frame, area: Rect) {
        // 使用 tui-markdown 渲染每个消息
        let content_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        // 确保缓存大小与消息数量一致
        while self.message_render_cache.len() < self.messages.len() {
            self.message_render_cache.push(None);
        }
        if self.message_render_cache.len() > self.messages.len() {
            self.message_render_cache.truncate(self.messages.len());
        }

        // 收集所有消息的渲染行
        let mut all_lines: Vec<Line> = vec![];

        for (idx, message) in self.messages.iter().enumerate() {
            let role_label = match message.role {
                crate::agent::MessageRole::User => "You",
                crate::agent::MessageRole::Assistant => "TARS AI",
                crate::agent::MessageRole::System => "System",
            };

            let role_color = match message.role {
                crate::agent::MessageRole::User => self.current_theme.accent_color,
                crate::agent::MessageRole::Assistant => self.current_theme.primary_color,
                crate::agent::MessageRole::System => Color::Yellow,
            };

            // 格式化时间戳
            let time_str = message.timestamp.format("%H:%M:%S").to_string();

            // 添加角色标签和时间戳
            all_lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}]", role_label),
                    Style::default()
                        .fg(role_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", time_str),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::DIM),
                ),
            ]));

            // 渲染 Markdown 内容（使用缓存）
            let content_lines = if let Some(cached) = &self.message_render_cache[idx] {
                cached.clone()
            } else {
                // 解析 Markdown 并缓存
                let markdown_text = from_str(&message.content);
                let lines: Vec<String> = markdown_text.lines.iter()
                    .map(|line| line.spans.iter().map(|s| s.content.clone()).collect::<String>())
                    .collect();
                self.message_render_cache[idx] = Some(lines.clone());
                lines
            };

            for line in content_lines {
                all_lines.push(Line::from(vec![Span::raw(line)]));
            }

            // 添加空行分隔
            all_lines.push(Line::from(""));
        }

        // 🔧 修复：手动计算wrap后的实际行数
        // 遍历所有行，计算每行在给定宽度下会被换成几行
        let available_width = content_area.width as usize;
        let mut total_lines = 0;
        
        for line in &all_lines {
            // 计算行的实际显示宽度（考虑中文等宽字符）
            let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            let line_width = unicode_width::UnicodeWidthStr::width(line_text.as_str());
            
            // 计算这一行会被wrap成几行
            if line_width == 0 {
                total_lines += 1;  // 空行也占一行
            } else if available_width > 0 {
                total_lines += (line_width + available_width - 1) / available_width;  // 向上取整
            } else {
                total_lines += 1;
            }
        }
        
        let visible_lines = content_area.height as usize;
        let max_scroll = total_lines.saturating_sub(visible_lines);

        // 如果启用了自动滚动，始终滚动到底部
        if self.auto_scroll {
            self.scroll_offset = max_scroll;
        } else {
            // 限制 scroll_offset 在有效范围内
            if self.scroll_offset > max_scroll {
                self.scroll_offset = max_scroll;
            }
        }

        // 渲染边框
        let title = "交互信息 (鼠标拖拽选择, Esc 清除选择)";
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title);
        frame.render_widget(block, area);

        // 🔧 修复：使用Paragraph::scroll()方法而不是手动skip/take
        // 这样Paragraph会正确处理wrap后的滚动
        let paragraph_with_scroll = if self.selection_active {
            // 选择模式下仍需要手动处理（因为需要高亮）
            let display_lines = self.apply_selection_highlight(all_lines, self.scroll_offset, visible_lines);
            Paragraph::new(display_lines).wrap(Wrap { trim: false })
        } else {
            // 正常模式使用Paragraph的内置滚动
            Paragraph::new(all_lines)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll_offset as u16, 0))  // 使用scroll方法
        };
        
        frame.render_widget(paragraph_with_scroll, content_area);

        // 渲染滚动条
        if total_lines > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            // 🔧 修复：使用实际的total_lines（wrap后的行数）
            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(self.scroll_offset)
                .viewport_content_length(visible_lines);

            let scrollbar_area = area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            });

            frame.render_stateful_widget(
                scrollbar,
                scrollbar_area,
                &mut scrollbar_state,
            );
        }
    }

    /// 应用选择高亮
    fn apply_selection_highlight<'a>(&self, lines: Vec<Line<'a>>, scroll_offset: usize, visible_lines: usize) -> Vec<Line<'a>> {
        let (start, end) = match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) => (s, e),
            _ => return lines.into_iter().skip(scroll_offset).take(visible_lines).collect(),
        };

        // 如果选择范围完全在可见区域之外，直接返回
        let start_line = start.0.min(end.0);
        let end_line = end.0.max(start.0);

        // 可见区域的行范围是 [scroll_offset, scroll_offset + visible_lines)
        let visible_start = scroll_offset;
        let visible_end = scroll_offset + visible_lines;

        // 如果选择区域和可见区域没有重叠，直接返回
        if end_line < visible_start || start_line >= visible_end {
            // log::debug!("选择区域和可见区域没有重叠，直接返回");
            return lines.into_iter().skip(scroll_offset).take(visible_lines).collect();
        }

        let start_col = if start.0 <= end.0 { start.1 } else { end.1 };
        let end_col = if start.0 <= end.0 { end.1 } else { start.1 };

        // 使用反色样式使高亮更明显
        let highlight_style = Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD);

        let mut highlighted_count = 0;
        let mut total_processed = 0;

        let result = lines
            .into_iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_lines)
            .map(|(original_idx, line)| {
                total_processed += 1;
                // original_idx 是原始的行索引（从 0 开始）
                // skip(scroll_offset) 后，visible_idx 从 0 开始
                let in_range = original_idx >= start_line && original_idx <= end_line;

                if in_range {
                    // 这一行在选择范围内
                    highlighted_count += 1;
                    let line_text: String = line.spans.iter().map(|s| s.content.clone()).collect();
                    let chars: Vec<char> = line_text.chars().collect();
                    let char_len = chars.len();

                    if original_idx == start_line && original_idx == end_line {
                        // 单行选择
                        let safe_start_col = start_col.min(char_len);
                        let safe_end_col = end_col.min(char_len);
                        if safe_start_col < char_len && safe_end_col <= char_len && safe_start_col < safe_end_col {
                            let before: String = chars[..safe_start_col].iter().collect();
                            let selected: String = chars[safe_start_col..safe_end_col].iter().collect();
                            let after: String = chars[safe_end_col..].iter().collect();

                            Line::from(vec![
                                Span::raw(before),
                                Span::styled(selected, highlight_style),
                                Span::raw(after),
                            ])
                        } else {
                            line
                        }
                    } else if original_idx == start_line {
                        // 起始行
                        let safe_start_col = start_col.min(char_len);
                        if safe_start_col < char_len {
                            let before: String = chars[..safe_start_col].iter().collect();
                            let selected: String = chars[safe_start_col..].iter().collect();

                            Line::from(vec![
                                Span::raw(before),
                                Span::styled(selected, highlight_style),
                            ])
                        } else {
                            line
                        }
                    } else if original_idx == end_line {
                        // 结束行
                        let safe_end_col = end_col.min(char_len);
                        if safe_end_col <= char_len {
                            let selected: String = chars[..safe_end_col].iter().collect();
                            let after: String = chars[safe_end_col..].iter().collect();

                            Line::from(vec![
                                Span::styled(selected, highlight_style),
                                Span::raw(after),
                            ])
                        } else {
                            line
                        }
                    } else {
                        // 中间行，整行高亮
                        Line::from(vec![Span::styled(
                            line_text,
                            highlight_style,
                        )])
                    }
                } else {
                    line
                }
            })
            .collect();

        result
    }

    /// 渲染输入框 - 使用 tui-textarea
    fn render_input(&mut self, frame: &mut Frame, area: Rect) {
        // 保存输入框可用宽度（减去边框的2个字符）
        self.input_area_width = area.width.saturating_sub(2);
        frame.render_widget(&self.input_textarea, area);
    }

    /// 渲染日志面板
    fn render_log_panel(&mut self, frame: &mut Frame, area: Rect) {
        let visible_lines = area.height as usize;
        let max_scroll = self.log_lines.len().saturating_sub(visible_lines);
        self.log_scroll_offset = self.log_scroll_offset.min(max_scroll);

        let display_lines: Vec<Line> = self
            .log_lines
            .iter()
            .skip(self.log_scroll_offset)
            .take(visible_lines)
            .map(|line| {
                let style = if line.contains("ERROR") {
                    Style::default().fg(Color::Red)
                } else if line.contains("WARN") {
                    Style::default().fg(Color::Yellow)
                } else if line.contains("INFO") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(line.clone(), style))
            })
            .collect();

        let paragraph = Paragraph::new(display_lines)
            .block(Block::default().borders(Borders::ALL).title("日志 (Esc 关闭)"))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        // 渲染滚动条
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        // 🔧 修复：正确设置ScrollbarState，包括viewport_content_length
        let mut scrollbar_state = ScrollbarState::new(self.log_lines.len())
            .position(self.log_scroll_offset)
            .viewport_content_length(visible_lines);  // 关键：设置可见行数

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        frame.render_stateful_widget(
            scrollbar,
            scrollbar_area,
            &mut scrollbar_state,
        );
    }

    /// 渲染帮助弹窗
    fn render_help_modal(&mut self, frame: &mut Frame) {
        // 计算弹窗大小（居中显示）
        let area = frame.area();
        let modal_width = area.width.saturating_sub(20).min(80);
        let modal_height = area.height.saturating_sub(10).min(25);

        let x = (area.width - modal_width) / 2;
        let y = (area.height - modal_height) / 2;

        let modal_area = Rect::new(x, y, modal_width, modal_height);

        // 创建半透明背景遮罩（使用深灰色）
        let overlay_area = area;
        let overlay_block = Block::default()
            .style(Style::default().bg(Color::Rgb(20, 20, 20)));

        frame.render_widget(overlay_block, overlay_area);

        // 渲染弹窗内容
        let visible_lines = modal_height.saturating_sub(4) as usize; // 减去边框和标题
        let max_scroll = self.help_content.len().saturating_sub(visible_lines);
        self.help_scroll_offset = self.help_scroll_offset.min(max_scroll);

        let display_lines: Vec<Line> = self
            .help_content
            .iter()
            .skip(self.help_scroll_offset)
            .take(visible_lines)
            .cloned()
            .collect();

        let paragraph = Paragraph::new(display_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.current_theme.border_color))
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(Style::default().fg(self.current_theme.primary_color).add_modifier(Modifier::BOLD))
                    .title(" 帮助信息 (Esc 关闭) ")
                    .style(Style::default().bg(self.current_theme.background_color))
            )
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, modal_area);

        // 渲染滚动条
        if self.help_content.len() > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            // 🔧 修复：正确设置ScrollbarState，包括viewport_content_length
            let mut scrollbar_state = ScrollbarState::new(self.help_content.len())
                .position(self.help_scroll_offset)
                .viewport_content_length(visible_lines);  // 关键：设置可见行数

            let scrollbar_area = modal_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            });

            frame.render_stateful_widget(
                scrollbar,
                scrollbar_area,
                &mut scrollbar_state,
            );
        }
    }

    /// 渲染主题选择弹窗
    fn render_theme_modal(&mut self, frame: &mut Frame) {
        // 计算弹窗大小（居中显示）
        let area = frame.area();
        let modal_width = 50;
        let modal_height = 15;

        let x = (area.width - modal_width) / 2;
        let y = (area.height - modal_height) / 2;

        let modal_area = Rect::new(x, y, modal_width, modal_height);

        // 创建半透明背景遮罩（使用深灰色）
        let overlay_area = area;
        let overlay_block = Block::default()
            .style(Style::default().bg(Color::Rgb(20, 20, 20)));

        frame.render_widget(overlay_block, overlay_area);

        // 创建主题列表项
        let items: Vec<ListItem> = Theme::all()
            .iter()
            .map(|theme| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        theme.name,
                        Style::default()
                            .fg(theme.primary_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" - "),
                    Span::styled(
                        "●",
                        Style::default().fg(theme.accent_color),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.current_theme.primary_color))
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(
                        Style::default()
                            .fg(self.current_theme.primary_color)
                            .add_modifier(Modifier::BOLD)
                    )
                    .title(" 选择主题 (Esc 关闭, Enter 确认) ")
                    .style(Style::default().bg(self.current_theme.background_color))
            )
            .highlight_style(
                Style::default()
                    .bg(self.current_theme.secondary_color)
                    .add_modifier(Modifier::REVERSED)
            );

        frame.render_stateful_widget(list, modal_area, &mut self.theme_list_state);
    }

    /// Get input text, filtering out auto-wrap newlines
    /// Heuristic: next line starts with whitespace = user newline (Shift+Enter)
    ///           next line starts without whitespace = auto-wrap continuation
    pub fn get_input_text(&self) -> String {
        let lines = self.input_textarea.lines();
        if lines.len() <= 1 {
            return lines.first().map(|s| s.as_str()).unwrap_or("").to_string();
        }

        let mut result = Vec::new();
        let mut current_line = lines[0].clone();

        for i in 1..lines.len() {
            let line = &lines[i];
            let starts_with_space = line.starts_with(' ') || line.starts_with('\t');

            if starts_with_space {
                result.push(current_line);
                current_line = line.trim_start().to_string();
            } else {
                if !current_line.is_empty() && !current_line.ends_with(' ') {
                    current_line.push(' ');
                }
                current_line.push_str(line);
            }
        }

        result.push(current_line);

        if result.len() <= 1 {
            return result.first().map(|s| s.as_str()).unwrap_or("").to_string();
        }

        result.join("\n")
    }

    /// Clear input box
    pub fn clear_input(&mut self) {
        self.input_textarea = TextArea::default();
        let _ = self.input_textarea.set_block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.current_theme.border_color))
            .title("输入消息或命令 (Enter 发送, 输入 /help 查看命令)"));
        let _ = self.input_textarea.set_cursor_line_style(Style::default());
    }

    /// 解析并执行命令
    pub fn parse_and_execute_command(&mut self, input: &str) -> Option<KeyAction> {
        let trimmed = input.trim();

        // 检查是否是命令（以 / 开头）
        if !trimmed.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts.get(0).map(|s| s.to_lowercase()).unwrap_or_default();

        match command.as_str() {
            "/quit" => {
                log::info!("执行命令: /quit");
                Some(KeyAction::Quit)
            }
            "/cls" | "/clear" => {
                log::info!("执行命令: {}", command);
                Some(KeyAction::ClearChat)
            }
            "/help" => {
                log::info!("执行命令: /help");
                Some(KeyAction::ShowHelp)
            }
            "/themes" => {
                log::info!("执行命令: /themes");
                Some(KeyAction::ShowThemes)
            }
            "/dump-chats" => {
                log::info!("执行命令: /dump-chats");
                Some(KeyAction::DumpChats)
            }
            "/enable-audio-input" => {
                log::info!("执行命令: /enable-audio-input");
                Some(KeyAction::EnableAudioInput)
            }
            "/disable-audio-input" => {
                log::info!("执行命令: /disable-audio-input");
                Some(KeyAction::DisableAudioInput)
            }
            _ => {
                log::warn!("未知命令: {}", command);
                None
            }
        }
    }

    /// 解析帮助内容为 Line 列表
    fn parse_help_content() -> Vec<Line<'static>> {
        let help_text = "# Cortex TARS AI Program - 帮助信息

欢迎使用TARS演示程序，我是由Cortex Memory技术驱动的人工智能程序，作为你的第二大脑，我能够作为你的外脑与你的记忆深度链接。

## 可用命令

  - /quit          退出程序
  - /cls /clear    清空会话区域
  - /help          显示此帮助信息
  - /themes        切换主题
  - /dump-chats    复制会话区域的所有内容到剪贴板

## 快捷键

  - Enter          发送消息
  - Shift+Enter    换行
  - Ctrl+L         打开/关闭日志面板
  - Esc            关闭弹窗

---

Powered by Cortex Memory";

        // 使用 tui-markdown 渲染帮助文本
        let markdown_text = from_str(help_text);

        // 转换为 Line 列表
        markdown_text.lines.into_iter().map(|line| {
            Line::from(line.spans.iter().map(|s| {
                Span::raw(s.content.clone())
            }).collect::<Vec<Span>>())
        }).collect()
    }

    /// 导出所有会话内容到剪贴板
    pub fn dump_chats_to_clipboard(&self) -> Result<String, String> {
        let mut content = String::new();

        for message in &self.messages {
            let role = match message.role {
                crate::agent::MessageRole::User => "You",
                crate::agent::MessageRole::Assistant => "TARS AI",
                crate::agent::MessageRole::System => "System",
            };

            let time_str = message.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

            content.push_str(&format!("[{}] [{}]\n", role, time_str));
            content.push_str(&message.content);
            content.push_str("\n\n");
        }

        if content.is_empty() {
            return Err("没有会话内容可导出".to_string());
        }

        // 尝试复制到剪贴板
        match clipboard::ClipboardContext::new() {
            Ok(mut ctx) => {
                match ctx.set_contents(content.clone()) {
                    Ok(_) => {
                        log::info!("已导出 {} 个字符到剪贴板", content.len());
                        Ok(format!("已导出 {} 条消息到剪贴板", self.messages.len()))
                    }
                    Err(e) => {
                        log::error!("复制到剪贴板失败: {}", e);
                        Err(format!("复制到剪贴板失败: {}", e))
                    }
                }
            }
            Err(e) => {
                log::error!("无法访问剪贴板: {}", e);
                Err(format!("无法访问剪贴板: {}", e))
            }
        }
    }
}

impl Default for AppUi {
    fn default() -> Self {
        Self::new()
    }
}

impl AppUi {
    /// 渲染机器人管理弹窗
    fn render_bot_management_modal(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // 计算弹窗大小（居中显示）
        let modal_width = area.width.saturating_sub(20).min(80);
        let modal_height = area.height.saturating_sub(10).min(30);

        let x = (area.width - modal_width) / 2;
        let y = (area.height - modal_height) / 2;

        let modal_area = Rect::new(x, y, modal_width, modal_height);

        // 创建纯黑色背景遮罩，完全遮挡主界面
        frame.render_widget(
            Block::default()
                .style(Style::default().bg(Color::Black)),
            area
        );

        // 在弹窗区域绘制实心背景块，确保完全遮挡
        frame.render_widget(
            Paragraph::new("")
                .block(Block::default())
                .style(Style::default().bg(self.current_theme.background_color)),
            modal_area
        );

        match self.bot_management_state {
            BotManagementState::List => {
                self.render_bot_management_list(frame, modal_area);
            }
            BotManagementState::Creating => {
                self.render_bot_create_edit(frame, modal_area, true);
            }
            BotManagementState::Editing => {
                self.render_bot_create_edit(frame, modal_area, false);
            }
            BotManagementState::ConfirmDelete => {
                self.render_bot_confirm_delete(frame, modal_area);
            }
        }
    }

    /// 渲染机器人管理列表
    fn render_bot_management_list(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // 标题
        let title = Paragraph::new("机器人管理")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.current_theme.primary_color))
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(Style::default().fg(self.current_theme.primary_color).add_modifier(Modifier::BOLD))
                    .title(" Esc 关闭 ")
            )
            .alignment(Alignment::Center)
            .style(Style::default().bg(self.current_theme.background_color));

        frame.render_widget(title, chunks[0]);

        // 机器人列表
        let items: Vec<ListItem> = self
            .bot_list
            .iter()
            .map(|bot| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        bot.name.clone(),
                        Style::default()
                            .fg(self.current_theme.primary_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" - "),
                    Span::styled(
                        format!("{}...", &bot.system_prompt.chars().take(30).collect::<String>()),
                        Style::default().fg(Color::Gray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().bg(self.current_theme.background_color))
            .highlight_style(
                Style::default()
                    .bg(self.current_theme.secondary_color)
                    .add_modifier(Modifier::REVERSED),
            );

        frame.render_stateful_widget(list, chunks[1], &mut self.bot_management_list_state);

        // 帮助提示
        let help = Paragraph::new("↑/↓: 选择 | c: 创建 | e: 编辑 | d: 删除 | Esc: 关闭")
            .alignment(Alignment::Center)
            .style(Style::default().bg(self.current_theme.background_color));

        frame.render_widget(help, chunks[2]);
    }

    /// 渲染创建/编辑机器人界面
    fn render_bot_create_edit(&mut self, frame: &mut Frame, area: Rect, is_create: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        // 标题
        let title_text = if is_create { "创建机器人" } else { "编辑机器人" };
        let title = Paragraph::new(title_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.current_theme.primary_color))
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(Style::default().fg(self.current_theme.primary_color).add_modifier(Modifier::BOLD))
                    .title(" Esc: 取消 | Ctrl+S: 保存 ")
            )
            .alignment(Alignment::Center)
            .style(Style::default().bg(self.current_theme.background_color));

        frame.render_widget(title, chunks[0]);

        // 机器人名称输入 - 使用 Paragraph 渲染以支持多行显示
        let name_block = Block::default()
            .borders(Borders::ALL)
            .title("机器人名称")
            .border_style(if self.active_input_field == BotInputField::Name {
                Style::default().fg(self.current_theme.primary_color)
            } else {
                Style::default().fg(Color::Gray)
            });

        let name_text = self.bot_name_input.lines().join("\n");
        let name_para = Paragraph::new(name_text)
            .block(name_block)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(self.current_theme.background_color));
        frame.render_widget(name_para, chunks[1]);

        // 系统提示词输入 - 使用 Paragraph 渲染以支持多行显示
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .title("系统提示词")
            .border_style(if self.active_input_field == BotInputField::Prompt {
                Style::default().fg(self.current_theme.primary_color)
            } else {
                Style::default().fg(Color::Gray)
            });

        let prompt_text = self.bot_prompt_input.lines().join("\n");
        let prompt_para = Paragraph::new(prompt_text)
            .block(prompt_block)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(self.current_theme.background_color));
        frame.render_widget(prompt_para, chunks[2]);

        // 访问密码输入 - 使用 Paragraph 渲染以支持多行显示
        let password_block = Block::default()
            .borders(Borders::ALL)
            .title("访问密码")
            .border_style(if self.active_input_field == BotInputField::Password {
                Style::default().fg(self.current_theme.primary_color)
            } else {
                Style::default().fg(Color::Gray)
            });

        let password_text = self.bot_password_input.lines().join("\n");
        let password_para = Paragraph::new(password_text)
            .block(password_block)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(self.current_theme.background_color));
        frame.render_widget(password_para, chunks[3]);

        // 在活动输入框位置绘制光标
        self.render_cursor_in_active_input(frame, chunks.to_vec());

        // 帮助提示
        let help = Paragraph::new("Tab: 切换输入框 | Ctrl+S: 保存 | Esc: 取消")
            .alignment(Alignment::Center)
            .style(Style::default().bg(self.current_theme.background_color));

        frame.render_widget(help, chunks[4]);
    }

    /// 在活动输入框中绘制光标
    fn render_cursor_in_active_input(&mut self, frame: &mut Frame, chunks: Vec<Rect>) {
        let (input_area, input_lines) = match self.active_input_field {
            BotInputField::Name => (chunks[1], &self.bot_name_input),
            BotInputField::Prompt => (chunks[2], &self.bot_prompt_input),
            BotInputField::Password => (chunks[3], &self.bot_password_input),
        };

        // 获取光标位置
        let (cursor_row, cursor_col) = input_lines.cursor();

        // 计算光标在屏幕上的绝对位置
        let content_area = input_area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let lines = input_lines.lines();
        if cursor_row < lines.len() {
            let line = &lines[cursor_row];

            // 计算光标列位置（考虑 Unicode 字符宽度）
            let mut col_offset = 0usize;
            let chars: Vec<char> = line.chars().collect();
            for (i, c) in chars.iter().enumerate() {
                if i >= cursor_col {
                    break;
                }
                col_offset += unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
            }

            // 获取内容区域的宽度
            let content_width = content_area.width as usize;

            // 计算换行后的光标位置
            let display_row = cursor_row + (col_offset / content_width);
            let display_col = col_offset % content_width;

            // 确保光标在内容区域内
            if display_row < content_area.height as usize {
                // 计算光标在屏幕上的位置
                let cursor_x = content_area.x + display_col as u16;
                let cursor_y = content_area.y + display_row as u16;

                // 绘制光标（使用反色块）
                let cursor_area = Rect::new(cursor_x, cursor_y, 1, 1);
                let cursor_block = Block::default()
                    .style(Style::default()
                        .fg(self.current_theme.background_color)
                        .bg(self.current_theme.text_color));
                frame.render_widget(cursor_block, cursor_area);
            }
        }
    }

    /// 渲染确认删除界面
    fn render_bot_confirm_delete(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        // 标题
        let title = Paragraph::new("确认删除")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    .title(" Esc: 取消 ")
            )
            .alignment(Alignment::Center)
            .style(Style::default().bg(self.current_theme.background_color));

        frame.render_widget(title, chunks[0]);

        // 获取选中的机器人
        let bot_name = if let Some(index) = self.bot_management_list_state.selected() {
            self.bot_list.get(index).map(|b| b.name.clone()).unwrap_or_else(|| "未知".to_string())
        } else {
            "未知".to_string()
        };

        // 确认消息
        let confirm_msg = Paragraph::new(format!("确定要删除机器人 '{}' 吗？", bot_name))
            .alignment(Alignment::Center)
            .style(Style::default().fg(self.current_theme.text_color).bg(self.current_theme.background_color));

        frame.render_widget(confirm_msg, chunks[1]);

        // 帮助提示
        let help = Paragraph::new("y: 确认删除 | Esc: 取消")
            .alignment(Alignment::Center)
            .style(Style::default().bg(self.current_theme.background_color));

        frame.render_widget(help, chunks[2]);
    }

    /// 处理机器人管理弹窗的键盘事件
    pub fn handle_bot_management_key(&mut self, key: KeyEvent) -> KeyAction {
        match self.bot_management_state {
            BotManagementState::List => {
                self.handle_bot_management_list_key(key)
            }
            BotManagementState::Creating | BotManagementState::Editing => {
                self.handle_bot_create_edit_key(key)
            }
            BotManagementState::ConfirmDelete => {
                self.handle_bot_confirm_delete_key(key)
            }
        }
    }

    /// 处理机器人管理列表的键盘事件
    fn handle_bot_management_list_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.bot_management_modal_visible = false;
                KeyAction::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.bot_management_list_state.selected() {
                    if selected > 0 {
                        self.bot_management_list_state.select(Some(selected - 1));
                    }
                }
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.bot_management_list_state.selected() {
                    if selected < self.bot_list.len().saturating_sub(1) {
                        self.bot_management_list_state.select(Some(selected + 1));
                    }
                }
                KeyAction::Continue
            }
            KeyCode::Char('c') => {
                // 创建机器人
                self.bot_management_state = BotManagementState::Creating;
                self.clear_bot_inputs();
                KeyAction::CreateBot
            }
            KeyCode::Char('e') => {
                // 编辑机器人
                if let Some(index) = self.bot_management_list_state.selected() {
                    if let Some(bot) = self.bot_list.get(index) {
                        self.bot_management_state = BotManagementState::Editing;
                        self.bot_name_input = TextArea::from(vec![bot.name.clone()]);
                        self.bot_prompt_input = TextArea::from(vec![bot.system_prompt.clone()]);
                        self.bot_password_input = TextArea::from(vec![bot.access_password.clone()]);
                        return KeyAction::EditBot;
                    }
                }
                KeyAction::Continue
            }
            KeyCode::Char('d') => {
                // 删除机器人
                if !self.bot_list.is_empty() {
                    self.bot_management_state = BotManagementState::ConfirmDelete;
                }
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    /// 处理创建/编辑机器人的键盘事件
    fn handle_bot_create_edit_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                self.bot_management_state = BotManagementState::List;
                KeyAction::CancelBot
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // 保存机器人
                KeyAction::SaveBot
            }
            KeyCode::Tab => {
                // 切换输入框
                self.active_input_field = match self.active_input_field {
                    BotInputField::Name => BotInputField::Prompt,
                    BotInputField::Prompt => BotInputField::Password,
                    BotInputField::Password => BotInputField::Name,
                };
                KeyAction::Continue
            }
            _ => {
                // 所有其他按键都让当前活动的输入框处理（包括 Enter 键换行）
                match self.active_input_field {
                    BotInputField::Name => {
                        self.bot_name_input.input(key);
                    }
                    BotInputField::Prompt => {
                        self.bot_prompt_input.input(key);
                    }
                    BotInputField::Password => {
                        self.bot_password_input.input(key);
                    }
                }
                KeyAction::Continue
            }
        }
    }

    /// 处理确认删除的键盘事件
    fn handle_bot_confirm_delete_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.bot_management_state = BotManagementState::List;
                KeyAction::CancelBot
            }
            KeyCode::Char('y') => {
                // 确认删除
                KeyAction::DeleteBot
            }
            _ => KeyAction::Continue,
        }
    }

    /// 清空机器人输入框
    fn clear_bot_inputs(&mut self) {
        self.bot_name_input = TextArea::default();
        let _ = self.bot_name_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("机器人名称"));

        self.bot_prompt_input = TextArea::default();
        let _ = self.bot_prompt_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("系统提示词"));

        self.bot_password_input = TextArea::default();
        let _ = self.bot_password_input.set_block(Block::default()
            .borders(Borders::ALL)
            .title("访问密码"));

        // 重置活动输入框
        self.active_input_field = BotInputField::Name;
    }

    /// 获取机器人输入框的内容
    pub fn get_bot_input_data(&self) -> (String, String, String) {
        let name = self.bot_name_input.lines().join("\n");
        let prompt = self.bot_prompt_input.lines().join("\n");
        let password = self.bot_password_input.lines().join("\n");
        (name, prompt, password)
    }

    /// 获取当前选中的机器人索引（用于编辑和删除）
    pub fn get_selected_bot_index(&self) -> Option<usize> {
        self.bot_management_list_state.selected()
    }
}
