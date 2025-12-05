use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

/// 日志文件监听器
pub struct LogFileMonitor {
    log_file_path: Option<PathBuf>,
    last_position: u64,
}

impl LogFileMonitor {
    /// 创建新的日志文件监听器
    pub fn new() -> Self {
        Self {
            log_file_path: None,
            last_position: 0,
        }
    }

    /// 查找最新的日志文件
    pub async fn find_latest_log_file(&mut self, log_dir: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let log_path = Path::new(log_dir);
        
        if !log_path.exists() {
            return Err("日志目录不存在".into());
        }

        let mut latest_file = None;
        let mut latest_time = std::time::UNIX_EPOCH;

        if let Ok(entries) = std::fs::read_dir(log_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if modified > latest_time && entry.file_name().to_string_lossy().ends_with(".log") {
                            latest_time = modified;
                            latest_file = Some(entry.path());
                        }
                    }
                }
            }
        }

        if let Some(log_file) = latest_file {
            self.log_file_path = Some(log_file);
            // 设置初始位置为文件末尾，只读取新增内容
            if let Ok(file) = File::open(self.log_file_path.as_ref().unwrap()) {
                if let Ok(metadata) = file.metadata() {
                    self.last_position = metadata.len();
                }
            }
            Ok(())
        } else {
            Err("未找到日志文件".into())
        }
    }

    /// 读取新增的日志内容
    pub fn read_new_logs(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut new_logs = Vec::new();
        
        if let Some(ref log_file_path) = self.log_file_path {
            let mut file = File::open(log_file_path)?;
            
            // 检查文件大小
            let metadata = file.metadata()?;
            let current_size = metadata.len();
            
            // 如果文件没有新内容，直接返回
            if current_size <= self.last_position {
                return Ok(new_logs);
            }
            
            // 移动到上次读取的位置
            file.seek(SeekFrom::Start(self.last_position))?;
            
            // 读取新内容
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if !line.trim().is_empty() {
                        new_logs.push(line);
                    }
                }
            }
            
            // 更新位置
            self.last_position = current_size;
        }
        
        Ok(new_logs)
    }

    /// 启动日志监听，持续输出新日志到控制台
    pub async fn start_monitoring(&mut self, log_dir: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 查找最新日志文件
        self.find_latest_log_file(log_dir).await?;
        
        println!("🔍 开始监听日志文件: {:?}", self.log_file_path);
        
        loop {
            match self.read_new_logs() {
                Ok(new_logs) => {
                    for log_line in new_logs {
                        // 直接输出到控制台，保持原始格式
                        let formatted_log = self.format_log_for_console(&log_line);
                        println!("{}", formatted_log);
                    }
                }
                Err(e) => {
                    eprintln!("读取日志文件时出错: {}", e);
                    // 尝试重新查找日志文件（可能有新的日志文件生成）
                    if let Err(_find_err) = self.find_latest_log_file(log_dir).await {
                        eprintln!("重新查找日志文件失败");
                    }
                }
            }
            
            // 短暂休眠，避免过度占用CPU
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// 格式化日志内容用于控制台显示
    fn format_log_for_console(&self, log_line: &str) -> String {
        // 解析日志级别并添加颜色
        let colored_line = if log_line.contains(" ERROR ") {
            format!("\x1b[91m{}\x1b[0m", log_line) // 亮红色
        } else if log_line.contains(" WARN ") {
            format!("\x1b[93m{}\x1b[0m", log_line) // 亮黄色
        } else if log_line.contains(" INFO ") {
            format!("\x1b[36m{}\x1b[0m", log_line) // 亮青色
        } else if log_line.contains(" DEBUG ") {
            format!("\x1b[94m{}\x1b[0m", log_line) // 亮蓝色
        } else if log_line.contains(" TRACE ") {
            format!("\x1b[95m{}\x1b[0m", log_line) // 亮紫色
        } else {
            log_line.to_string() // 默认颜色
        };
        
        // 添加前缀标识这是来自日志文件的内容
        format!("📋 {}", colored_line)
    }
}

/// 启动日志监听任务（异步）
pub async fn start_log_monitoring_task(log_dir: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut monitor = LogFileMonitor::new();
    monitor.start_monitoring(&log_dir).await
}