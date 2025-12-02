use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

use crate::app::{App, FocusArea};
use unicode_width::UnicodeWidthChar;

/// UI 绘制函数
pub fn draw_ui(f: &mut Frame, app: &mut App) {
    // 创建主布局
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(f.area());

    // 左列：对话区域和输入框
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(chunks[0]);

    // 对话历史 - 构建所有对话文本，包括正在流式生成的内容
    let display_conversations = app.get_display_conversations();
    let conversation_text = display_conversations
        .iter()
        .enumerate()
        .flat_map(|(index, (user, assistant))| {
            let is_streaming = app.current_streaming_response.is_some() && 
                               index == display_conversations.len() - 1;
            
            let assistant_style = if is_streaming {
                Style::default().fg(Color::Yellow) // 流式生成中用黄色
            } else {
                Style::default().fg(Color::Green)  // 完成的回复用绿色
            };
            
            let assistant_prefix = if is_streaming {
                "助手 (生成中): "
            } else {
                "助手: "
            };
            
            vec![
                Line::from(vec![
                    Span::styled("用户: ", Style::default().fg(Color::Cyan)),
                    Span::raw(user.clone()),
                ]),
                Line::from(vec![
                    Span::styled(assistant_prefix, assistant_style),
                    Span::styled(assistant.clone(), assistant_style),
                    if is_streaming {
                        Span::styled("▋", Style::default().fg(Color::Yellow)) // 光标效果
                    } else {
                        Span::raw("")
                    }
                ]),
                Line::from(""), // 空行分隔
            ]
        })
        .collect::<Vec<_>>();

    let total_conversations = display_conversations.len();

    // 构建对话区域标题，显示滚动状态和焦点状态
    let conversation_title = if app.focus_area == FocusArea::Conversation {
        if total_conversations > 0 {
            format!(
                "💬 对话历史 ({} 对, 偏移:{}) [Tab切换焦点 ↑向后 ↓向前 Home/End快速跳转]",
                total_conversations, app.conversation_scroll_offset
            )
        } else {
            format!("💬 对话历史 (0 对) [Tab切换焦点]")
        }
    } else {
        if total_conversations > 0 {
            format!(
                "对话历史 ({} 对, 偏移:{}) [Tab切换焦点]",
                total_conversations, app.conversation_scroll_offset
            )
        } else {
            format!("对话历史 (0 对) [Tab切换焦点]")
        }
    };

    let conversation_paragraph = Paragraph::new(conversation_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(conversation_title)
                .title_style(if app.focus_area == FocusArea::Conversation {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(ratatui::widgets::Wrap { trim: true })
        .scroll((app.conversation_scroll_offset as u16, 0));

    f.render_widget(Clear, left_chunks[0]);
    f.render_widget(conversation_paragraph, left_chunks[0]);

    // 渲染会话区滚动条
    if total_conversations > 0 {
        let total_lines = total_conversations * 3; // 每个对话3行
        let visible_height = left_chunks[0].height.saturating_sub(2) as usize; // 减去边框

        // 更新滚动条状态，使用实际的可见高度
        app.conversation_scrollbar_state = app
            .conversation_scrollbar_state
            .content_length(total_lines)
            .viewport_content_length(visible_height)
            .position(app.conversation_scroll_offset);

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            left_chunks[0],
            &mut app.conversation_scrollbar_state,
        );
    }

    // 输入区域 - 根据状态显示不同的内容
    if app.is_shutting_down {
        // 在shutting down时显示说明文案，不显示输入框
        let shutdown_text = Paragraph::new(Text::from(
            "正在执行记忆化存储，请稍候...\n\n系统将自动保存本次对话记录到记忆库中。",
        ))
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("正在退出程序... (记忆迭代中)")
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: true });

        f.render_widget(Clear, left_chunks[1]);
        f.render_widget(shutdown_text, left_chunks[1]);
        // 不设置光标，光标会自动隐藏
    } else {
        // 正常状态显示输入框
        let input_title = if app.focus_area == FocusArea::Input {
            "📝 输入消息 (Enter发送, Tab切换焦点, /quit退出)"
        } else {
            "输入消息 (Enter发送, Tab切换焦点, /quit退出)"
        };

        let input_paragraph = Paragraph::new(Text::from(app.current_input.as_str()))
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(input_title)
                    .title_style(if app.focus_area == FocusArea::Input {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    }),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, left_chunks[1]);
        f.render_widget(input_paragraph, left_chunks[1]);

        // 只有当焦点在输入框时才设置光标
        if app.focus_area == FocusArea::Input {
            // 计算输入框可用宽度（减去边框和边距）
            let available_width = left_chunks[1].width.saturating_sub(2) as usize;

            // 使用ratatui的wrap逻辑来计算光标位置
            // 我们需要模拟ratatui::widgets::Wrap的行为

            // 获取光标前的所有字符
            let chars_before_cursor: Vec<char> = app
                .current_input
                .chars()
                .take(app.cursor_position)
                .collect();

            // 模拟ratatui的换行逻辑
            let mut line_offset = 0;
            let mut current_line_width = 0;

            // 遍历光标前的所有字符，计算换行
            for ch in chars_before_cursor {
                let char_width = ch.width().unwrap_or(0);

                // 如果当前字符会超出行宽，则换行
                if current_line_width + char_width > available_width {
                    line_offset += 1;
                    current_line_width = 0;
                }

                current_line_width += char_width;
            }

            // 计算最终的光标位置
            let cursor_x = left_chunks[1].x + 1 + current_line_width as u16;
            let cursor_y = left_chunks[1].y + 1 + line_offset as u16;

            // 确保光标在输入框范围内
            if cursor_y < left_chunks[1].y + left_chunks[1].height {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // 右列：日志区域 - 构建所有日志文本，使用Paragraph的scroll功能
    let total_logs = app.logs.len();

    // 构建要显示的日志文本
    let log_text = app
        .logs
        .iter()
        .map(|log| {
            let style = if log.starts_with("[WARN]") {
                Style::default().fg(Color::Yellow)
            } else if log.starts_with("[ERROR]") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Gray)
            };

            Line::from(Span::styled(log.clone(), style))
        })
        .collect::<Vec<_>>();

    // 构建日志区域标题，显示滚动状态和焦点状态
    let log_title = if app.focus_area == FocusArea::Logs {
        if total_logs > 0 {
            format!(
                "🔍 系统日志 ({} 行, 偏移:{}) [Tab切换焦点 ↑向后 ↓向前 Home/End快速跳转]",
                total_logs, app.log_scroll_offset
            )
        } else {
            format!("🔍 系统日志 (0 行) [Tab切换焦点]")
        }
    } else {
        if total_logs > 0 {
            format!(
                "系统日志 ({} 行, 偏移:{}) [Tab切换焦点]",
                total_logs, app.log_scroll_offset
            )
        } else {
            format!("系统日志 (0 行) [Tab切换焦点]")
        }
    };

    let log_paragraph = Paragraph::new(log_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(log_title)
                .title_style(if app.focus_area == FocusArea::Logs {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(ratatui::widgets::Wrap { trim: true })
        .scroll((app.log_scroll_offset as u16, 0));

    f.render_widget(Clear, chunks[1]);
    f.render_widget(log_paragraph, chunks[1]);

    // 渲染日志区滚动条
    if total_logs > 0 {
        let visible_height = chunks[1].height.saturating_sub(2) as usize; // 减去边框

        // 更新滚动条状态，使用实际的可见高度
        app.log_scrollbar_state = app
            .log_scrollbar_state
            .content_length(total_logs)
            .viewport_content_length(visible_height)
            .position(app.log_scroll_offset);

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            chunks[1],
            &mut app.log_scrollbar_state,
        );
    }

    // 不再使用全屏覆盖层，保持所有UI区域可见
    // 这样用户可以在日志区域看到详细的quit执行过程
}
