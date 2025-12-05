use cortex_mem_core::{Config, init_logging};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = Config::load("config.toml")?;
    
    // 初始化日志系统
    init_logging(&config.logging)?;
    
    // 记录启动信息
    tracing::debug!("Debug: Loading configuration completed");
    tracing::info!("Application starting...");
    tracing::info!("Logging configuration: enabled={}, directory={}, level={}", 
                   config.logging.enabled, 
                   config.logging.log_directory, 
                   config.logging.level);
    
    println!("Hello, world!");
    
    tracing::debug!("Debug: Application execution completed");
    tracing::info!("Application finished.");
    
    Ok(())
}
