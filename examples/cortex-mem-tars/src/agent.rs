use anyhow::Result;
use chrono::{DateTime, Local};
use cortex_mem_tools::MemoryOperations;
use cortex_mem_rig::create_memory_tools;
use rig::{
    agent::Agent as RigAgent,
    client::CompletionClient,
    providers::openai::{Client, CompletionModel},
    completion::Prompt,
};
use std::sync::Arc;

/// 消息角色
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Local>,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: Local::now(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content.into())
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content.into())
    }
}

/// 创建带记忆功能的Agent（OpenViking 风格）
pub async fn create_memory_agent(
    operations: Arc<MemoryOperations>,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    _agent_id: &str,
    _user_id: &str,
) -> Result<RigAgent<CompletionModel>, Box<dyn std::error::Error>> {
    // 创建新的 OpenViking 风格记忆工具
    let memory_tools = create_memory_tools(operations.clone());

    let llm_client = Client::builder(api_key)
        .base_url(api_base_url)
        .build();

    // 构建 system prompt（OpenViking 风格）
    let base_system_prompt = if let Some(info) = user_info {
        format!(r#"你是一个拥有分层记忆功能的智能 AI 助手。

此会话发生的初始时间：{current_time}

记忆工具说明（OpenViking 风格分层访问）：

🔍 搜索工具：
- search(query, options): 智能搜索记忆
  - engine: "keyword"（默认）| "vector" | "hybrid"
  - return_layers: ["L0"] (默认) | ["L0", "L1"] | ["L0", "L1", "L2"]
  - scope: 搜索范围，支持以下格式：
    * "cortex://threads" - 所有对话线程（默认）
    * "cortex://agents" - 所有 Agent 记忆
    * "cortex://users" - 所有用户记忆
    * "cortex://global" - 全局共享记忆
    * "cortex://threads/thread_123" - 特定线程
  - 示例：search(query="Python 装饰器", return_layers=["L0"])

- find(query, scope): 快速查找，返回 L0 摘要
  - scope 参数同上，会自动修正为有效的 dimension
  - 例如：find(query="系统状态", scope="cortex://threads")
  - 注意：不要使用 "cortex://system" 等无效 dimension

📖 分层访问工具（按需加载）：
- abstract(uri): 获取 L0 摘要（~100 tokens）- 快速判断相关性
- overview(uri): 获取 L1 概览（~2000 tokens）- 理解核心信息
- read(uri): 获取 L2 完整内容 - 仅在必须了解详细信息时使用

📂 文件系统工具：
- ls(uri, options): 列出目录内容
  - include_abstracts: 是否包含文件摘要
  - 用于浏览记忆结构

💾 存储工具：
- store(content, thread_id): 存储新内容，自动生成 L0/L1 摘要

使用策略（重要）：
1. 优先使用 search 查找相关记忆，默认只返回 L0 摘要
2. 根据 L0 摘要判断相关性，需要更多信息时调用 overview 获取 L1
3. 仅在必须了解完整细节时调用 read 获取 L2
4. 这种渐进式加载可以大幅减少 token 消耗（节省 80-90%）
5. 重要信息自动使用 store 存储

用户基本信息：
{info}

重要指令：
- 对话历史将作为上下文提供，请使用这些信息来理解当前的对话流程
- 自然地融入记忆信息，避免刻意复述，关注当前会话内容
- 专注于用户的需求和想要了解的信息
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            info = info)
    } else {
        format!(r#"你是一个拥有分层记忆功能的智能 AI 助手。

此会话发生的初始时间：{current_time}

记忆工具说明（OpenViking 风格分层访问）：

🔍 搜索工具：
- search(query, options): 智能搜索记忆
  - engine: "keyword"（默认）| "vector" | "hybrid"
  - return_layers: ["L0"] (默认) | ["L0", "L1"] | ["L0", "L1", "L2"]
  - scope: 搜索范围，支持以下格式：
    * "cortex://threads" - 所有对话线程（默认）
    * "cortex://agents" - 所有 Agent 记忆
    * "cortex://users" - 所有用户记忆
    * "cortex://global" - 全局共享记忆
  - 示例：search(query="Python 装饰器", return_layers=["L0"])

- find(query): 快速查找，返回 L0 摘要
  - 自动在 threads 维度下搜索
  - 例如：find(query="系统状态")

📖 分层访问工具（按需加载）：
- abstract(uri): L0 摘要（~100 tokens）- 快速判断相关性
- overview(uri): L1 概览（~2000 tokens）- 理解核心信息
- read(uri): L2 完整内容 - 仅在必要时使用

📂 文件系统工具：
- ls(uri): 列出目录内容

💾 存储工具：
- store(content, thread_id): 存储新内容

使用策略：
1. 优先使用 search，默认返回 L0 摘要
2. 根据 L0 判断相关性，需要时调用 overview 获取 L1
3. 仅在必须时调用 read 获取 L2 完整内容
4. 渐进式加载可节省 80-90% token
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"))
    };

    // 追加机器人系统提示词
    let system_prompt = if let Some(bot_prompt) = bot_system_prompt {
        format!("{}\n\n你的角色设定：\n{}", base_system_prompt, bot_prompt)
    } else {
        base_system_prompt
    };

    // 构建带有新的 OpenViking 风格记忆工具的 agent
    let completion_model = llm_client
        .completion_model(model)
        .completions_api()
        .into_agent_builder()
        .preamble(&system_prompt)
        // ==================== 新的 OpenViking 风格工具 ====================
        // 搜索工具（最常用）
        .tool(memory_tools.search_tool())
        .tool(memory_tools.find_tool())
        // 分层访问工具
        .tool(memory_tools.abstract_tool())
        .tool(memory_tools.overview_tool())
        .tool(memory_tools.read_tool())
        // 文件系统工具
        .tool(memory_tools.ls_tool())
        // 存储工具
        .tool(memory_tools.store_tool())
        .build();

    Ok(completion_model)
}

/// 从记忆中提取用户基本信息（使用新的 search 工具）
pub async fn extract_user_basic_info(
    operations: Arc<MemoryOperations>,
    user_id: &str,
    _agent_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // 使用新的 search 工具查找用户相关信息
    let search_args = cortex_mem_tools::SearchArgs {
        query: format!("用户 {} 的基本信息", user_id),
        engine: Some("keyword".to_string()),
        recursive: Some(true),
        return_layers: Some(vec!["L1".to_string()]),  // 获取 L1 概览
        scope: Some(format!("cortex://threads")),
        limit: Some(10),
    };

    match operations.search(search_args).await {
        Ok(response) => {
            if response.results.is_empty() {
                return Ok(None);
            }

            let mut context = String::new();
            context.push_str("用户相关信息:\n");

            for (i, result) in response.results.iter().enumerate() {
                if let Some(overview) = &result.overview_text {
                    context.push_str(&format!("{}. {}\n", i + 1, overview));
                }
            }

            Ok(Some(context))
        }
        Err(e) => {
            tracing::warn!("Failed to extract user info: {}", e);
            Ok(None)
        }
    }
}

/// Agent多轮对话处理器
pub struct AgentChatHandler {
    agent: RigAgent<CompletionModel>,
    history: Vec<ChatMessage>,
}

impl AgentChatHandler {
    pub fn new(agent: RigAgent<CompletionModel>) -> Self {
        Self {
            agent,
            history: Vec::new(),
        }
    }

    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    /// 进行对话（简化版本，使用 prompt）
    pub async fn chat(
        &mut self,
        user_input: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // 添加用户消息到历史
        self.history.push(ChatMessage::user(user_input));

        // 构建完整的提示（包含历史）
        let mut full_prompt = String::new();
        for msg in &self.history {
            match msg.role {
                MessageRole::User => full_prompt.push_str(&format!("User: {}\n", msg.content)),
                MessageRole::Assistant => full_prompt.push_str(&format!("Assistant: {}\n", msg.content)),
            }
        }
        full_prompt.push_str("Assistant: ");

        // 使用 prompt 而不是 chat
        let response = self.agent.prompt(&full_prompt).await?;

        // 添加助手回复到历史
        self.history.push(ChatMessage::assistant(response.clone()));

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }
}
