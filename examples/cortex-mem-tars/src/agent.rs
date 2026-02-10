use anyhow::Result;
use chrono::{DateTime, Local};
use cortex_mem_tools::MemoryOperations;
use cortex_mem_rig::{create_memory_tools_with_tenant, create_memory_tools_with_tenant_and_llm};
use futures::StreamExt;
use rig::{
    agent::Agent as RigAgent,
    client::CompletionClient,
    providers::openai::{Client, CompletionModel},
    completion::Message,
    streaming::StreamingChat,
    message::Text,
};
use rig::agent::MultiTurnStreamItem;
use std::sync::Arc;
use tokio::sync::mpsc;

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

/// 创建带记忆功能的Agent（OpenViking 风格 + 租户隔离）
pub async fn create_memory_agent(
    data_dir: impl AsRef<std::path::Path>,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    agent_id: &str,
    _user_id: &str,
) -> Result<RigAgent<CompletionModel>, Box<dyn std::error::Error>> {
    // 创建 cortex LLMClient 用于 L0/L1 生成
    let llm_config = cortex_mem_core::llm::LLMConfig {
        api_base_url: api_base_url.to_string(),
        api_key: api_key.to_string(),
        model_efficient: model.to_string(),
        temperature: 0.1,
        max_tokens: 4096,
    };
    let cortex_llm_client: Arc<dyn cortex_mem_core::llm::LLMClient> = 
        Arc::new(cortex_mem_core::llm::LLMClientImpl::new(llm_config)?);
    
    // 创建租户工具（agent_id 作为 tenant_id）+ LLM 支持
    let memory_tools = create_memory_tools_with_tenant_and_llm(
        data_dir, 
        agent_id,
        cortex_llm_client,
    ).await?;
    
    // 创建 Rig LLM 客户端用于 Agent 对话
    let llm_client = Client::builder(api_key)
        .base_url(api_base_url)
        .build();

    // 构建 system prompt（OpenViking 风格）
    let base_system_prompt = if let Some(info) = user_info {
        format!(r#"你是一个拥有分层记忆功能的智能 AI 助手。

此会话发生的初始时间：{current_time}

你的 Bot ID：{bot_id}

记忆工具说明（OpenViking 风格分层访问）：

🔍 搜索工具：
- search(query, options): 智能搜索记忆
  - engine: "keyword"（默认）| "vector" | "hybrid"
  - return_layers: ["L0"] (默认) | ["L0", "L1"] | ["L0", "L1", "L2"]
  - scope: 搜索范围（可选）
    * 可以指定搜索范围：
      - "cortex://user/memories/" - 用户记忆
      - "cortex://agent/memories/" - Agent 记忆
      - "cortex://session/{{session_id}}/" - 特定会话
      - "cortex://resources/" - 知识库
  - 示例：search(query="Python 装饰器", return_layers=["L0"])

- find(query): 快速查找，返回 L0 摘要
  - 自动在记忆空间中搜索
  - 例如：find(query="用户偏好")

📖 分层访问工具（按需加载）：
- abstract(uri): 获取 L0 摘要（~100 tokens）- 快速判断相关性
- overview(uri): 获取 L1 概览（~2000 tokens）- 理解核心信息
- read(uri): 获取 L2 完整内容 - 仅在必须了解详细信息时使用

📂 文件系统工具：
- ls(uri, options): 列出目录内容
  - include_abstracts: 是否包含文件摘要
  - 用于浏览记忆结构

💾 存储工具：
- store(content): 存储新内容到记忆空间，自动生成 L0/L1 摘要
  - 内容会自动存储到会话中
  - 自动生成分层摘要

使用策略（重要）：
1. 优先使用 search 查找相关记忆，默认只返回 L0 摘要
2. 根据 L0 摘要判断相关性，需要更多信息时调用 overview 获取 L1
3. 仅在必须了解完整细节时调用 read 获取 L2
4. 这种渐进式加载可以大幅减少 token 消耗（节省 80-90%）
5. 重要信息自动使用 store 存储

记忆隔离说明：
- 每个 Bot 拥有独立的租户空间（物理隔离）
- 记忆组织采用 OpenViking 架构：
  - cortex://resources/ - 知识库
  - cortex://user/ - 用户记忆
  - cortex://agent/ - Agent 记忆
  - cortex://session/ - 会话记录

用户基本信息：
{info}

重要指令：
- 对话历史将作为上下文提供，请使用这些信息来理解当前的对话流程
- 自然地融入记忆信息，避免刻意复述，关注当前会话内容
- 专注于用户的需求和想要了解的信息
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            bot_id = agent_id,
            info = info)
    } else {
        format!(r#"你是一个拥有分层记忆功能的智能 AI 助手。

此会话发生的初始时间：{current_time}

你的 Bot ID：{bot_id}

记忆工具说明（OpenViking 风格分层访问）：

🔍 搜索工具：
- search(query, options): 智能搜索记忆
  - engine: "keyword"（默认）| "vector" | "hybrid"
  - return_layers: ["L0"] (默认) | ["L0", "L1"] | ["L0", "L1", "L2"]
  - scope: 搜索范围（可选）
  - 示例：search(query="Python 装饰器", return_layers=["L0"])

- find(query): 快速查找，返回 L0 摘要
  - 自动在记忆空间中搜索
  - 例如：find(query="用户偏好")

📖 分层访问工具（按需加载）：
- abstract(uri): L0 摘要（~100 tokens）- 快速判断相关性
- overview(uri): L1 概览（~2000 tokens）- 理解核心信息
- read(uri): L2 完整内容 - 仅在必要时使用

📂 文件系统工具：
- ls(uri): 列出目录内容

💾 存储工具：
- store(content): 存储新内容到你的记忆空间

使用策略：
1. 优先使用 search，默认返回 L0 摘要
2. 根据 L0 判断相关性，需要时调用 overview 获取 L1
3. 仅在必须时调用 read 获取 L2 完整内容
4. 渐进式加载可节省 80-90% token

记忆隔离说明：
- 每个 Bot 拥有独立的租户空间（物理隔离）
- 你的记忆不会与其他 Bot 共享
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            bot_id = agent_id)
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

/// Agent多轮对话处理器 - 支持流式输出和多轮工具调用
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

    /// 进行对话（流式版本，支持多轮工具调用）
    pub async fn chat_stream(
        &mut self,
        user_input: &str,
    ) -> Result<mpsc::Receiver<String>, anyhow::Error> {
        // 添加用户消息到历史
        self.history.push(ChatMessage::user(user_input));

        // 构建对话历史 - 转换为 Rig Message 格式
        let chat_history: Vec<Message> = self
            .history
            .iter()
            .filter_map(|msg| match msg.role {
                MessageRole::User => Some(Message::User {
                    content: rig::OneOrMany::one(rig::completion::message::UserContent::Text(Text {
                        text: msg.content.clone(),
                    })),
                }),
                MessageRole::Assistant => Some(Message::Assistant {
                    id: None,
                    content: rig::OneOrMany::one(rig::completion::message::AssistantContent::Text(Text {
                        text: msg.content.clone(),
                    })),
                }),
            })
            .collect();

        // 获取当前用户输入（最后一条用户消息）
        let prompt_message = Message::User {
            content: rig::OneOrMany::one(rig::completion::message::UserContent::Text(Text {
                text: user_input.to_string(),
            })),
        };

        // 创建通道用于发送流式内容
        let (tx, rx) = mpsc::channel(100);

        // 克隆 agent 用于异步任务
        let agent = self.agent.clone();

        // 在后台任务中处理流式响应
        tokio::spawn(async move {
            let mut full_response = String::new();

            // 使用 stream_chat + multi_turn 支持工具调用（Rig 0.23 风格）
            let mut stream = agent
                .stream_chat(prompt_message, chat_history)
                .multi_turn(20)  // 支持最多 20 轮工具调用
                .await;
                
            // 处理流式响应
            while let Some(item) = stream.next().await {
                match item {
                    Ok(stream_item) => {
                        match stream_item {
                            MultiTurnStreamItem::StreamItem(content) => {
                                use rig::streaming::StreamedAssistantContent;
                                match content {
                                    StreamedAssistantContent::Text(text_content) => {
                                        let text = &text_content.text;
                                        full_response.push_str(text);
                                        
                                        // 发送流式内容
                                        if tx.send(text.clone()).await.is_err() {
                                            break;
                                        }
                                    }
                                    StreamedAssistantContent::ToolCall(_) => {
                                        // 工具调用，可以选择显示
                                        log::debug!("调用工具中...");
                                    }
                                    _ => {}
                                }
                            }
                            MultiTurnStreamItem::FinalResponse(final_resp) => {
                                // 最终响应
                                full_response = final_resp.response().to_string();
                                let _ = tx.send(full_response.clone()).await;
                                break;
                            }
                            _ => {
                                // 其他类型的流式项目
                                log::debug!("收到其他类型的流式项目");
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("流式处理错误: {:?}", e);
                        let error_msg = format!("[错误: {}]", e);
                        let _ = tx.send(error_msg).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// 进行对话（非流式版本）
    pub async fn chat(
        &mut self,
        user_input: &str,
    ) -> Result<String, anyhow::Error> {
        let mut rx = self.chat_stream(user_input).await?;
        let mut response = String::new();

        while let Some(chunk) = rx.recv().await {
            response.push_str(&chunk);
        }

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
