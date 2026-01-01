use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use cortex_mem_config::Config;
use cortex_mem_core::memory::MemoryManager;
use cortex_mem_rig::{ListMemoriesArgs, create_memory_tools, tool::MemoryToolConfig};
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

/// Agent 抽象 trait
#[async_trait]
pub trait Agent: Send + Sync {
    /// 发送消息并获取响应
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String>;

    /// 获取 Agent 名称
    fn name(&self) -> &str;

    /// 获取 Agent 描述
    fn description(&self) -> &str;
}

/// Mock Agent 实现，用于模拟 AI 调用
pub struct MockAgent {
    name: String,
    description: String,
}

impl MockAgent {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[async_trait]
impl Agent for MockAgent {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        // 模拟 AI 响应
        let last_message = messages.last().map(|m| m.content.as_str()).unwrap_or("");

        // 简单的模拟响应逻辑
        let response = if last_message.contains("你好") || last_message.contains("hello") {
            "你好！我是你的 AI 助手。很高兴为你服务！\n\n有什么我可以帮助你的吗？".to_string()
        } else if last_message.contains("markdown") || last_message.contains("表格") {
            "# Markdown 渲染演示\n\n这是一个 **Markdown** 渲染演示，包含各种格式。\n\n## 功能列表\n\n1. 支持多级标题\n2. 支持 **粗体** 和 *斜体*\n3. 支持有序列表\n4. 支持无序列表\n5. 支持 `代码`\n6. 支持引用\n\n## 数据表格\n\n| 名称 | 类型 | 描述 |\n|------|------|------|\n| String | 字符串 | 文本数据 |\n| Number | 数字 | 数值数据 |\n| Boolean | 布尔 | 真假值 |\n\n## 代码示例\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n    let x = 42;\n    println!(\"The answer is: {}\", x);\n}\n```\n\n```python\ndef hello():\n    print(\"Hello, Python!\")\n    return True\n```\n\n## 引用示例\n\n> 这是一段引用文本。\n> 可以有多行。\n\n希望这个演示对你有帮助！".to_string()
        } else if last_message.contains("帮助") || last_message.contains("help") {
            "# 帮助信息\n\n我可以演示以下 Markdown 功能：\n\n- 输入 \"markdown\" 或 \"表格\" 查看 Markdown 渲染\n- 输入 \"你好\" 查看简单问候\n\n## 快捷键\n\n- **Enter**: 发送消息\n- **Shift+Enter**: 换行\n- **l**: 打开/关闭日志面板\n- **Esc**: 关闭日志面板\n- **q**: 退出程序".to_string()
        } else {
            format!(
                "# 响应\n\n我收到了你的消息：\n\n> {}\n\n这是一个模拟的 AI 响应。在实际使用中，这里会调用真实的 AI API。\n\n## 提示\n\n你可以尝试输入以下内容：\n\n1. \"你好\" - 查看问候\n2. \"markdown\" - 查看 Markdown 渲染效果\n3. \"帮助\" - 查看帮助信息",
                last_message
            )
        };

        Ok(response)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Agent 工厂
pub struct AgentFactory;

impl AgentFactory {
    /// 创建 Mock Agent
    pub fn create_mock_agent(name: impl Into<String>, description: impl Into<String>) -> Box<dyn Agent> {
        Box::new(MockAgent::new(name, description))
    }
}

/// 创建带记忆功能的Agent
pub async fn create_memory_agent(
    memory_manager: Arc<MemoryManager>,
    memory_tool_config: MemoryToolConfig,
    config: &Config,
) -> Result<RigAgent<CompletionModel>, Box<dyn std::error::Error>> {
    // 创建记忆工具
    let memory_tools =
        create_memory_tools(memory_manager.clone(), &config, Some(memory_tool_config));

    let llm_client = Client::builder(&config.llm.api_key)
        .base_url(&config.llm.api_base_url)
        .build();

    // 构建带有记忆工具的agent，让agent能够自主决定何时调用记忆功能
    let completion_model = llm_client
        .completion_model(&config.llm.model_efficient)
        .completions_api()
        .into_agent_builder()
        // 注册四个独立的记忆工具，保持与MCP一致
        .tool(memory_tools.store_memory())
        .tool(memory_tools.query_memory())
        .tool(memory_tools.list_memories())
        .tool(memory_tools.get_memory())
        .preamble(&format!(r#"你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

此会话发生的初始时间：{current_time}

你的工具:
- CortexMemoryTool: 可以存储、搜索和检索记忆。支持以下操作:
  * store_memory: 存储新记忆
  * query_memory: 搜索相关记忆
  * list_memories: 获取一系列的记忆集合
  * get_memory: 获取特定记忆

重要指令:
- 对话历史将作为上下文提供，请使用这些信息来理解当前的对话流程
- 用户基本信息将在上下文中提供一次，请不要再使用memory工具来创建或更新用户基本信息
- 在需要时可以自主使用memory工具搜索其他相关记忆
- 当用户提供新的重要信息时，可以主动使用memory工具存储
- 保持对话的连贯性和一致性
- 自然地融入记忆信息，避免刻意复述此前的记忆信息，关注当前的会话内容，记忆主要用于做隐式的逻辑与事实支撑
- 专注于用户的需求和想要了解的信息，以及想要你做的事情

记住：你正在与一个了解的用户进行连续对话，对话过程中不需要刻意表达你的记忆能力。"#, current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S")))
        .build();

    Ok(completion_model)
}

/// 从记忆中提取用户基本信息
pub async fn extract_user_basic_info(
    config: &Config,
    memory_manager: Arc<MemoryManager>,
    user_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let memory_tools = create_memory_tools(
        memory_manager,
        config,
        Some(MemoryToolConfig {
            default_user_id: Some(user_id.to_string()),
            ..Default::default()
        }),
    );

    let mut context = String::new();

    let search_args_personal = ListMemoriesArgs {
        limit: Some(20),
        memory_type: Some("personal".to_string()), // 使用小写以匹配新API
        user_id: Some(user_id.to_string()),
        agent_id: None,
    };

    let search_args_factual = ListMemoriesArgs {
        limit: Some(20),
        memory_type: Some("factual".to_string()), // 使用小写以匹配新API
        user_id: Some(user_id.to_string()),
        agent_id: None,
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
    _memory_manager: Arc<MemoryManager>,
    user_input: &str,
    _user_id: &str,
    user_info: Option<&str>,
    conversations: &[(String, String)],
    stream_sender: mpsc::UnboundedSender<String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 构建对话历史 - 转换为rig的Message格式
    let mut chat_history = Vec::new();
    for (user_msg, assistant_msg) in conversations {
        chat_history.push(Message::user(user_msg));
        chat_history.push(Message::assistant(assistant_msg));
    }

    // 构建system prompt，包含明确的指令
    let system_prompt = r#"你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

重要指令:
- 对话历史已提供在上下文中，请使用这些信息来理解当前的对话上下文
- 用户基本信息已在下方提供一次，请不要再使用memory工具来创建或更新用户基本信息
- 在需要时可以自主使用memory工具搜索其他相关记忆
- 当用户提供新的重要信息时，可以主动使用memory工具存储
- 保持对话的连贯性和一致性
- 自然地融入记忆信息，避免显得刻意
- 专注于用户的需求和想要了解的信息，以及想要你做的事情

记住：你正在与一个了解的用户进行连续对话，对话过程中不需要刻意表达你的记忆能力。"#;

    // 构建完整的prompt
    let prompt_content = if let Some(info) = user_info {
        format!(
            "{}\n\n用户基本信息:\n{}\n\n当前用户输入: {}",
            system_prompt, info, user_input
        )
    } else {
        format!("{}\n\n当前用户输入: {}", system_prompt, user_input)
    };

    log::debug!("正在生成AI回复（真实流式模式）...");

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
            Ok(stream_item) => {
                // 根据rig的流式响应类型处理
                match stream_item {
                    MultiTurnStreamItem::StreamItem(content) => {
                        match content {
                            StreamedAssistantContent::Text(text_content) => {
                                let text = text_content.text;
                                full_response.push_str(&text);

                                // 发送流式内容到UI
                                if let Err(_) = stream_sender.send(text) {
                                    // 如果发送失败，说明接收端已关闭，停止流式处理
                                    break;
                                }
                            }
                            StreamedAssistantContent::ToolCall(_) => {
                                // 处理工具调用（如果需要）
                                log::debug!("收到工具调用");
                            }
                            StreamedAssistantContent::Reasoning(_) => {
                                // 处理推理过程（如果需要）
                                log::debug!("收到推理过程");
                            }
                            StreamedAssistantContent::Final(_) => {
                                // 处理最终响应
                                log::debug!("收到最终响应");
                            }
                            StreamedAssistantContent::ToolCallDelta { .. } => {
                                // 处理工具调用增量
                                log::debug!("收到工具调用增量");
                            }
                        }
                    }
                    MultiTurnStreamItem::FinalResponse(final_response) => {
                        // 处理最终响应
                        log::debug!("收到最终响应: {}", final_response.response());
                        full_response = final_response.response().to_string();
                        break;
                    }
                    _ => {
                        // 处理其他未知的流式项目类型
                        log::debug!("收到未知的流式项目类型");
                    }
                }
            }
            Err(e) => {
                log::error!("流式处理错误: {}", e);
                return Err(format!("Streaming error: {}", e).into());
            }
        }
    }

    log::debug!("AI回复生成完成");
    Ok(full_response.trim().to_string())
}

/// 批量存储对话到记忆系统（优化版）
pub async fn store_conversations_batch(
    memory_manager: Arc<MemoryManager>,
    conversations: &[(String, String)],
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 只创建一次ConversationProcessor实例
    let conversation_processor =
        cortex_mem_rig::processor::ConversationProcessor::new(memory_manager);

    let metadata = cortex_mem_core::types::MemoryMetadata::new(
        cortex_mem_core::types::MemoryType::Conversational,
    )
    .with_user_id(user_id.to_string());

    // 将对话历史转换为消息格式
    let mut messages = Vec::new();
    for (user_msg, assistant_msg) in conversations {
        // 添加用户消息
        messages.push(cortex_mem_core::types::Message {
            role: "user".to_string(),
            content: user_msg.clone(),
            name: None,
        });

        // 添加助手回复
        messages.push(cortex_mem_core::types::Message {
            role: "assistant".to_string(),
            content: assistant_msg.clone(),
            name: None,
        });
    }

    // 一次性处理所有消息
    conversation_processor
        .process_turn(&messages, metadata)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_agent() {
        let agent = AgentFactory::create_mock_agent("TestBot", "A test agent");

        let messages = vec![
            ChatMessage::system("你是一个有用的助手"),
            ChatMessage::user("你好"),
        ];

        let response = agent.chat(&messages).await.unwrap();
        assert!(response.contains("你好"));
    }
}