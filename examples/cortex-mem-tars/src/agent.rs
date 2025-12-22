use cortex_mem_config::Config;
use cortex_mem_core::memory::MemoryManager;
use cortex_mem_rig::{ListMemoriesArgs, create_memory_tools, tool::MemoryToolConfig};
use rig::{
    agent::Agent,
    client::CompletionClient,
    providers::openai::{Client, CompletionModel},
    tool::Tool,
};

use std::sync::Arc;

// 导入日志重定向函数
use crate::app::redirect_log_to_ui;

/// 创建带记忆功能的Agent
pub async fn create_memory_agent(
    memory_manager: Arc<MemoryManager>,
    memory_tool_config: MemoryToolConfig,
    config: &Config,
) -> Result<Agent<CompletionModel>, Box<dyn std::error::Error>> {
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
        .preamble(r#"你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

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

记住：你正在与一个了解的用户进行连续对话，对话过程中不需要刻意表达你的记忆能力。"#)
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

use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::Message;
use rig::streaming::{StreamedAssistantContent, StreamingChat};
use tokio::sync::mpsc;

/// Agent回复函数 - 基于tool call的记忆引擎使用（真实流式版本）
pub async fn agent_reply_with_memory_retrieval_streaming(
    agent: &Agent<CompletionModel>,
    _memory_manager: Arc<MemoryManager>,
    user_input: &str,
    _user_id: &str,
    user_info: Option<&str>,
    conversations: &[(String, String)],
    stream_sender: mpsc::UnboundedSender<String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 记录开始处理
    redirect_log_to_ui("DEBUG", &format!("开始处理用户请求: {}", user_input));

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
        redirect_log_to_ui("DEBUG", "已添加用户基本信息和对话历史到上下文");
        format!(
            "{}\n\n用户基本信息:\n{}\n\n当前用户输入: {}",
            system_prompt, info, user_input
        )
    } else {
        redirect_log_to_ui("DEBUG", "已添加对话历史到上下文");
        format!("{}\n\n当前用户输入: {}", system_prompt, user_input)
    };

    redirect_log_to_ui("DEBUG", "正在生成AI回复（真实流式模式）...");

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
                                redirect_log_to_ui("DEBUG", "收到工具调用");
                            }
                            StreamedAssistantContent::Reasoning(_) => {
                                // 处理推理过程（如果需要）
                                redirect_log_to_ui("DEBUG", "收到推理过程");
                            }
                            StreamedAssistantContent::Final(_) => {
                                // 处理最终响应
                                redirect_log_to_ui("DEBUG", "收到最终响应");
                            }
                            StreamedAssistantContent::ToolCallDelta { .. } => {
                                // 处理工具调用增量
                                redirect_log_to_ui("DEBUG", "收到工具调用增量");
                            }
                        }
                    }
                    MultiTurnStreamItem::FinalResponse(final_response) => {
                        // 处理最终响应
                        redirect_log_to_ui(
                            "DEBUG",
                            &format!("收到最终响应: {}", final_response.response()),
                        );
                        full_response = final_response.response().to_string();
                        break;
                    }
                    _ => {
                        // 处理其他未知的流式项目类型
                        redirect_log_to_ui("DEBUG", "收到未知的流式项目类型");
                    }
                }
            }
            Err(e) => {
                redirect_log_to_ui("ERROR", &format!("流式处理错误: {}", e));
                return Err(format!("Streaming error: {}", e).into());
            }
        }
    }

    redirect_log_to_ui("DEBUG", "AI回复生成完成");
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
