use std::collections::VecDeque;
use tokio::sync::mpsc;
use ratatui::widgets::ScrollbarState;

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
}

impl Default for App {
    fn default() -> Self {
        Self {
            conversations: VecDeque::with_capacity(100),
            current_input: String::new(),
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
        }
    }
}

impl App {
    pub fn new(message_sender: mpsc::UnboundedSender<AppMessage>) -> Self {
        Self {
            message_sender: Some(message_sender),
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
