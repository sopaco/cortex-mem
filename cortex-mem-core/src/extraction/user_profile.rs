use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 用户信息分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserInfoCategory {
    /// 个人基本信息：姓名、年龄、性格特质、价值观等
    PersonalInfo,
    /// 工作履历：职位、公司、职责、成就等
    WorkHistory,
    /// 偏好习惯：兴趣、爱好、工作方式、沟通风格等
    Preferences,
    /// 人际关系：重要的人、组织、社交圈等
    Relationships,
    /// 目标规划：职业目标、个人发展目标等
    Goals,
}

/// 用户信息条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoItem {
    /// 类别
    pub category: UserInfoCategory,
    /// 内容
    pub content: String,
    /// 置信度（0.0-1.0）
    pub confidence: f32,
    /// 重要性（0-10）
    pub importance: u8,
    /// 来源 session ID
    pub source_session: String,
    /// 提取时间
    pub extracted_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 版本号（用于跟踪更新）
    pub version: u32,
}

impl UserInfoItem {
    pub fn new(
        category: UserInfoCategory,
        content: impl Into<String>,
        confidence: f32,
        importance: u8,
        source_session: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            category,
            content: content.into(),
            confidence: confidence.clamp(0.0, 1.0),
            importance: importance.min(10),
            source_session: source_session.into(),
            extracted_at: now,
            updated_at: now,
            version: 1,
        }
    }
    
    /// 更新内容
    pub fn update(&mut self, new_content: impl Into<String>, confidence: f32) {
        self.content = new_content.into();
        self.confidence = confidence.clamp(0.0, 1.0);
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

/// 用户信息档案（结构化）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    /// 个人基本信息
    pub personal_info: Vec<UserInfoItem>,
    /// 工作履历
    pub work_history: Vec<UserInfoItem>,
    /// 偏好习惯
    pub preferences: Vec<UserInfoItem>,
    /// 人际关系
    pub relationships: Vec<UserInfoItem>,
    /// 目标规划
    pub goals: Vec<UserInfoItem>,
}

impl UserProfile {
    /// 创建新的空档案
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 添加信息条目
    pub fn add_item(&mut self, item: UserInfoItem) {
        match item.category {
            UserInfoCategory::PersonalInfo => self.personal_info.push(item),
            UserInfoCategory::WorkHistory => self.work_history.push(item),
            UserInfoCategory::Preferences => self.preferences.push(item),
            UserInfoCategory::Relationships => self.relationships.push(item),
            UserInfoCategory::Goals => self.goals.push(item),
        }
    }
    
    /// 获取指定类别的信息
    pub fn get_category(&self, category: &UserInfoCategory) -> &Vec<UserInfoItem> {
        match category {
            UserInfoCategory::PersonalInfo => &self.personal_info,
            UserInfoCategory::WorkHistory => &self.work_history,
            UserInfoCategory::Preferences => &self.preferences,
            UserInfoCategory::Relationships => &self.relationships,
            UserInfoCategory::Goals => &self.goals,
        }
    }
    
    /// 获取指定类别的可变引用
    pub fn get_category_mut(&mut self, category: &UserInfoCategory) -> &mut Vec<UserInfoItem> {
        match category {
            UserInfoCategory::PersonalInfo => &mut self.personal_info,
            UserInfoCategory::WorkHistory => &mut self.work_history,
            UserInfoCategory::Preferences => &mut self.preferences,
            UserInfoCategory::Relationships => &mut self.relationships,
            UserInfoCategory::Goals => &mut self.goals,
        }
    }
    
    /// 总信息数量
    pub fn total_count(&self) -> usize {
        self.personal_info.len()
            + self.work_history.len()
            + self.preferences.len()
            + self.relationships.len()
            + self.goals.len()
    }
    
    /// 按类别统计
    pub fn category_stats(&self) -> String {
        format!(
            "个人信息: {}, 工作履历: {}, 偏好习惯: {}, 人际关系: {}, 目标规划: {}",
            self.personal_info.len(),
            self.work_history.len(),
            self.preferences.len(),
            self.relationships.len(),
            self.goals.len()
        )
    }
    
    /// 转换为 Markdown 格式
    pub fn to_markdown(&self) -> String {
        let mut md = String::from("# 用户信息档案\n\n");
        
        if !self.personal_info.is_empty() {
            md.push_str("## 个人基本信息\n\n");
            for item in &self.personal_info {
                md.push_str(&format!(
                    "- {} (置信度: {:.2}, 重要性: {}/10)\n",
                    item.content, item.confidence, item.importance
                ));
            }
            md.push('\n');
        }
        
        if !self.work_history.is_empty() {
            md.push_str("## 工作履历\n\n");
            for item in &self.work_history {
                md.push_str(&format!(
                    "- {} (置信度: {:.2}, 重要性: {}/10)\n",
                    item.content, item.confidence, item.importance
                ));
            }
            md.push('\n');
        }
        
        if !self.preferences.is_empty() {
            md.push_str("## 偏好习惯\n\n");
            for item in &self.preferences {
                md.push_str(&format!(
                    "- {} (置信度: {:.2}, 重要性: {}/10)\n",
                    item.content, item.confidence, item.importance
                ));
            }
            md.push('\n');
        }
        
        if !self.relationships.is_empty() {
            md.push_str("## 人际关系\n\n");
            for item in &self.relationships {
                md.push_str(&format!(
                    "- {} (置信度: {:.2}, 重要性: {}/10)\n",
                    item.content, item.confidence, item.importance
                ));
            }
            md.push('\n');
        }
        
        if !self.goals.is_empty() {
            md.push_str("## 目标规划\n\n");
            for item in &self.goals {
                md.push_str(&format!(
                    "- {} (置信度: {:.2}, 重要性: {}/10)\n",
                    item.content, item.confidence, item.importance
                ));
            }
            md.push('\n');
        }
        
        md
    }
}

/// LLM 提取的原始用户信息（JSON 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedUserInfo {
    /// 个人基本信息
    #[serde(default)]
    pub personal_info: Vec<InfoItem>,
    /// 工作履历
    #[serde(default)]
    pub work_history: Vec<InfoItem>,
    /// 偏好习惯
    #[serde(default)]
    pub preferences: Vec<InfoItem>,
    /// 人际关系
    #[serde(default)]
    pub relationships: Vec<InfoItem>,
    /// 目标规划
    #[serde(default)]
    pub goals: Vec<InfoItem>,
}

/// LLM 返回的单条信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoItem {
    /// 内容
    pub content: String,
    /// 置信度（0.0-1.0）
    pub confidence: f32,
    /// 重要性（0-10）
    #[serde(default = "default_importance")]
    pub importance: u8,
}

fn default_importance() -> u8 {
    5
}

impl ExtractedUserInfo {
    /// 转换为 UserProfile
    pub fn to_user_profile(&self, source_session: &str) -> UserProfile {
        let mut profile = UserProfile::new();
        
        for item in &self.personal_info {
            profile.add_item(UserInfoItem::new(
                UserInfoCategory::PersonalInfo,
                &item.content,
                item.confidence,
                item.importance,
                source_session,
            ));
        }
        
        for item in &self.work_history {
            profile.add_item(UserInfoItem::new(
                UserInfoCategory::WorkHistory,
                &item.content,
                item.confidence,
                item.importance,
                source_session,
            ));
        }
        
        for item in &self.preferences {
            profile.add_item(UserInfoItem::new(
                UserInfoCategory::Preferences,
                &item.content,
                item.confidence,
                item.importance,
                source_session,
            ));
        }
        
        for item in &self.relationships {
            profile.add_item(UserInfoItem::new(
                UserInfoCategory::Relationships,
                &item.content,
                item.confidence,
                item.importance,
                source_session,
            ));
        }
        
        for item in &self.goals {
            profile.add_item(UserInfoItem::new(
                UserInfoCategory::Goals,
                &item.content,
                item.confidence,
                item.importance,
                source_session,
            ));
        }
        
        profile
    }
}
