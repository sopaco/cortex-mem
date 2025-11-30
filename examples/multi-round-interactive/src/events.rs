use crate::app::{redirect_log_to_ui, App, FocusArea};
use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::agent::store_conversations_batch;
use memo_rig::memory::manager::MemoryManager;
use memo_rig::types::Message;
use std::sync::Arc;

/// 处理退出逻辑（包含记忆化流程）
/// 返回 true 表示记忆化完成，需要发送 MemoryIterationCompleted 消息
pub async fn handle_quit(
    conversations: Vec<(String, String)>,
    memory_manager: Arc<MemoryManager>,
    user_id: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // 发送日志并立即处理显示
    redirect_log_to_ui("SHUTDOWN", "🚀 用户选择退出，开始记忆化流程...");

    // 收集所有非quit消息
    let mut all_messages = Vec::new();
    let mut valid_conversations = 0;
    
    for (user_msg, assistant_msg) in &conversations {
        let user_msg_trimmed = user_msg.trim().to_lowercase();
        if user_msg_trimmed == "quit"
            || user_msg_trimmed == "exit"
            || user_msg_trimmed == "/quit"
            || user_msg_trimmed == "/exit"
        {
            continue;
        }

        valid_conversations += 1;
        all_messages.extend(vec![
            Message {
                role: "user".to_string(),
                content: user_msg.clone(),
                name: None,
            },
            Message {
                role: "assistant".to_string(),
                content: assistant_msg.clone(),
                name: None,
            },
        ]);
    }

    // 发送分析日志并立即处理显示
    redirect_log_to_ui(
        "SHUTDOWN",
        &format!("📊 找到 {} 条有效对话记录，开始处理...", valid_conversations),
    );

    if all_messages.is_empty() {
        redirect_log_to_ui("SHUTDOWN", "⚠️ 没有需要存储的内容");
        redirect_log_to_ui("SHUTDOWN", "✅ 记忆化流程完成（无需处理）");
        redirect_log_to_ui("SHUTDOWN", "🎉 退出流程完成！");
        return Ok(true);
    }

    // 发送开始批量处理日志并立即处理显示
    redirect_log_to_ui("SHUTDOWN", &format!("🚀 开始存储 {} 条消息到记忆系统...", all_messages.len()));

    // 添加短暂延迟让用户看到日志
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 执行批量记忆化
    let result = store_conversations_batch(memory_manager.clone(), &all_messages, user_id).await;

    match result {
        Ok(_) => {
            redirect_log_to_ui("SHUTDOWN", "✨ 记忆化完成！");
            redirect_log_to_ui("SHUTDOWN", "✅ 所有对话已成功存储到记忆系统");
            redirect_log_to_ui("SHUTDOWN", "🎉 退出流程完成！");
        }
        Err(e) => {
            let error_msg = format!("❌ 记忆存储失败: {}", e);
            redirect_log_to_ui("ERROR", &error_msg);
            redirect_log_to_ui("SHUTDOWN", "❌ 记忆化操作失败，但仍会退出");
            // 即使失败也返回true，因为用户要退出
        }
    }

    // 添加短暂延迟让用户看到最后的日志
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // 返回 true，告诉调用者记忆化已完成
    Ok(true)
}

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
