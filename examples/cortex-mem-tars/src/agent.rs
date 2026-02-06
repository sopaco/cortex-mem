use anyhow::Result;
use chrono::{DateTime, Local};
use cortex_mem_tools::MemoryOperations;
use cortex_mem_rig::{ListMemoriesArgs, create_memory_tools, MemoryToolConfig};
use futures::StreamExt;
use rig::{
    agent::Agent as RigAgent,
    client::CompletionClient,
    providers::openai::{Client, CompletionModel},
    tool::Tool,
};
use rig::agent::MultiTurnStreamItem;
use rig::completion::Message;
use rig::streaming::{StreamedAssistantContent, StreamingChat};
use std::sync::Arc;
use tokio::sync::mpsc;

/// 消息角色
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    System,
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

    #[allow(dead_code)]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content.into())
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content.into())
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content.into())
    }
}

/// 创建带记忆功能的Agent
pub async fn create_memory_agent(
    operations: Arc<MemoryOperations>,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    agent_id: &str,
    user_id: &str,
) -> Result<RigAgent<CompletionModel>, Box<dyn std::error::Error>> {
    // 提前获取 user_id，避免所有权问题
    let user_id_str = user_id.to_string();

    // 创建记忆工具
    let memory_tools = create_memory_tools(
        operations.clone(),
        Some(MemoryToolConfig {
            default_user_id: Some(user_id.to_string()),
            default_agent_id: Some(agent_id.to_string()),
        }),
    );

    let llm_client = Client::builder(api_key)
        .base_url(api_base_url)
        .build();

    // 构建 base system prompt，包含用户基本信息和 agent_id 说明
    let base_system_prompt = if let Some(info) = user_info {
        format!(r#"你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

此会话发生的初始时间：{current_time}

重要说明：
- 你的身份标识（agent_id）：{agent_id}
- 你服务的用户标识（user_id）：{user_id}
- 当你调用记忆工具时，必须明确传入 user_id="{user_id}" 和 agent_id="{agent_id}" 参数
- 你的记忆是独立的，只属于你这个 agent，不会与其他 agent 混淆

用户基本信息:
{info}

重要指令:
- 对话历史将作为上下文提供，请使用这些信息来理解当前的对话流程
- 用户基本信息已在上方提供，请不要再使用memory工具来创建或更新用户基本信息
- 在需要时可以自主使用memory工具搜索其他相关记忆，但必须传入正确的 user_id 和 agent_id
- 当用户提供新的重要信息时，可以主动使用memory工具存储，确保使用正确的 user_id 和 agent_id
- 保持对话的连贯性和一致性
- 自然地融入记忆信息，避免刻意复述此前的记忆信息，关注当前的会话内容，记忆主要用于做隐式的逻辑与事实支撑
- 专注于用户的需求和想要了解的信息，以及想要你做的事情

记住：你正在与一个了解的用户进行连续对话，对话过程中不需要刻意表达你的记忆能力。"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            agent_id = agent_id,
            user_id = user_id_str,
            info = info)
    } else {
        format!(r#"你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

此会话发生的初始时间：{current_time}

重要说明：
- 你的身份标识（agent_id）：{agent_id}
- 你服务的用户标识（user_id）：{user_id}
- 当你调用记忆工具时，必须明确传入 user_id="{user_id}" 和 agent_id="{agent_id}" 参数
- 你的记忆是独立的，只属于你这个 agent，不会与其他 agent 混淆

重要指令:
- 对话历史将作为上下文提供，请使用这些信息来理解当前的对话流程
- 在需要时可以自主使用memory工具搜索其他相关记忆，但必须传入正确的 user_id 和 agent_id
- 当用户提供新的重要信息时，可以主动使用memory工具存储，确保使用正确的 user_id 和 agent_id
- 保持对话的连贯性和一致性
- 自然地融入记忆信息，避免刻意复述此前的记忆信息，关注当前的会话内容，记忆主要用于做隐式的逻辑与事实支撑
- 专注于用户的需求和想要了解的信息，以及想要你做的事情

记住：你正在与一个了解的用户进行连续对话，对话过程中不需要刻意表达你的记忆能力。"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            agent_id = agent_id,
            user_id = user_id_str)
    };

    // 追加机器人系统提示词
    let system_prompt = if let Some(bot_prompt) = bot_system_prompt {
        format!("{}\n\n你的角色设定：\n{}", base_system_prompt, bot_prompt)
    } else {
        base_system_prompt
    };

    // 构建带有记忆工具的agent，让agent能够自主决定何时调用记忆功能
    let completion_model = llm_client
        .completion_model(model)
        .completions_api()
        .into_agent_builder()
        // 注册四个独立的记忆工具，保持与MCP一致
        .tool(memory_tools.store_memory())
        .tool(memory_tools.query_memory())
        .tool(memory_tools.list_memories())
        .tool(memory_tools.get_memory())
        .preamble(&system_prompt)
        .build();

    Ok(completion_model)
}

/// 从记忆中提取用户基本信息
pub async fn extract_user_basic_info(
    operations: Arc<MemoryOperations>,
    user_id: &str,
    agent_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let memory_tools = create_memory_tools(
        operations,
        Some(MemoryToolConfig {
            default_user_id: Some(user_id.to_string()),
            default_agent_id: Some(agent_id.to_string()),
        }),
    );

    let mut context = String::new();

    let search_args_personal = ListMemoriesArgs {
        limit: Some(20),
        memory_type: Some("personal".to_string()), // 使用小写以匹配新API
        user_id: Some(user_id.to_string()),
        agent_id: Some(agent_id.to_string()),
        created_after: None,
        created_before: None,
    };

    if let Ok(search_result) = memory_tools
        .list_memories()
        .call(search_args_personal)
        .await
    {
        if let Some(data) = search_result.data {
            // 根据新的MCP格式调整数据结构访问
            if let Some(results) = data.get("memories").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    context.push_str("用户基本信息 - 特征:\n");
                    for (i, result) in results.iter().enumerate() {
                        if let Some(content) = result.get("content").and_then(|c| c.as_str()) {
                            context.push_str(&format!("{}. {}\n", i + 1, content));
                        }
                    }
                    return Ok(Some(context));
                }
            }
        }
    }

    let search_args_factual = ListMemoriesArgs {
        limit: Some(20),
        memory_type: Some("factual".to_string()), // 使用小写以匹配新API
        user_id: Some(user_id.to_string()),
        agent_id: Some(agent_id.to_string()),
        created_after: None,
        created_before: None,
    };

    if let Ok(search_result) = memory_tools.list_memories().call(search_args_factual).await {
        if let Some(data) = search_result.data {
            if let Some(results) = data.get("memories").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    context.push_str("用户基本信息 - 事实:\n");
                    for (i, result) in results.iter().enumerate() {
                        if let Some(content) = result.get("content").and_then(|c| c.as_str()) {
                            context.push_str(&format!("{}. {}\n", i + 1, content));
                        }
                    }
                    return Ok(Some(context));
                }
            }
        }
    }

    match context.len() > 0 {
        true => Ok(Some(context)),
        false => Ok(None),
    }
}

/// Agent回复函数 - 基于tool call的记忆引擎使用（真实流式版本）
pub async fn agent_reply_with_memory_retrieval_streaming(
    agent: &RigAgent<CompletionModel>,
    _operations: Arc<MemoryOperations>,
    user_input: &str,
    _user_id: &str,
    conversations: &[(String, String)],
    stream_sender: mpsc::UnboundedSender<String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 构建对话历史 - 转换为rig的Message格式
    let mut chat_history = Vec::new();

    // 添加历史对话
    for (user_msg, assistant_msg) in conversations {
        chat_history.push(Message::user(user_msg));
        if !assistant_msg.is_empty() {
            chat_history.push(Message::assistant(assistant_msg));
        }
    }

    // 构建当前用户输入消息
    let prompt_content = user_input.to_string();

    log::debug!("正在生成AI回复（真实流式模式）...");
    log::debug!("当前用户输入: {}", user_input);
    log::debug!("对话历史长度: {}", chat_history.len());
    for (i, msg) in chat_history.iter().enumerate() {
        match msg {
            Message::User { .. } => log::debug!("  [{}] User message", i),
            Message::Assistant { .. } => log::debug!("  [{}] Assistant message", i),
        }
    }

    // 使用rig的真实流式API
    let prompt_message = Message::user(&prompt_content);

    // 获取流式响应
    let stream = agent
        .stream_chat(prompt_message, chat_history)
        .multi_turn(10);

    let mut full_response = String::new();

    // 处理流式响应
    let mut stream = stream.await;
    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamItem(stream_item)) => match stream_item {
                StreamedAssistantContent::Text(text) => {
                    // 收集完整响应
                    let text_str = text.text();
                    full_response.push_str(text_str);

                    // 发送流式块
                    let _ = stream_sender.send(text_str.to_string());
                }
                StreamedAssistantContent::ToolCall(_) => {
                    log::debug!("工具调用中...");
                }
                StreamedAssistantContent::ToolCallDelta { .. } => {
                    log::debug!("工具调用增量");
                }
                StreamedAssistantContent::Reasoning(_) => {
                    log::debug!("推理中...");
                }
                StreamedAssistantContent::Final(_) => {
                    log::debug!("收到最终内容");
                }
            },
            Ok(MultiTurnStreamItem::FinalResponse(final_response)) => {
                log::debug!("收到最终响应，使用量: {:?}", final_response);
            }
            Ok(_) => {
                log::debug!("收到其他类型的响应项");
            }
            Err(e) => {
                log::error!("流式响应错误: {}", e);
                return Err(e.into());
            }
        }
    }

    log::debug!("AI回复生成完成");
    Ok(full_response.trim().to_string())
}

/// 批量存储对话到记忆系统（V2简化版）
pub async fn store_conversations_batch(
    operations: Arc<MemoryOperations>,
    conversations: &[(String, String)],
    thread_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for (user_msg, assistant_msg) in conversations {
        // Store user message
        operations
            .add_message(thread_id, "user", user_msg)
            .await?;

        // Store assistant message
        operations
            .add_message(thread_id, "assistant", assistant_msg)
            .await?;
    }

    Ok(())
}
