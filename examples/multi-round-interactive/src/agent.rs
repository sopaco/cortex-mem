use memo_config::Config;
use memo_rig::{
    memory::manager::MemoryManager,
    tool::{MemoryArgs, MemoryToolConfig, create_memory_tool},
};
use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::Prompt,
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
    let memory_tool = create_memory_tool(memory_manager.clone(), &config, Some(memory_tool_config));

    let llm_client = Client::builder(&config.llm.api_key)
        .base_url(&config.llm.api_base_url)
        .build();

    // 构建带有记忆工具的agent，让agent能够自主决定何时调用记忆功能
    let completion_model = llm_client
        .completion_model(&config.llm.model_efficient)
        .completions_api()
        .into_agent_builder()
        .tool(memory_tool) // 注册记忆工具
        .preamble(r#"你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

你的工具:
- memory: 可以存储、搜索和检索记忆。支持以下操作:
  * store: 存储新记忆
  * search: 搜索相关记忆
  * recall: 召回上下文
  * get: 获取特定记忆

重要指令:
- 对话历史将作为上下文提供，请使用这些信息来理解当前的对话流程
- 用户基本信息将在上下文中提供一次，请不要再使用memory工具来创建或更新用户基本信息
- 在需要时可以自主使用memory工具搜索其他相关记忆
- 当用户提供新的重要信息时，可以主动使用memory工具存储
- 保持对话的连贯性和一致性
- 自然地融入记忆信息，避免显得刻意
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
    let memory_tool = create_memory_tool(
        memory_manager,
        config,
        Some(MemoryToolConfig {
            default_user_id: Some(user_id.to_string()),
            ..Default::default()
        }),
    );

    let mut context = String::new();

    let search_args_personal = MemoryArgs {
        action: "search".to_string(),
        query: None,
        user_id: Some(user_id.to_string()),
        limit: Some(20),
        content: None,
        memory_id: None,
        agent_id: None,
        memory_type: Some("Personal".to_owned()),
        topics: None,
        keywords: None,
    };

    let search_args_factual = MemoryArgs {
        action: "search".to_string(),
        query: None,
        user_id: Some(user_id.to_string()),
        limit: Some(20),
        content: None,
        memory_id: None,
        agent_id: None,
        memory_type: Some("Factual".to_owned()),
        topics: None,
        keywords: None,
    };

    if let Ok(search_result) = memory_tool.call(search_args_personal).await {
        if let Some(data) = search_result.data {
            if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
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

    if let Ok(search_result) = memory_tool.call(search_args_factual).await {
        if let Some(data) = search_result.data {
            if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
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

/// Agent回复函数 - 基于tool call的记忆引擎使用
pub async fn agent_reply_with_memory_retrieval(
    agent: &Agent<CompletionModel>,
    _memory_manager: Arc<MemoryManager>,
    _config: &Config,
    user_input: &str,
    _user_id: &str,
    user_info: Option<&str>,
    conversations: &[(String, String)],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 记录开始处理
    redirect_log_to_ui("DEBUG", &format!("开始处理用户请求: {}", user_input));

    // 构建对话历史上下文
    let mut conversation_history = String::new();
    if !conversations.is_empty() {
        conversation_history.push_str("对话历史记录:\n");
        for (i, (user_msg, assistant_msg)) in conversations.iter().enumerate() {
            conversation_history.push_str(&format!(
                "回合 {}: 用户: {}\n助手: {}\n",
                i + 1,
                user_msg,
                assistant_msg
            ));
        }
        conversation_history.push_str("\n");
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
    let prompt = if let Some(info) = user_info {
        redirect_log_to_ui("DEBUG", "已添加用户基本信息和对话历史到上下文");
        format!(
            "{}\n\n用户基本信息:\n{}\n\n{}\n\n当前用户输入: {}",
            system_prompt, info, conversation_history, user_input
        )
    } else {
        redirect_log_to_ui("DEBUG", "已添加对话历史到上下文");
        format!(
            "{}\n\n{}\n\n当前用户输入: {}",
            system_prompt, conversation_history, user_input
        )
    };

    redirect_log_to_ui("DEBUG", "正在生成AI回复（包含历史对话上下文）...");
    let response = agent
        .prompt(&prompt)
        .multi_turn(10)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    redirect_log_to_ui("DEBUG", "AI回复生成完成");
    Ok(response.trim().to_string())
}

/// 批量存储对话到记忆系统（优化版）
pub async fn store_conversations_batch(
    memory_manager: Arc<MemoryManager>,
    conversations: &[(String, String)],
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 只创建一次ConversationProcessor实例
    let conversation_processor = memo_rig::processor::ConversationProcessor::new(memory_manager);

    let metadata =
        memo_rig::types::MemoryMetadata::new(memo_rig::types::MemoryType::Conversational)
            .with_user_id(user_id.to_string());

    // 将对话历史转换为消息格式
    let mut messages = Vec::new();
    for (user_msg, assistant_msg) in conversations {
        // 添加用户消息
        messages.push(memo_rig::types::Message {
            role: "user".to_string(),
            content: user_msg.clone(),
            name: None,
        });

        // 添加助手回复
        messages.push(memo_rig::types::Message {
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
