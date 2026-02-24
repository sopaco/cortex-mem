use anyhow::{Context, Result};
use log::{Level, LevelFilter, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// 日志管理器
pub struct LogManager {
    #[allow(dead_code)]
    log_file: PathBuf,
    file: Arc<Mutex<File>>,
    lines: Arc<Mutex<Vec<String>>>,
}

impl LogManager {
    /// 创建新的日志管理器
    pub fn new(log_dir: &Path) -> Result<Self> {
        let log_file = log_dir.join("app.log");

        // 确保日志目录存在
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent).context("无法创建日志目录")?;
        }

        // 打开或创建日志文件
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("无法打开日志文件")?;

        Ok(Self {
            log_file,
            file: Arc::new(Mutex::new(file)),
            lines: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// 写入日志
    pub fn write(&self, level: Level, message: &str) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{} {}] {}", timestamp, level, message);

        // 写入文件
        let mut file = self.file.lock().map_err(|e| anyhow::anyhow!("无法获取文件锁: {}", e))?;
        writeln!(file, "{}", log_line)
            .context("无法写入日志")?;
        file.flush().context("无法刷新日志")?;

        // 添加到内存中的日志行
        let mut lines = self.lines.lock().map_err(|e| anyhow::anyhow!("无法获取日志行锁: {}", e))?;
        lines.push(log_line.clone());

        // 限制内存中的日志行数
        if lines.len() > 1000 {
            let excess = lines.len() - 1000;
            lines.drain(0..excess);
        }

        Ok(())
    }

    /// 读取日志内容
    pub fn read_logs(&self, max_lines: usize) -> Result<Vec<String>> {
        let lines = self.lines.lock().map_err(|e| anyhow::anyhow!("无法获取日志行锁: {}", e))?;

        // 返回最后 max_lines 行
        if lines.len() > max_lines {
            Ok(lines[lines.len() - max_lines..].to_vec())
        } else {
            Ok(lines.clone())
        }
    }
}

/// 自定义 Logger
struct SimpleLogger {
    manager: Arc<LogManager>,
}

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // 🔧 过滤TRACE和DEBUG日志，只保留INFO及以上级别
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}", record.args());
            // 🔇 静默处理日志写入失败，避免干扰TUI
            let _ = self.manager.write(record.level(), &message);
        }
    }

    fn flush(&self) {}
}

/// 初始化日志系统
pub fn init_logger(log_dir: &Path) -> Result<Arc<LogManager>> {
    let manager = Arc::new(LogManager::new(log_dir)?);

    // 创建自定义 logger
    let logger = SimpleLogger {
        manager: Arc::clone(&manager),
    };

    // 设置全局 logger
    log::set_logger(Box::leak(Box::new(logger)))
        .map_err(|e| anyhow::anyhow!("无法设置 logger: {}", e))?;
    log::set_max_level(LevelFilter::Info);

    log::info!("日志系统初始化完成");
    log::info!("日志文件路径: {}", log_dir.display());

    Ok(manager)
}
