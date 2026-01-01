use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
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
    pub fn new(name: impl Into<String>, system_prompt: impl Into<String>, access_password: impl Into<String>) -> Self {
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
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("com", "cortex", "mem-tars")
            .context("无法获取项目目录")?
            .config_dir()
            .to_path_buf();

        fs::create_dir_all(&config_dir).context("无法创建配置目录")?;

        let bots_file = config_dir.join("bots.json");

        Ok(Self {
            config_dir,
            bots_file,
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
    pub fn get_bot(&self, bot_id: &str) -> Result<Option<BotConfig>> {
        let bots = self.get_bots()?;
        Ok(bots.into_iter().find(|bot| bot.id == bot_id))
    }

    /// 获取配置目录路径
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("无法初始化配置管理器")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_config_creation() {
        let bot = BotConfig::new("TestBot", "You are a helpful assistant", "password123");
        assert_eq!(bot.name, "TestBot");
        assert_eq!(bot.system_prompt, "You are a helpful assistant");
        assert_eq!(bot.access_password, "password123");
        assert!(!bot.id.is_empty());
    }
}