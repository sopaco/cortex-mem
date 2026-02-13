use crate::{
    extraction::{
        ExtractedMemories, ExtractedUserInfo, MemoryExtractor, UserInfoCategory, UserProfile,
    },
    filesystem::CortexFilesystem,
    llm::LLMClient,
    session::{manager::SessionMetadata, SessionManager},
    Result,
};
use std::sync::Arc;
use tracing::{info, warn};

/// 会话自动提取配置
#[derive(Debug, Clone)]
pub struct AutoExtractConfig {
    /// 触发自动提取的最小消息数
    pub min_message_count: usize,
    /// 是否在会话关闭时自动提取
    pub extract_on_close: bool,
    /// 是否保存用户记忆
    pub save_user_memories: bool,
    /// 是否保存Agent记忆
    pub save_agent_memories: bool,
}

impl Default for AutoExtractConfig {
    fn default() -> Self {
        Self {
            min_message_count: 5,
            extract_on_close: true,
            save_user_memories: true,
            save_agent_memories: true,
        }
    }
}

/// 自动提取统计
#[derive(Debug, Clone, Default)]
pub struct AutoExtractStats {
    pub facts_extracted: usize,
    pub decisions_extracted: usize,
    pub entities_extracted: usize,
    pub user_memories_saved: usize,
    pub agent_memories_saved: usize,
}

/// 会话自动提取器
///
/// 参考OpenViking的自迭代机制：
/// 1. 在会话关闭时自动触发LLM提取
/// 2. 将提取的记忆分类存储（用户记忆、Agent记忆）
/// 3. 支持增量更新
pub struct AutoExtractor {
    filesystem: Arc<CortexFilesystem>,
    #[allow(dead_code)]
    llm: Arc<dyn LLMClient>,
    extractor: MemoryExtractor,
    config: AutoExtractConfig,
    /// 用户ID，用于保存用户记忆
    user_id: String,
}

impl AutoExtractor {
    /// 创建新的自动提取器
    pub fn new(
        filesystem: Arc<CortexFilesystem>,
        llm: Arc<dyn LLMClient>,
        config: AutoExtractConfig,
    ) -> Self {
        let extraction_config = crate::extraction::ExtractionConfig::default();
        let extractor = MemoryExtractor::new(filesystem.clone(), llm.clone(), extraction_config);

        Self {
            filesystem,
            llm,
            extractor,
            config,
            user_id: "default".to_string(),
        }
    }

    /// 创建新的自动提取器，指定用户ID
    pub fn with_user_id(
        filesystem: Arc<CortexFilesystem>,
        llm: Arc<dyn LLMClient>,
        config: AutoExtractConfig,
        user_id: impl Into<String>,
    ) -> Self {
        let extraction_config = crate::extraction::ExtractionConfig::default();
        let extractor = MemoryExtractor::new(filesystem.clone(), llm.clone(), extraction_config);

        Self {
            filesystem,
            llm,
            extractor,
            config,
            user_id: user_id.into(),
        }
    }

    /// 设置用户ID
    pub fn set_user_id(&mut self, user_id: impl Into<String>) {
        self.user_id = user_id.into();
    }

    /// 在会话关闭时自动提取
    pub async fn on_session_close(
        &self,
        session: &SessionMetadata,
    ) -> Result<Option<AutoExtractStats>> {
        if !self.config.extract_on_close {
            return Ok(None);
        }

        // 检查消息数是否达到阈值
        if session.message_count < self.config.min_message_count {
            info!(
                "Session {} has only {} messages, skipping auto-extraction (threshold: {})",
                session.thread_id, session.message_count, self.config.min_message_count
            );
            return Ok(None);
        }

        info!(
            "Auto-extracting memories from session: {} ({} messages)",
            session.thread_id, session.message_count
        );

        // 执行提取
        let extracted = self
            .extractor
            .extract_from_thread(&session.thread_id)
            .await?;

        let mut stats = AutoExtractStats {
            facts_extracted: extracted.facts.len(),
            decisions_extracted: extracted.decisions.len(),
            entities_extracted: extracted.entities.len(),
            user_memories_saved: 0,
            agent_memories_saved: 0,
        };

        // 保存提取结果
        self.extractor
            .save_extraction(&session.thread_id, &extracted)
            .await?;

        // 分类存储记忆
        if self.config.save_user_memories {
            stats.user_memories_saved = self
                .save_user_memories(&session.thread_id, &extracted)
                .await?;
        }

        if self.config.save_agent_memories {
            stats.agent_memories_saved = self
                .save_agent_memories(&session.thread_id, &extracted)
                .await?;
        }

        info!(
            "Auto-extraction complete: {} facts, {} decisions, {} entities",
            stats.facts_extracted, stats.decisions_extracted, stats.entities_extracted
        );

        Ok(Some(stats))
    }

    /// 保存用户记忆（结构化版本）
    ///
    /// 新的实现：
    /// 1. 使用 LLM 提取结构化用户信息（5 大类）
    /// 2. 读取已有记忆，传给 LLM 作为上下文
    /// 3. LLM 判断是更新还是新增
    /// 4. 控制每类记忆的总量
    async fn save_user_memories(
        &self,
        thread_id: &str,
        _extracted: &ExtractedMemories,
    ) -> Result<usize> {
        // 使用配置的用户ID
        let profile_uri = format!("cortex://user/{}/profile.json", self.user_id);

        // Step 1: 读取已有的用户档案
        let existing_profile = self.load_user_profile(&profile_uri).await?;

        // Step 2: 从当前 session 提取用户信息
        let new_info = self
            .extract_user_info_structured(thread_id, &existing_profile)
            .await?;

        // Step 3: 合并新旧信息（LLM 驱动的去重和更新）
        let merged_profile = self
            .merge_user_profiles(existing_profile, new_info, thread_id)
            .await?;

        // Step 4: 控制每类记忆的总量
        let limited_profile = self.limit_profile_size(merged_profile);

        // Step 5: 保存更新后的档案
        let saved_count = self
            .save_user_profile(&profile_uri, &limited_profile)
            .await?;

        info!(
            "用户记忆已更新: {} ({}) for user: {}",
            saved_count,
            limited_profile.category_stats(),
            self.user_id
        );

        Ok(saved_count)
    }

    /// 加载用户档案
    async fn load_user_profile(&self, profile_uri: &str) -> Result<UserProfile> {
        use crate::filesystem::FilesystemOperations;

        match self.filesystem.read(profile_uri).await {
            Ok(content) => match serde_json::from_str::<UserProfile>(&content) {
                Ok(profile) => {
                    info!("已加载用户档案: {}", profile.category_stats());
                    Ok(profile)
                }
                Err(e) => {
                    warn!("解析用户档案失败: {}, 创建新档案", e);
                    Ok(UserProfile::new())
                }
            },
            Err(_) => {
                info!("用户档案不存在，创建新档案");
                Ok(UserProfile::new())
            }
        }
    }

    /// 从 session 提取结构化用户信息
    async fn extract_user_info_structured(
        &self,
        thread_id: &str,
        existing_profile: &UserProfile,
    ) -> Result<ExtractedUserInfo> {
        // 读取 session 的所有对话
        let timeline_uri = format!("cortex://session/{}/timeline", thread_id);
        let conversation = self.collect_conversation_text(&timeline_uri).await?;

        if conversation.is_empty() {
            return Ok(ExtractedUserInfo {
                personal_info: vec![],
                work_history: vec![],
                preferences: vec![],
                relationships: vec![],
                goals: vec![],
            });
        }

        // 构建包含已有记忆的 Prompt
        let existing_context = existing_profile.to_markdown();

        let prompt = self.build_extraction_prompt(&conversation, &existing_context);

        // 调用 LLM 提取
        info!(
            "调用 LLM 提取用户信息，对话长度: {} 字符",
            conversation.len()
        );
        let response = self.llm.complete(&prompt).await?;

        // 记录 LLM 返回的原始内容（用于调试）
        info!("LLM 返回内容长度: {} 字符", response.len());
        tracing::debug!("LLM 原始返回: {}", response);

        // 解析 JSON
        match serde_json::from_str::<ExtractedUserInfo>(&response) {
            Ok(mut info) => {
                // 后处理：过滤掉与已有记忆重复的内容
                info = self.filter_duplicate_info(info, existing_profile);

                info!(
                    "成功解析用户信息（去重后）: {} 条个人信息, {} 条工作履历, {} 条偏好, {} 条关系, {} 条目标",
                    info.personal_info.len(),
                    info.work_history.len(),
                    info.preferences.len(),
                    info.relationships.len(),
                    info.goals.len()
                );
                Ok(info)
            }
            Err(e) => {
                warn!("❌ 解析 LLM 返回的用户信息失败: {}", e);
                warn!("LLM 返回内容: {}", response);

                // 尝试提取 JSON 部分（可能被包裹在 ```json ... ``` 中）
                let json_content = if response.contains("```json") {
                    // 提取 ```json ... ``` 之间的内容
                    response
                        .split("```json")
                        .nth(1)
                        .and_then(|s| s.split("```").next())
                        .unwrap_or(&response)
                        .trim()
                } else if response.contains("```") {
                    // 提取 ``` ... ``` 之间的内容
                    response.split("```").nth(1).unwrap_or(&response).trim()
                } else {
                    &response
                };

                // 再次尝试解析
                match serde_json::from_str::<ExtractedUserInfo>(json_content) {
                    Ok(info) => {
                        info!("✅ 从 Markdown 代码块中提取 JSON 成功");
                        Ok(info)
                    }
                    Err(e2) => {
                        warn!("❌ 再次解析失败: {}", e2);
                        // 返回空结果而不是失败
                        Ok(ExtractedUserInfo {
                            personal_info: vec![],
                            work_history: vec![],
                            preferences: vec![],
                            relationships: vec![],
                            goals: vec![],
                        })
                    }
                }
            }
        }
    }

    /// 构建结构化提取 Prompt
    fn build_extraction_prompt(&self, conversation: &str, existing_context: &str) -> String {
        format!(
            r#"Extract REAL USER PROFILE INFORMATION from the conversation below.

## TASK
Analyze the conversation and extract information about the **REAL USER** (the human person who is chatting, NOT the AI assistant).

IMPORTANT:
- In this conversation, the REAL USER might say "I am..." or "my name is..." - extract THIS person's information
- DO NOT extract information about the AI assistant or its role-play character
- Focus on the HUMAN user's actual background, preferences, and characteristics

Extract information categorized into 5 types:

1. **personal_info** - Personal background and traits:
   - Name, age, location, education
   - Personality type (e.g., INTJ), character traits
   - Core values, beliefs
   - Example: "用户名叫 SkyronJ，是 INTJ 人格"

2. **work_history** - Professional background:
   - Current/past positions, companies
   - Key responsibilities, projects
   - Achievements, skills
   - Example: "SkyronJ 曾在SGNetworks担任技术负责人"

3. **preferences** - Likes, habits, and style:
   - Food preferences (e.g., likes Hunan cuisine)
   - Hobbies, interests
   - Work style, communication preferences
   - Daily habits
   - Example: "SkyronJ 专业领域是 Rust 技术"

4. **relationships** - Important people and organizations:
   - Colleagues, friends, family
   - Affiliation with organizations
   - Social connections
   - Example: "SkyronJ 有个同事叫李硅基"

5. **goals** - Aspirations and plans:
   - Career goals
   - Personal development goals
   - Life objectives
   - Example: "SkyronJ 的目标是成为技术领导者"

## EXISTING USER PROFILE (for reference and update)

```markdown
{}
```

## IMPORTANT RULES

1. **ONLY extract REAL USER information** (the human person). DO NOT extract:
   - ❌ AI assistant's information or role-play character
   - ❌ AI system status or errors
   - ❌ Conversation metadata ("information comes from...")
   - ❌ AI's thoughts or self-evaluations
   - ❌ General knowledge or definitions
   - ❌ Temporary conversation states

2. **Identify who is the REAL USER**:
   - Look for "I am...", "My name is...", "我叫...", "我是..." from the human
   - The person giving information about themselves is the REAL USER
   - Example: If someone says "I am SkyronJ, I work at...", extract info about SkyronJ

3. **CRITICAL: Avoid duplication with existing profile**:
   - The "EXISTING USER PROFILE" section above shows what we ALREADY know about the user
   - DO NOT extract information that is already in the existing profile
   - Only extract NEW information that is not in the existing profile
   - If the conversation mentions something we already know (e.g., "As I told you, I'm SkyronJ"), DO NOT extract it again
   - Example: If existing profile says "用户名叫 SkyronJ", and conversation mentions "I'm SkyronJ", DO NOT extract "用户名叫 SkyronJ" again

4. **Be objective and factual**:
   - Extract WHAT the user said/did, not interpretations
   - Avoid subjective evaluations unless directly stated
   - Use clear, concise language

5. **Set appropriate scores**:
   - confidence: 0.7-1.0 (how certain this is about the REAL USER)
   - importance: 1-10 (how valuable this information is)

## OUTPUT FORMAT

Return ONLY valid JSON in this exact structure:

```json
{{
  "personal_info": [
    {{
      "content": "Clear statement about real user's personal background/traits",
      "confidence": 0.85,
      "importance": 7
    }}
  ],
  "work_history": [
    {{
      "content": "Clear statement about real user's work experience",
      "confidence": 0.9,
      "importance": 8
    }}
  ],
  "preferences": [
    {{
      "content": "Clear statement about real user's preferences/habits",
      "confidence": 0.8,
      "importance": 6
    }}
  ],
  "relationships": [
    {{
      "content": "Clear statement about real user's relationships",
      "confidence": 0.85,
      "importance": 7
    }}
  ],
  "goals": [
    {{
      "content": "Clear statement about real user's goals",
      "confidence": 0.8,
      "importance": 8
    }}
  ]
}}
```

## CONVERSATION

```
{}
```

## RESPONSE

Return ONLY the JSON object. No additional text before or after."#,
            existing_context, conversation
        )
    }

    /// 收集 session 的对话文本（使用 Box::pin 避免递归问题）
    fn collect_conversation_text<'a>(
        &'a self,
        timeline_uri: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            use crate::filesystem::FilesystemOperations;

            let mut conversation = String::new();

            // 递归收集所有消息文件
            if let Ok(entries) = self.filesystem.list(timeline_uri).await {
                for entry in entries {
                    if entry.is_directory && !entry.name.starts_with('.') {
                        // 递归子目录
                        let sub_text = self.collect_conversation_text(&entry.uri).await?;
                        conversation.push_str(&sub_text);
                    } else if entry.name.ends_with(".md") && !entry.name.starts_with('.') {
                        // 读取消息文件
                        if let Ok(content) = self.filesystem.read(&entry.uri).await {
                            conversation.push_str(&content);
                            conversation.push_str("\n\n---\n\n");
                        }
                    }
                }
            }

            Ok(conversation)
        })
    }

    /// 合并新旧用户档案（带去重逻辑）
    async fn merge_user_profiles(
        &self,
        existing: UserProfile,
        new_info: ExtractedUserInfo,
        source_session: &str,
    ) -> Result<UserProfile> {
        let mut merged = existing.clone();

        // 将新提取的信息转换为 UserProfile
        let new_profile = new_info.to_user_profile(source_session);

        // 合并每个类别
        for category in &[
            UserInfoCategory::PersonalInfo,
            UserInfoCategory::WorkHistory,
            UserInfoCategory::Preferences,
            UserInfoCategory::Relationships,
            UserInfoCategory::Goals,
        ] {
            let new_items = new_profile.get_category(category);

            // 收集需要添加的新项目
            let items_to_add: Vec<_> = new_items
                .into_iter()
                .filter(|new_item| {
                    // 检查是否已存在相似内容
                    let existing_items = merged.get_category(category);
                    let is_duplicate = existing_items.iter().any(|existing_item| {
                        Self::is_similar_content(&existing_item.content, &new_item.content)
                    });

                    if is_duplicate {
                        info!(
                            "跳过重复项: {}",
                            new_item.content.chars().take(50).collect::<String>()
                        );
                        false
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            // 添加新项目
            for item in items_to_add {
                merged.add_item(item);
            }
        }

        Ok(merged)
    }

    /// 检查两个内容是否相似（简单的相似度检查）
    fn is_similar_content(a: &str, b: &str) -> bool {
        // 标准化：小写、去除多余空格
        let normalize = |s: &str| -> String {
            s.to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };

        let normalized_a = normalize(a);
        let normalized_b = normalize(b);

        // 完全相同
        if normalized_a == normalized_b {
            return true;
        }

        // 包含关系（一个包含另一个的主要部分）
        let char_count_a = normalized_a.chars().count();
        let char_count_b = normalized_b.chars().count();

        if char_count_a > 10 && char_count_b > 10 {
            // 提取关键词（简单实现：取前30个字符作为"指纹"）
            let fingerprint_a: String = normalized_a.chars().take(30).collect();
            let fingerprint_b: String = normalized_b.chars().take(30).collect();

            // 如果前30个字符有80%相似，认为是重复
            let similarity = Self::calculate_similarity(&fingerprint_a, &fingerprint_b);
            if similarity > 0.8 {
                return true;
            }
        }

        false
    }

    /// 过滤掉与已有记忆重复的新信息
    fn filter_duplicate_info(
        &self,
        new_info: ExtractedUserInfo,
        existing_profile: &UserProfile,
    ) -> ExtractedUserInfo {
        // 提取已有记忆的内容用于比较
        let existing_contents: Vec<&str> = existing_profile
            .personal_info
            .iter()
            .map(|item| item.content.as_str())
            .chain(
                existing_profile
                    .work_history
                    .iter()
                    .map(|item| item.content.as_str()),
            )
            .chain(
                existing_profile
                    .preferences
                    .iter()
                    .map(|item| item.content.as_str()),
            )
            .chain(
                existing_profile
                    .relationships
                    .iter()
                    .map(|item| item.content.as_str()),
            )
            .chain(
                existing_profile
                    .goals
                    .iter()
                    .map(|item| item.content.as_str()),
            )
            .collect();

        // 过滤每个类别
        let personal_info: Vec<_> = new_info
            .personal_info
            .into_iter()
            .filter(|item| {
                !existing_contents
                    .iter()
                    .any(|existing| Self::is_similar_content(&item.content, existing))
            })
            .collect();

        let work_history: Vec<_> = new_info
            .work_history
            .into_iter()
            .filter(|item| {
                !existing_contents
                    .iter()
                    .any(|existing| Self::is_similar_content(&item.content, existing))
            })
            .collect();

        let preferences: Vec<_> = new_info
            .preferences
            .into_iter()
            .filter(|item| {
                !existing_contents
                    .iter()
                    .any(|existing| Self::is_similar_content(&item.content, existing))
            })
            .collect();

        let relationships: Vec<_> = new_info
            .relationships
            .into_iter()
            .filter(|item| {
                !existing_contents
                    .iter()
                    .any(|existing| Self::is_similar_content(&item.content, existing))
            })
            .collect();

        let goals: Vec<_> = new_info
            .goals
            .into_iter()
            .filter(|item| {
                !existing_contents
                    .iter()
                    .any(|existing| Self::is_similar_content(&item.content, existing))
            })
            .collect();

        // 记录过滤结果
        let total_new = personal_info.len()
            + work_history.len()
            + preferences.len()
            + relationships.len()
            + goals.len();
        if total_new > 0 {
            info!("过滤后新增 {} 条新记忆", total_new);
        } else {
            info!("没有新记忆需要添加（全部已在 profile 中）");
        }

        ExtractedUserInfo {
            personal_info,
            work_history,
            preferences,
            relationships,
            goals,
        }
    }

    /// 计算两个字符串的相似度（基于最长公共子串）
    fn calculate_similarity(a: &str, b: &str) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let mut max_match = 0;
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 || b_len == 0 {
            return 0.0;
        }

        let min_len = a_len.min(b_len);

        // 滑动窗口检查相似度
        for window_size in (1..=min_len).rev() {
            for i in 0..=a_len.saturating_sub(window_size) {
                let window_a: String = a_chars[i..(i + window_size).min(a_len)].iter().collect();
                // 在 b 中查找这个窗口
                for j in 0..=b_len.saturating_sub(window_size) {
                    let window_b: String =
                        b_chars[j..(j + window_size).min(b_len)].iter().collect();
                    if window_a == window_b {
                        max_match = max_match.max(window_size);
                        break;
                    }
                }
                if max_match == window_size {
                    break;
                }
            }
        }

        max_match as f64 / a_len.max(b_len) as f64
    }

    /// 限制档案大小
    fn limit_profile_size(&self, mut profile: UserProfile) -> UserProfile {
        const MAX_PER_CATEGORY: usize = 10;

        // 对每个类别，保留最重要的 N 条
        for category in &[
            UserInfoCategory::PersonalInfo,
            UserInfoCategory::WorkHistory,
            UserInfoCategory::Preferences,
            UserInfoCategory::Relationships,
            UserInfoCategory::Goals,
        ] {
            let items = profile.get_category_mut(category);

            if items.len() > MAX_PER_CATEGORY {
                // 按重要性和置信度排序
                items.sort_by(|a, b| {
                    let score_a = a.importance as f32 * a.confidence;
                    let score_b = b.importance as f32 * b.confidence;
                    score_b.partial_cmp(&score_a).unwrap()
                });

                // 只保留前 N 条
                items.truncate(MAX_PER_CATEGORY);

                info!(
                    "类别 {:?} 超出限制，保留前 {} 条",
                    category, MAX_PER_CATEGORY
                );
            }
        }

        profile
    }

    /// 保存用户档案
    async fn save_user_profile(&self, profile_uri: &str, profile: &UserProfile) -> Result<usize> {
        use crate::filesystem::FilesystemOperations;

        let json_content = serde_json::to_string_pretty(profile)?;
        self.filesystem.write(profile_uri, &json_content).await?;

        Ok(profile.total_count())
    }

    /// 保存Agent记忆
    ///
    /// 参考OpenViking的Agent Memory Update机制：
    /// - Agent学到的知识、经验、决策模式
    /// - 存储到 cortex://agent/{agent_id}/memories/
    ///
    /// 注意：使用 cortex://agent/ (单数) 而不是 agents/，这样在租户模式下会自动路由到 tenants/{tenant_id}/agent/
    async fn save_agent_memories(
        &self,
        thread_id: &str,
        extracted: &ExtractedMemories,
    ) -> Result<usize> {
        use crate::filesystem::FilesystemOperations;

        let agent_id = "tars"; // TARS Agent ID
        let memories_dir = format!("cortex://agent/{}/memories", agent_id);

        let mut saved_count = 0;

        // 保存decisions作为Agent记忆
        for decision in &extracted.decisions {
            if decision.confidence >= 0.7 {
                let memory_id = uuid::Uuid::new_v4();
                let memory_uri = format!("{}/{}.md", memories_dir, memory_id);

                let content = format!(
                    "# Agent Memory (Decision)\n\n\
                    **Source**: {}\n\
                    **Extracted**: {}\n\
                    **Confidence**: {}\n\n\
                    ## Decision\n\n\
                    {}\n\n\
                    ## Context\n\n\
                    {}\n\n\
                    ## Rationale\n\n\
                    {}\n",
                    thread_id,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    decision.confidence,
                    decision.decision,
                    decision.context,
                    decision.rationale.as_deref().unwrap_or("N/A")
                );

                match self.filesystem.write(&memory_uri, &content).await {
                    Ok(_) => {
                        saved_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to save agent memory: {}", e);
                    }
                }
            }
        }

        info!("Saved {} agent memories", saved_count);
        Ok(saved_count)
    }

    /// 手动触发提取（可用于测试或手动操作）
    pub async fn extract_session(&self, thread_id: &str) -> Result<AutoExtractStats> {
        info!("Manually extracting memories from session: {}", thread_id);

        let extracted = self.extractor.extract_from_thread(thread_id).await?;

        let mut stats = AutoExtractStats {
            facts_extracted: extracted.facts.len(),
            decisions_extracted: extracted.decisions.len(),
            entities_extracted: extracted.entities.len(),
            user_memories_saved: 0,
            agent_memories_saved: 0,
        };

        // 保存提取结果
        self.extractor
            .save_extraction(thread_id, &extracted)
            .await?;

        // 分类存储
        if self.config.save_user_memories {
            stats.user_memories_saved = self.save_user_memories(thread_id, &extracted).await?;
        }

        if self.config.save_agent_memories {
            stats.agent_memories_saved = self.save_agent_memories(thread_id, &extracted).await?;
        }

        Ok(stats)
    }
}

/// 增强SessionManager支持自动提取
pub struct AutoSessionManager {
    session_manager: SessionManager,
    auto_extractor: Option<Arc<AutoExtractor>>,
}

impl AutoSessionManager {
    /// 创建新的自动会话管理器
    pub fn new(
        session_manager: SessionManager,
        auto_extractor: Option<Arc<AutoExtractor>>,
    ) -> Self {
        Self {
            session_manager,
            auto_extractor,
        }
    }

    /// 关闭会话并自动提取
    pub async fn close_session(&mut self, thread_id: &str) -> Result<SessionMetadata> {
        // 先关闭会话
        let metadata = self.session_manager.close_session(thread_id).await?;

        // 如果配置了自动提取器，执行提取
        if let Some(extractor) = &self.auto_extractor {
            match extractor.on_session_close(&metadata).await {
                Ok(Some(stats)) => {
                    info!(
                        "Session {} auto-extraction: {} facts, {} user memories, {} agent memories",
                        thread_id,
                        stats.facts_extracted,
                        stats.user_memories_saved,
                        stats.agent_memories_saved
                    );
                }
                Ok(None) => {
                    info!("Session {} skipped auto-extraction", thread_id);
                }
                Err(e) => {
                    warn!("Session {} auto-extraction failed: {}", thread_id, e);
                }
            }
        }

        Ok(metadata)
    }

    /// 获取内部SessionManager的引用
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// 获取内部SessionManager的可变引用
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_extract_config_default() {
        let config = AutoExtractConfig::default();
        assert_eq!(config.min_message_count, 5);
        assert!(config.extract_on_close);
        assert!(config.save_user_memories);
        assert!(config.save_agent_memories);
    }
}
