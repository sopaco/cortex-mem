// use crossterm::execute;
// use std::io::Write;

/// 终极终端清理函数
pub fn cleanup_terminal_final(_terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) {
    // 直接使用标准输出流进行最彻底的清理
    // let mut stdout = std::io::stdout();
    
    // // 执行必要的重置命令，但不清除屏幕内容
    // let _ = execute!(&mut stdout, crossterm::style::ResetColor);
    // let _ = execute!(&mut stdout, crossterm::cursor::Show);
    // let _ = execute!(&mut stdout, crossterm::terminal::LeaveAlternateScreen);
    // let _ = execute!(&mut stdout, crossterm::event::DisableMouseCapture);
    // let _ = execute!(&mut stdout, crossterm::style::SetAttribute(crossterm::style::Attribute::Reset));
    // let _ = execute!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::Reset));
    // let _ = execute!(&mut stdout, crossterm::style::SetBackgroundColor(crossterm::style::Color::Reset));
    
    // // 禁用原始模式
    // let _ = crossterm::terminal::disable_raw_mode();
    
    // // 立即刷新输出
    // let _ = stdout.flush();
    
    // // 只重置样式，不清除屏幕内容
    // let style_reset = "\x1b[0m\x1b[?25h";
    // print!("{}", style_reset);
    // let _ = stdout.flush();
}