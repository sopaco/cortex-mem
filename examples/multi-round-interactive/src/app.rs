use ratatui::widgets::ScrollbarState;
use std::collections::VecDeque;
use tokio::sync::mpsc;

// 全局消息发送器，用于日志重定向
use once_cell::sync::OnceCell;
use std::sync::Mutex;

static LOG_SENDER: OnceCell<Mutex<Option<mpsc::UnboundedSender<AppMessage>>>> = OnceCell::new();

// 设置全局日志发送器 (crate可见性)
pub(crate) fn set_global_log_sender(sender: mpsc::UnboundedSender<AppMessage>) {
    LOG_SENDER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .replace(sender);
}

// 获取全局日志发送器 (crate可见性)
pub(crate) fn get_global_log_sender() -> Option<mpsc::UnboundedSender<AppMessage>> {
    LOG_SENDER
        .get()
        .and_then(|mutex| mutex.lock().unwrap().clone())
}

// 简单的日志重定向函数
pub fn redirect_log_to_ui(level: &str, message: &str) {
    if let Some(sender) = get_global_log_sender() {
        let full_message = format!("[{}] {}", level, message);
        let _ = sender.send(AppMessage::Log(full_message));
    }
}

#[derive(Debug)]
pub enum AppMessage {
    Log(String),
    Conversation {
        user: String,
        assistant: String,
    },
    StreamingChunk {
        user: String,
        chunk: String,
    },
    StreamingComplete {
        user: String,
        full_response: String,
    },
    #[allow(dead_code)]
    MemoryIterationCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusArea {
    Input,        // 输入框
    Conversation, // 对话区域
    Logs,         // 日志区域
}

/// 应用状态
pub struct App {
    // 对话历史
    pub conversations: VecDeque<(String, String)>,
    // 当前输入
    pub current_input: String,
    // 光标位置（以字符为单位）
    pub cursor_position: usize,
    // 日志信息
    pub logs: VecDeque<String>,
    // Agent 是否正在处理
    pub is_processing: bool,
    // 用户信息
    pub user_info: Option<String>,
    // 是否需要退出
    pub should_quit: bool,
    // 是否在shut down过程中
    pub is_shutting_down: bool,
    // 记忆迭代是否完成
    pub memory_iteration_completed: bool,
    // 消息发送器
    pub message_sender: Option<mpsc::UnboundedSender<AppMessage>>,
    // 日志滚动偏移
    pub log_scroll_offset: usize,
    // 对话滚动偏移
    pub conversation_scroll_offset: usize,
    // 当前焦点区域
    pub focus_area: FocusArea,
    // 用户是否手动滚动过日志（用于决定是否自动滚动到底部）
    pub user_scrolled_logs: bool,
    // 用户是否手动滚动过对话（用于决定是否自动滚动到底部）
    pub user_scrolled_conversations: bool,
    // 滚动条状态
    pub conversation_scrollbar_state: ScrollbarState,
    pub log_scrollbar_state: ScrollbarState,
    // 当前正在流式生成的回复
    pub current_streaming_response: Option<(String, String)>, // (user_input, partial_response)
}

impl Default for App {
    fn default() -> Self {
        Self {
            conversations: VecDeque::with_capacity(100),
            current_input: String::new(),
            cursor_position: 0,
            logs: VecDeque::with_capacity(50),
            is_processing: false,
            user_info: None,
            should_quit: false,
            is_shutting_down: false,
            memory_iteration_completed: false,
            message_sender: None,
            log_scroll_offset: 0,
            conversation_scroll_offset: 0,
            focus_area: FocusArea::Input,
            user_scrolled_logs: false,
            user_scrolled_conversations: false,
            conversation_scrollbar_state: ScrollbarState::default(),
            log_scrollbar_state: ScrollbarState::default(),
            current_streaming_response: None,
        }
    }
}

impl App {
    pub fn new(message_sender: mpsc::UnboundedSender<AppMessage>) -> Self {
        Self {
            message_sender: Some(message_sender),
            current_streaming_response: None,
            ..Default::default()
        }
    }

    pub fn add_log(&mut self, log: String) {
        self.logs.push_back(log);
        if self.logs.len() > 50 {
            self.logs.pop_front();
        }

        // 如果用户没有手动滚动过，自动滚动到最新日志
        if !self.user_scrolled_logs {
            self.scroll_logs_to_bottom();
        }
    }

    pub fn add_conversation(&mut self, user: String, assistant: String) {
        self.conversations.push_back((user, assistant));
        if self.conversations.len() > 100 {
            self.conversations.pop_front();
        }

        // 如果用户没有手动滚动过，自动滚动到最新对话
        if !self.user_scrolled_conversations {
            self.scroll_conversations_to_bottom();
        }
    }

    /// 开始流式回复
    pub fn start_streaming_response(&mut self, user_input: String) {
        self.current_streaming_response = Some((user_input, String::new()));
        self.is_processing = true;
    }

    /// 添加流式内容块
    pub fn add_streaming_chunk(&mut self, chunk: String) {
        if let Some((_, ref mut response)) = self.current_streaming_response {
            response.push_str(&chunk);
            
            // 如果用户没有手动滚动过，自动滚动到最新对话
            if !self.user_scrolled_conversations {
                self.scroll_conversations_to_bottom();
            }
        }
    }

    /// 完成流式回复
    pub fn complete_streaming_response(&mut self) {
        if let Some((user_input, full_response)) = self.current_streaming_response.take() {
            self.add_conversation(user_input, full_response);
        }
        self.is_processing = false;
    }

    /// 获取当前显示的对话（包括正在流式生成的）
    pub fn get_display_conversations(&self) -> Vec<(String, String)> {
        let mut conversations: Vec<(String, String)> = self.conversations.iter().cloned().collect();
        
        // 如果有正在流式生成的回复，添加到显示列表
        if let Some((ref user_input, ref partial_response)) = self.current_streaming_response {
            conversations.push((user_input.clone(), partial_response.clone()));
        }
        
        conversations
    }

    /// 在光标位置插入字符
    pub fn insert_char_at_cursor(&mut self, c: char) {
        // 将光标位置转换为字节索引
        let byte_pos = self
            .current_input
            .chars()
            .take(self.cursor_position)
            .map(|ch| ch.len_utf8())
            .sum();

        self.current_input.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    /// 在光标位置删除字符（退格键）
    pub fn delete_char_at_cursor(&mut self) {
        if self.cursor_position > 0 {
            // 将光标位置转换为字节索引
            let chars: Vec<char> = self.current_input.chars().collect();
            if self.cursor_position <= chars.len() {
                // 找到要删除字符的字节范围
                let byte_start: usize = chars
                    .iter()
                    .take(self.cursor_position - 1)
                    .map(|ch| ch.len_utf8())
                    .sum();

                let byte_end: usize = chars
                    .iter()
                    .take(self.cursor_position)
                    .map(|ch| ch.len_utf8())
                    .sum();

                // 安全地删除字符
                self.current_input.drain(byte_start..byte_end);
                self.cursor_position -= 1;
            }
        }
    }

    /// 将光标向左移动一个字符
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// 将光标向右移动一个字符
    pub fn move_cursor_right(&mut self) {
        let input_len = self.current_input.chars().count();
        if self.cursor_position < input_len {
            self.cursor_position += 1;
        }
    }

    /// 重置光标位置到末尾
    pub fn reset_cursor_to_end(&mut self) {
        self.cursor_position = self.current_input.chars().count();
    }

    /// 滚动到日志底部（最新日志）
    pub fn scroll_logs_to_bottom(&mut self) {
        self.log_scroll_offset = 0;
    }

    /// 滚动到对话底部（最新对话）
    pub fn scroll_conversations_to_bottom(&mut self) {
        self.conversation_scroll_offset = 0;
    }

    /// 向前滚动日志（查看更早日志）
    pub fn scroll_logs_forward(&mut self) {
        if self.logs.is_empty() {
            return;
        }

        let page_size = 10; // 每次翻页的行数

        // 简单增加偏移量，让UI层处理边界
        self.log_scroll_offset += page_size;
        self.user_scrolled_logs = true;
    }

    /// 向后滚动日志（查看更新日志）
    pub fn scroll_logs_backward(&mut self) {
        if self.logs.is_empty() {
            return;
        }

        let page_size = 10; // 每次翻页的行数

        // 向后翻页（减少偏移量，查看更新的日志）
        if self.log_scroll_offset >= page_size {
            self.log_scroll_offset -= page_size;
        } else {
            self.log_scroll_offset = 0;
            self.user_scrolled_logs = false;
        }
    }

    /// 向前滚动对话（查看更早内容）
    pub fn scroll_conversations_forward(&mut self) {
        if self.conversations.is_empty() {
            return;
        }

        let page_size = 5; // 每次翻页的行数

        // 简单增加偏移量，让UI层处理边界
        self.conversation_scroll_offset += page_size;
        self.user_scrolled_conversations = true;
    }

    /// 向后滚动对话（查看更新内容）
    pub fn scroll_conversations_backward(&mut self) {
        if self.conversations.is_empty() {
            return;
        }

        let page_size = 5; // 每次翻页的行数

        // 向后翻页（减少偏移量，查看更新的内容）
        if self.conversation_scroll_offset >= page_size {
            self.conversation_scroll_offset -= page_size;
        } else {
            self.conversation_scroll_offset = 0;
            self.user_scrolled_conversations = false;
        }
    }

    /// 切换焦点到下一个区域
    pub fn next_focus(&mut self) {
        self.focus_area = match self.focus_area {
            FocusArea::Input => {
                if self.is_shutting_down {
                    // 在退出过程中，跳过输入框，直接到对话区域
                    FocusArea::Conversation
                } else {
                    FocusArea::Conversation
                }
            }
            FocusArea::Conversation => {
                if self.is_shutting_down {
                    // 在退出过程中，从对话区域切换到日志区域
                    FocusArea::Logs
                } else {
                    FocusArea::Logs
                }
            }
            FocusArea::Logs => {
                if self.is_shutting_down {
                    // 在退出过程中，从日志区域切换回对话区域
                    FocusArea::Conversation
                } else {
                    FocusArea::Input
                }
            }
        };
    }

    pub fn log_info(&self, message: &str) {
        if let Some(sender) = &self.message_sender {
            let _ = sender.send(AppMessage::Log(format!("[INFO] {}", message)));
        }
    }
}
