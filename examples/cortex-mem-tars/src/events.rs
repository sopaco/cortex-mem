use crate::app::{App, FocusArea};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};

pub fn handle_key_event(event: Event, app: &mut App) -> Option<String> {
    // 处理鼠标事件
    if let Event::Mouse(mouse) = event {
        return handle_mouse_event(mouse, app);
    }

    // Some(input)表示需要处理的输入，None表示不需要处理
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Enter => {
                    if app.focus_area == FocusArea::Input && !app.current_input.trim().is_empty() {
                        let input = app.current_input.clone();
                        app.current_input.clear();
                        app.reset_cursor_to_end();
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
                        app.insert_char_at_cursor(c);
                    }
                    None
                }
                KeyCode::Backspace => {
                    if !app.is_processing
                        && !app.is_shutting_down
                        && app.focus_area == FocusArea::Input
                    {
                        app.delete_char_at_cursor();
                    }
                    None
                }
                KeyCode::Left => {
                    if !app.is_processing
                        && !app.is_shutting_down
                        && app.focus_area == FocusArea::Input
                    {
                        app.move_cursor_left();
                    }
                    None
                }
                KeyCode::Right => {
                    if !app.is_processing
                        && !app.is_shutting_down
                        && app.focus_area == FocusArea::Input
                    {
                        app.move_cursor_right();
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
                        FocusArea::Input => {
                            // 将光标移动到输入框开头
                            app.cursor_position = 0;
                        }
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
                        FocusArea::Input => {
                            // 将光标移动到输入框末尾
                            app.reset_cursor_to_end();
                        }
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

/// 处理鼠标事件
fn handle_mouse_event(mouse: MouseEvent, app: &mut App) -> Option<String> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // 左键点击时更新焦点区域
            // 这里可以根据鼠标位置判断点击了哪个区域
            // 简化处理：如果鼠标在左边区域，设置为输入或对话焦点；如果在右边区域，设置为日志焦点
            // 由于我们没有详细的坐标信息，这里只是简化处理
            None
        }
        MouseEventKind::ScrollUp => {
            // 鼠标向上滚动
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
        MouseEventKind::ScrollDown => {
            // 鼠标向下滚动
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
        MouseEventKind::Drag(MouseButton::Left) => {
            // 鼠标左键拖拽 - 这里我们不需要特别处理，终端默认支持文本选择
            None
        }
        _ => None,
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
