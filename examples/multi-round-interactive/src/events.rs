use crate::app::{App, FocusArea};
use crossterm::event::{Event, KeyCode, KeyEventKind};



pub fn handle_key_event(event: Event, app: &mut App) -> Option<String> {
    // Some(input)表示需要处理的输入，None表示不需要处理
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Enter => {
                    if app.focus_area == FocusArea::Input && !app.current_input.trim().is_empty() {
                        let input = app.current_input.clone();
                        app.current_input.clear();
                        app.is_processing = true;
                        Some(input) // 返回输入内容给上层处理
                    } else {
                        None
                    }
                }
                KeyCode::Char(c) => {
                    if !app.is_processing
                        && !app.is_shutting_down
                        && app.focus_area == FocusArea::Input
                    {
                        app.current_input.push(c);
                    }
                    None
                }
                KeyCode::Backspace => {
                    if !app.is_processing
                        && !app.is_shutting_down
                        && app.focus_area == FocusArea::Input
                    {
                        app.current_input.pop();
                    }
                    None
                }
                KeyCode::Up => {
                    // 上键：向后滚动（查看更新内容）
                    match app.focus_area {
                        FocusArea::Logs => {
                            app.scroll_logs_backward();
                        }
                        FocusArea::Conversation => {
                            app.scroll_conversations_backward();
                        }
                        FocusArea::Input => {}
                    }
                    None
                }
                KeyCode::Down => {
                    // 下键：向前滚动（查看更早内容）
                    match app.focus_area {
                        FocusArea::Logs => {
                            app.scroll_logs_forward();
                        }
                        FocusArea::Conversation => {
                            app.scroll_conversations_forward();
                        }
                        FocusArea::Input => {}
                    }
                    None
                }
                KeyCode::Tab => {
                    // 切换焦点
                    let _old_focus = app.focus_area;
                    app.next_focus();
                    None
                }
                KeyCode::Home => {
                    match app.focus_area {
                        FocusArea::Logs => {
                            // 滚动到最旧的日志（设置一个较大的偏移量）
                            app.log_scroll_offset = app.logs.len().saturating_sub(1);
                            app.user_scrolled_logs = true;
                        }
                        FocusArea::Conversation => {
                            // 滚动到最旧的对话（设置一个较大的偏移量）
                            let total_lines = app.conversations.len() * 3;
                            app.conversation_scroll_offset = total_lines.saturating_sub(1);
                            app.user_scrolled_conversations = true;
                        }
                        FocusArea::Input => {} // 输入框不支持滚动
                    }
                    None
                }
                KeyCode::End => {
                    match app.focus_area {
                        FocusArea::Logs => {
                            // 滚动到最新的日志
                            app.scroll_logs_to_bottom();
                        }
                        FocusArea::Conversation => {
                            // 滚动到最新的对话
                            app.scroll_conversations_to_bottom();
                        }
                        FocusArea::Input => {} // 输入框不支持滚动
                    }
                    None
                }
                KeyCode::Esc => {
                    app.should_quit = true;
                    app.is_shutting_down = true;
                    Some("/quit".to_string()) // 模拟quit命令
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn process_user_input(input: String, app: &mut App) -> bool {
    // true表示是quit命令，false表示普通输入
    // 检查是否为退出命令
    let is_quit = input.trim() == "/quit";
    if is_quit {
        app.should_quit = true;
    }
    is_quit
}
