use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub access_password: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl BotConfig {
    pub fn new(
        name: impl Into<String>,
        system_prompt: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            system_prompt: system_prompt.into(),
            access_password: password.into(),
            created_at: chrono::Utc::now(),
        }
    }
}

/// LLM Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub api_base_url: String,
    pub api_key: String,
    #[serde(alias = "model_efficient")]
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            api_base_url: std::env::var("LLM_API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
            api_key: std::env::var("LLM_API_KEY").unwrap_or_else(|_| "ollama".to_string()),
            model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string()),
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: LLMConfig,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub bots: HashMap<String, BotConfig>,
}

fn default_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "cortex-mem", "tars")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./.cortex"))
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = directories::ProjectDirs::from("com", "cortex-mem", "tars")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./.cortex"));

        Self {
            llm: LLMConfig::default(),
            data_dir,
            bots: HashMap::new(),
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    config: AppConfig,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        // Get current working directory
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        
        // System config directory
        let config_dir = directories::ProjectDirs::from("com", "cortex-mem", "tars")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./config"));

        fs::create_dir_all(&config_dir)?;

        // Check for config.toml: prioritize current directory
        let local_config_path = current_dir.join("config.toml");
        let system_config_path = config_dir.join("config.toml");
        
        let config_path = if local_config_path.exists() {
            println!("✓ Using config.toml from current directory: {:?}", local_config_path);
            local_config_path
        } else if system_config_path.exists() {
            println!("✓ Using config.toml from system directory: {:?}", system_config_path);
            system_config_path
        } else {
            println!("✓ No config.toml found, creating default in system directory: {:?}", system_config_path);
            system_config_path
        };

        // Check for bots.toml: prioritize current directory
        let local_bots_path = current_dir.join("bots.toml");
        let system_bots_path = config_dir.join("bots.toml");
        
        let bots_path = if local_bots_path.exists() {
            println!("✓ Using bots.toml from current directory: {:?}", local_bots_path);
            local_bots_path
        } else {
            println!("✓ Using bots.toml from system directory: {:?}", system_bots_path);
            system_bots_path
        };

        // Load or create main config
        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            match toml::from_str(&content) {
                Ok(c) => {
                    println!("✓ Successfully loaded config from: {:?}", config_path);
                    c
                }
                Err(e) => {
                    println!("⚠ Failed to parse config file: {}, using default", e);
                    AppConfig::default()
                }
            }
        } else {
            let default_config = AppConfig::default();
            // Save default config
            let content = toml::to_string_pretty(&default_config)?;
            fs::write(&config_path, content)?;
            println!("✓ Created default config at: {:?}", config_path);
            default_config
        };

        // Load bots configuration
        if bots_path.exists() {
            let content = fs::read_to_string(&bots_path)?;
            if let Ok(bots) = toml::from_str::<HashMap<String, BotConfig>>(&content) {
                let bots_count = bots.len();
                config.bots = bots;
                println!("✓ Loaded {} bots from: {:?}", bots_count, bots_path);
            }
        }

        Ok(Self {
            config,
            config_path,
        })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_dir(&self) -> PathBuf {
        self.config_path
            .parent()
            .unwrap_or(&self.config_path)
            .to_path_buf()
    }

    pub fn get_bots(&self) -> Result<Vec<BotConfig>> {
        Ok(self.config.bots.values().cloned().collect())
    }

    pub fn add_bot(&mut self, bot: BotConfig) -> Result<()> {
        self.config.bots.insert(bot.id.clone(), bot);
        self.save_bots(&self.config.bots.clone())
    }

    pub fn update_bot(&mut self, bot_id: &str, bot: BotConfig) -> Result<()> {
        self.config.bots.insert(bot_id.to_string(), bot);
        self.save_bots(&self.config.bots.clone())
    }

    pub fn remove_bot(&mut self, bot_id: &str) -> Result<bool> {
        let removed = self.config.bots.remove(bot_id).is_some();
        if removed {
            self.save_bots(&self.config.bots.clone())?;
        }
        Ok(removed)
    }

    fn save_bots(&self, bots: &HashMap<String, BotConfig>) -> Result<()> {
        // Try to save in current directory first, fallback to parent of config_path
        let current_dir = std::env::current_dir()?;
        let current_bots_path = current_dir.join("bots.toml");
        
        let bots_path = if current_bots_path.parent().is_some() {
            current_bots_path
        } else {
            self.config_path.parent().unwrap().join("bots.toml")
        };
        
        let content = toml::to_string_pretty(bots)?;
        fs::write(&bots_path, content)?;
        log::info!("Saved bots to: {:?}", bots_path);
        Ok(())
    }
}
