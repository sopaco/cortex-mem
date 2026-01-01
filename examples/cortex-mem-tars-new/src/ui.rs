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
    Chat,
}

/// 聊天界面状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatState {
    Normal,
    LogPanel,
    Selection,
}

/// 应用 UI 状态
pub struct AppUi {
    pub state: AppState,
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
    pub cursor_position: (usize, usize),         // 当前光标位置 (line_index, char_index)
    // 消息显示区域位置
    pub messages_area: Option<Rect>,
}

/// 键盘事件处理结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Continue,    // 继续运行
    Quit,        // 退出程序
    SendMessage, // 发送消息
    ClearChat,   // 清空会话
    ShowHelp,    // 显示帮助
    DumpChats,   // 导出会话到剪贴板
}

impl AppUi {
    pub fn new() -> Self {
        let mut bot_list_state = ListState::default();
        bot_list_state.select(Some(0));

        let mut input_textarea = TextArea::default();
        let _ = input_textarea.set_block(Block::default()
            .borders(Borders::ALL)
            .title("输入消息或命令 (Enter 发送, 输入 /help 查看命令)"));
        let _ = input_textarea.set_cursor_line_style(Style::default());

        Self {
            state: AppState::BotSelection,
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

        match self.state {
            AppState::BotSelection => {
                if self.handle_bot_selection_key(key) {
                    KeyAction::Continue
                } else {
                    KeyAction::Quit
                }
            }
            AppState::Chat => self.handle_chat_key(key),
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
                    self.state = AppState::Chat;
                }
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

    /// 处理聊天界面的键盘事件
    fn handle_chat_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

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
                        log::debug!("Shift+Enter: 换行");
                        self.input_textarea.input(key);
                        KeyAction::Continue
                    }
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    log::debug!("Ctrl-C: 退出");
                    KeyAction::Quit
                }
                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    log::debug!("Ctrl-L: 切换日志面板");
                    self.log_panel_visible = !self.log_panel_visible;
                    KeyAction::Continue
                }
                KeyCode::Esc => {
                    log::debug!("关闭日志面板");
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

    /// 处理选择模式的键盘事件
    fn handle_selection_key(&mut self, key: KeyEvent) -> KeyAction {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                // 退出选择模式
                log::debug!("退出选择模式");
                self.chat_state = ChatState::Normal;
                self.selection_active = false;
                self.selection_start = None;
                self.selection_end = None;
                KeyAction::Continue
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // 向上移动光标
                if self.cursor_position.0 > 0 {
                    self.cursor_position.0 -= 1;
                    // 调整列位置到新行的长度范围内
                    let all_lines = self.get_all_rendered_lines();
                    if self.cursor_position.0 < all_lines.len() {
                        let line_len = all_lines[self.cursor_position.0].len();
                        self.cursor_position.1 = self.cursor_position.1.min(line_len);
                    }
                    self.selection_end = Some(self.cursor_position);
                }
                KeyAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // 向下移动光标
                let total_lines = self.calculate_total_lines();
                if self.cursor_position.0 < total_lines.saturating_sub(1) {
                    self.cursor_position.0 += 1;
                    // 调整列位置到新行的长度范围内
                    let all_lines = self.get_all_rendered_lines();
                    if self.cursor_position.0 < all_lines.len() {
                        let line_len = all_lines[self.cursor_position.0].len();
                        self.cursor_position.1 = self.cursor_position.1.min(line_len);
                    }
                    self.selection_end = Some(self.cursor_position);
                }
                KeyAction::Continue
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // 向左移动光标
                if self.cursor_position.1 > 0 {
                    self.cursor_position.1 -= 1;
                    self.selection_end = Some(self.cursor_position);
                }
                KeyAction::Continue
            }
            KeyCode::Right | KeyCode::Char('l') => {
                // 向右移动光标
                let all_lines = self.get_all_rendered_lines();
                if self.cursor_position.0 < all_lines.len() {
                    let line_len = all_lines[self.cursor_position.0].len();
                    if self.cursor_position.1 < line_len {
                        self.cursor_position.1 += 1;
                        self.selection_end = Some(self.cursor_position);
                    }
                }
                KeyAction::Continue
            }
            KeyCode::Char('y') => {
                // 复制选中的内容
                log::debug!("复制选中的内容");
                self.copy_selection();
                KeyAction::Continue
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+C 复制选中的内容
                log::debug!("Ctrl+C 复制选中的内容");
                self.copy_selection();
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
                crate::agent::MessageRole::System => "[System]",
                crate::agent::MessageRole::User => "[You]",
                crate::agent::MessageRole::Assistant => "[AI]",
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
        match self.state {
            AppState::BotSelection => self.render_bot_selection(frame),
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
        let help = Paragraph::new("↑/↓ 或 j/k: 选择 | Enter: 进入 | q 或 Ctrl-C: 退出")
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
        let title_line = Line::from(vec![
            Span::styled(
                "TARS AI Program",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    )
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    )
                    .title(" [ SYSTEM ACTIVE ] ")
            )
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(20, 30, 40))
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
        let title_line = Line::from(vec![
            Span::styled(
                "TARS AI Program",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    )
                    .border_type(ratatui::widgets::BorderType::Double)
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    )
                    .title(" [ SYSTEM ACTIVE ] ")
            )
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(20, 30, 40))
            );

        frame.render_widget(title, chunks[0]);

        // 消息显示区域
        let messages_area = chunks[1];
        self.messages_area = Some(messages_area);
        self.render_messages(frame, messages_area);

        // 日志面板
        self.render_log_panel(frame, chunks[2]);
    }

    /// 计算消息的总行数
    fn calculate_total_lines(&self) -> usize {
        let mut total = 0;
        for message in &self.messages {
            // 角色标签占 1 行
            total += 1;
            // 消息内容行数
            total += message.content.lines().count().max(1);
            // 空行分隔
            total += 1;
        }
        total
    }

    /// 滚动到底部
    pub fn scroll_to_bottom(&mut self, area_height: u16) {
        let total_lines = self.calculate_total_lines();
        let visible_lines = area_height.saturating_sub(2) as usize;
        let max_scroll = total_lines.saturating_sub(visible_lines);
        self.scroll_offset = max_scroll;
    }

    /// 渲染消息
    fn render_messages(&mut self, frame: &mut Frame, area: Rect) {
        // 使用 tui-markdown 渲染每个消息
        let content_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        // 收集所有消息的渲染行
        let mut all_lines: Vec<Line> = vec![];

        for message in &self.messages {
            let role_label = match message.role {
                crate::agent::MessageRole::System => "System",
                crate::agent::MessageRole::User => "You",
                crate::agent::MessageRole::Assistant => "TARS AI",
            };

            let role_color = match message.role {
                crate::agent::MessageRole::System => Color::Yellow,
                crate::agent::MessageRole::User => Color::Green,
                crate::agent::MessageRole::Assistant => Color::Cyan,
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

            // 渲染 Markdown 内容
            let markdown_text = from_str(&message.content);
            // 将 tui_markdown 的 Text 转换为 ratatui 的 Text
            for line in markdown_text.lines {
                all_lines.push(Line::from(line.spans.iter().map(|s| {
                    Span::raw(s.content.clone())
                }).collect::<Vec<Span>>()));
            }

            // 添加空行分隔
            all_lines.push(Line::from(""));
        }

        // 计算滚动
        let total_lines = all_lines.len();
        let visible_lines = area.height.saturating_sub(2) as usize; // 减去边框
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

        // 应用选择高亮
        let display_lines: Vec<Line> = if self.selection_active {
            self.apply_selection_highlight(all_lines, self.scroll_offset, visible_lines)
        } else {
            all_lines
                .into_iter()
                .skip(self.scroll_offset)
                .take(visible_lines)
                .collect()
        };

        // 渲染边框
        let title = "聊天记录 (鼠标拖拽选择, Esc 清除选择)";
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title);
        frame.render_widget(block, area);

        // 渲染消息内容（在边框内部）
        let paragraph = Paragraph::new(display_lines)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, content_area);

        // 渲染滚动条
        if total_lines > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(self.scroll_offset);

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
            log::debug!("选择区域和可见区域没有重叠，直接返回");
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
                let visible_idx = original_idx - scroll_offset;
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

                            log::debug!("单行高亮: original_idx={}, before_len={}, selected_len={}, after_len={}",
                                       original_idx, before.len(), selected.len(), after.len());

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

        let mut scrollbar_state = ScrollbarState::new(self.log_lines.len())
            .position(self.log_scroll_offset);

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
            "/dump-chats" => {
                log::info!("执行命令: /dump-chats");
                Some(KeyAction::DumpChats)
            }
            _ => {
                log::warn!("未知命令: {}", command);
                None
            }
        }
    }

    /// 获取帮助信息
    pub fn get_help_message() -> String {
        "# TARS AI Program - 帮助信息\n\n## 版本\n\nv0.1.0\n\n## 可用命令\n\n| 命令 | 说明 |\n|------|------|\n| `/quit` | 退出程序 |\n| `/cls` 或 `/clear` | 清空会话区域 |\n| `/help` | 显示此帮助信息 |\n| `/dump-chats` | 复制会话区域的所有内容到剪贴板 |\n\n## 快捷键\n\n- **Enter**: 发送消息\n- **Shift+Enter**: 换行\n- **Ctrl+L**: 打开/关闭日志面板\n- **Esc**: 关闭日志面板\n- **q**: 退出程序\n\n## 使用说明\n\n在输入框中输入命令并按 Enter 即可执行。\n\n---\n\n*Powered by TARS AI*".to_string()
    }

    /// 导出所有会话内容到剪贴板
    pub fn dump_chats_to_clipboard(&self) -> Result<String, String> {
        let mut content = String::new();

        for message in &self.messages {
            let role = match message.role {
                crate::agent::MessageRole::System => "System",
                crate::agent::MessageRole::User => "You",
                crate::agent::MessageRole::Assistant => "TARS AI",
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
