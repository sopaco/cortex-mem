use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use cortex_mem_config::Config as CortexConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 机器人配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub access_password: String,
    pub created_at: DateTime<Utc>,
}

impl BotConfig {
    pub fn new(
        name: impl Into<String>,
        system_prompt: impl Into<String>,
        access_password: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            system_prompt: system_prompt.into(),
            access_password: access_password.into(),
            created_at: Utc::now(),
        }
    }
}

/// 配置管理器
pub struct ConfigManager {
    config_dir: PathBuf,
    bots_file: PathBuf,
    cortex_config: CortexConfig,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self> {
        // 使用系统应用数据目录（macOS: ~/Library/Application Support/com.cortex-mem.tars）
        let config_dir = directories::ProjectDirs::from("com", "cortex-mem", "tars")
            .context("无法获取项目目录")?
            .data_dir()  // 使用 data_dir 而不是 config_dir
            .to_path_buf();

        // 确保配置目录存在
        fs::create_dir_all(&config_dir).context("无法创建配置目录")?;
        log::info!("应用数据目录: {:?}", config_dir);

        // 所有配置文件都从系统目录读取
        let bots_file = config_dir.join("bots.json");
        let cortex_config_file = config_dir.join("config.toml");

        log::info!("机器人配置文件: {:?}", bots_file);
        log::info!("Cortex 配置文件: {:?}", cortex_config_file);

        // 加载或创建 cortex-mem 配置
        let cortex_config = if cortex_config_file.exists() {
            let config = CortexConfig::load(&cortex_config_file).context("无法加载 cortex-mem 配置")?;
            log::info!("已加载配置: embedding_dim={:?}", config.qdrant.embedding_dim);
            config
        } else {
            // 创建默认配置
            let default_config = CortexConfig {
                qdrant: cortex_mem_config::QdrantConfig {
                    url: "http://localhost:6334".to_string(),
                    collection_name: "cortex_mem".to_string(),
                    embedding_dim: Some(1536),
                    timeout_secs: 30,
                },
                embedding: cortex_mem_config::EmbeddingConfig::default(),
                llm: cortex_mem_config::LLMConfig {
                    api_base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "".to_string(),
                    model_efficient: "gpt-4o-mini".to_string(),
                    temperature: 0.7,
                    max_tokens: 2000,
                },
                server: cortex_mem_config::ServerConfig {
                    host: "localhost".to_string(),
                    port: 8080,
                    cors_origins: vec!["*".to_string()],
                },
                logging: cortex_mem_config::LoggingConfig::default(),
                cortex: cortex_mem_config::CortexConfig::default(),
            };
            let content = toml::to_string_pretty(&default_config).context("无法序列化默认配置")?;
            fs::write(&cortex_config_file, content).context("无法写入默认配置文件")?;
            log::info!("已创建默认 cortex-mem 配置文件: {:?}", cortex_config_file);
            default_config
        };

        Ok(Self {
            config_dir,
            bots_file,
            cortex_config,
        })
    }

    /// 获取所有机器人配置
    pub fn get_bots(&self) -> Result<Vec<BotConfig>> {
        if !self.bots_file.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&self.bots_file).context("无法读取配置文件")?;
        let bots: Vec<BotConfig> = serde_json::from_str(&content).context("无法解析配置文件")?;

        Ok(bots)
    }

    /// 保存所有机器人配置
    fn save_bots(&self, bots: &[BotConfig]) -> Result<()> {
        let content = serde_json::to_string_pretty(bots).context("无法序列化配置")?;
        fs::write(&self.bots_file, content).context("无法写入配置文件")?;
        Ok(())
    }

    /// 添加机器人
    pub fn add_bot(&self, bot: BotConfig) -> Result<()> {
        let bot_name = bot.name.clone();
        let bot_id = bot.id.clone();
        let mut bots = self.get_bots()?;
        bots.push(bot);
        self.save_bots(&bots)?;
        log::info!("添加机器人: {} (ID: {})", bot_name, bot_id);
        Ok(())
    }

    /// 删除机器人
    #[allow(dead_code)]
    pub fn remove_bot(&self, bot_id: &str) -> Result<bool> {
        let mut bots = self.get_bots()?;
        let original_len = bots.len();
        bots.retain(|bot| bot.id != bot_id);

        if bots.len() < original_len {
            self.save_bots(&bots)?;
            log::info!("删除机器人 ID: {}", bot_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 更新机器人
    #[allow(dead_code)]
    pub fn update_bot(&self, bot_id: &str, updated_bot: BotConfig) -> Result<bool> {
        let bot_name = updated_bot.name.clone();
        let mut bots = self.get_bots()?;
        if let Some(bot) = bots.iter_mut().find(|b| b.id == bot_id) {
            *bot = updated_bot;
            self.save_bots(&bots)?;
            log::info!("更新机器人: {} (ID: {})", bot_name, bot_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 根据 ID 获取机器人
    #[allow(dead_code)]
    pub fn get_bot(&self, bot_id: &str) -> Result<Option<BotConfig>> {
        let bots = self.get_bots()?;
        Ok(bots.into_iter().find(|bot| bot.id == bot_id))
    }

    /// 获取配置目录路径
    #[allow(dead_code)]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// 获取 cortex-mem 配置
    pub fn cortex_config(&self) -> &CortexConfig {
        &self.cortex_config
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("无法初始化配置管理器")
    }
}
