use anyhow::{Context, Result};
use log::{Level, LevelFilter, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Log manager
pub struct LogManager {
    #[allow(dead_code)]
    log_file: PathBuf,
    file: Arc<Mutex<File>>,
    lines: Arc<Mutex<Vec<String>>>,
}

impl LogManager {
    /// Create new log manager
    pub fn new(log_dir: &Path) -> Result<Self> {
        let log_file = log_dir.join("app.log");

        // Ensure log directory exists
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent).context("Failed to create log directory")?;
        }

        // Open or create log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open log file")?;

        Ok(Self {
            log_file,
            file: Arc::new(Mutex::new(file)),
            lines: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Write log entry
    pub fn write(&self, level: Level, message: &str) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{} {}] {}", timestamp, level, message);

        // Write to file
        let mut file = self
            .file
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire file lock: {}", e))?;
        writeln!(file, "{}", log_line).context("Failed to write log")?;
        file.flush().context("Failed to flush log")?;

        // Add to in-memory log lines
        let mut lines = self
            .lines
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire log lines lock: {}", e))?;
        lines.push(log_line.clone());

        // Limit in-memory log lines
        if lines.len() > 1000 {
            let excess = lines.len() - 1000;
            lines.drain(0..excess);
        }

        Ok(())
    }

    /// Read log content
    pub fn read_logs(&self, max_lines: usize) -> Result<Vec<String>> {
        let lines = self
            .lines
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire log lines lock: {}", e))?;

        // Return last max_lines
        if lines.len() > max_lines {
            Ok(lines[lines.len() - max_lines..].to_vec())
        } else {
            Ok(lines.clone())
        }
    }
}

/// Custom Logger
struct SimpleLogger {
    manager: Arc<LogManager>,
}

impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}", record.args());
            if let Err(e) = self.manager.write(record.level(), &message) {
                eprintln!("Failed to write log: {}", e);
            }
        }
    }

    fn flush(&self) {}
}

/// Initialize logging system
pub fn init_logger(log_dir: &Path) -> Result<Arc<LogManager>> {
    let manager = Arc::new(LogManager::new(log_dir)?);

    // Create custom logger
    let logger = SimpleLogger {
        manager: Arc::clone(&manager),
    };

    // Set global logger
    log::set_logger(Box::leak(Box::new(logger)))
        .map_err(|e| anyhow::anyhow!("Failed to set logger: {}", e))?;
    log::set_max_level(LevelFilter::Info);

    log::info!("Logging system initialized");
    log::info!("Log file path: {}", log_dir.display());

    Ok(manager)
}
