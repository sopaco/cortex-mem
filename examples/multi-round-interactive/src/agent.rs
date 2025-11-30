use memo_config::Config;
use memo_rig::{
    memory::manager::MemoryManager,
    tool::{MemoryArgs, MemoryToolConfig, create_memory_tool},
    types::Message,
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
    let _memory_tool =
        create_memory_tool(memory_manager.clone(), &config, Some(memory_tool_config));

    let llm_client = Client::builder(&config.llm.api_key)
        .base_url(&config.llm.api_base_url)
        .build();

    let completion_model = llm_client
        .completion_model(&config.llm.model_efficient)
        .completions_api()
        .into_agent_builder()
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

/// 从当前对话历史中检索相关对话内容
pub fn retrieve_relevant_conversations(
    conversations: &[(String, String)],
    current_input: &str,
) -> String {
    if conversations.is_empty() {
        return String::new();
    }

    // 简单的关键词匹配算法
    let input_lower = current_input.to_lowercase();
    let input_words: Vec<&str> = input_lower
        .split_whitespace()
        .filter(|w| w.len() > 1) // 忽略单字符词
        .collect();

    let mut relevant_pairs = Vec::new();

    for (user_msg, assistant_msg) in conversations.iter().rev() {
        // 从最新开始
        let user_lower = user_msg.to_lowercase();
        let assistant_lower = assistant_msg.to_lowercase();

        // 计算相似度分数
        let mut score = 0;
        for word in &input_words {
            if user_lower.contains(word) || assistant_lower.contains(word) {
                score += 1;
            }
        }

        if score > 0 {
            relevant_pairs.push((score, user_msg.clone(), assistant_msg.clone()));
        }
    }

    // 按分数排序，取前3个最相关的
    relevant_pairs.sort_by(|a, b| b.0.cmp(&a.0));
    relevant_pairs.truncate(3);

    if relevant_pairs.is_empty() {
        // 如果没有匹配，返回最近的对话作为上下文
        let recent_count = std::cmp::min(3, conversations.len());
        let mut recent_context = String::new();
        recent_context.push_str("📝 最近的对话记录:\n");

        for (i, (user_msg, assistant_msg)) in
            conversations.iter().rev().take(recent_count).enumerate()
        {
            recent_context.push_str(&format!(
                "{}️⃣ User: {}\n   Assistant: {}\n\n",
                i + 1,
                user_msg,
                assistant_msg
            ));
        }
        return recent_context;
    }

    // 构建上下文
    let mut context = String::new();
    context.push_str("🧠 相关对话记录:\n");

    for (i, (_, user_msg, assistant_msg)) in relevant_pairs.iter().enumerate() {
        context.push_str(&format!(
            "{}️⃣ User: {}\n   Assistant: {}\n\n",
            i + 1,
            user_msg,
            assistant_msg
        ));
    }

    context
}

/// Agent回复函数 - 带记忆检索和利用的智能回复
pub async fn agent_reply_with_memory_retrieval(
    agent: &Agent<CompletionModel>,
    memory_manager: Arc<MemoryManager>,
    config: &Config,
    user_input: &str,
    user_id: &str,
    user_info: Option<&str>,
    conversations: &[(String, String)],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 记录开始处理
    redirect_log_to_ui("DEBUG", &format!("开始处理用户请求: {}", user_input));

    let memory_tool = create_memory_tool(
        memory_manager.clone(),
        config,
        Some(MemoryToolConfig {
            default_user_id: Some(user_id.to_string()),
            ..Default::default()
        }),
    );

    // 1. 从当前对话历史中检索相关对话（短记忆）
    redirect_log_to_ui("DEBUG", "正在检索短期记忆...");
    let conversation_context = retrieve_relevant_conversations(conversations, user_input);

    // 2. 从长期记忆系统中检索相关记忆
    redirect_log_to_ui("DEBUG", "正在检索长期记忆...");
    let search_args = MemoryArgs {
        action: "search".to_string(),
        query: Some(user_input.to_string()),
        user_id: Some(user_id.to_string()),
        limit: Some(5),
        content: None,
        memory_id: None,
        agent_id: None,
        memory_type: None,
        topics: None,
        keywords: None,
    };

    let mut long_term_context = String::new();
    if let Ok(search_result) = memory_tool.call(search_args).await {
        if let Some(data) = search_result.data {
            if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    long_term_context.push_str("🔄 长期记忆:\n");
                    for (i, result) in results.iter().enumerate() {
                        if let Some(content) = result.get("content").and_then(|c| c.as_str()) {
                            long_term_context.push_str(&format!("{}. {}\n", i + 1, content));
                        }
                    }
                    long_term_context.push_str("\n");
                    redirect_log_to_ui("DEBUG", &format!("找到 {} 条相关长期记忆", results.len()));
                } else {
                    redirect_log_to_ui("DEBUG", "未找到相关长期记忆");
                }
            }
        }
    } else {
        redirect_log_to_ui("DEBUG", "检索长期记忆时出错");
    }

    // 构建完整上下文
    let mut context = String::new();

    // 添加用户基本信息
    if let Some(info) = user_info {
        context.push_str(&format!("📋 用户档案信息:\n{}\n\n", info));
    }

    // 添加对话历史上下文
    if !conversation_context.is_empty() {
        context.push_str(&conversation_context);
        context.push_str("\n");
        redirect_log_to_ui("DEBUG", "已添加短期记忆上下文");
    } else {
        redirect_log_to_ui("DEBUG", "未找到相关短期记忆");
    }

    // 添加长期记忆上下文
    if !long_term_context.is_empty() {
        context.push_str(&long_term_context);
    }

    // 构建system prompt
    let system_prompt = r#"你是一个拥有短期和长期记忆的智能AI助手。你可以访问：

🧠 短期记忆（本次会话中的对话记录）
🔄 长期记忆（之前会话中保存的重要信息）
📋 用户档案信息

📖 记忆使用指南：
- 优先使用短期记忆来理解当前对话的上下文
- 结合长期记忆提供个性化的回复
- 如果用户提到之前讨论过的内容，参考相关记忆
- 保持对话的连贯性和一致性
- 自然地融入记忆信息，避免显得刻意

记住：你正在与一个了解的用户进行连续对话，对话过程中专注于用户的需求和想要了解的信息，以及想要你做的事情，不需要刻意向用户表达你自己在记忆能力方面的特点和行为。"#;

    // 构建prompt
    let prompt = if !context.is_empty() {
        format!(
            "{}\n\n{}\n\n💬 当前对话:\nUser: {}\nAssistant:",
            system_prompt, context, user_input
        )
    } else {
        format!(
            "{}\n\n💬 当前对话:\nUser: {}\nAssistant:",
            system_prompt, user_input
        )
    };

    redirect_log_to_ui("DEBUG", "正在生成AI回复...");
    let response = agent
        .prompt(&prompt)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    redirect_log_to_ui("DEBUG", "AI回复生成完成");
    Ok(response.trim().to_string())
}

/// 批量存储对话到记忆系统（优化版）
pub async fn store_conversations_batch(
    memory_manager: Arc<MemoryManager>,
    messages: &[Message],
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 只创建一次ConversationProcessor实例
    let conversation_processor = memo_rig::processor::ConversationProcessor::new(memory_manager);

    let metadata =
        memo_rig::types::MemoryMetadata::new(memo_rig::types::MemoryType::Conversational)
            .with_user_id(user_id.to_string());

    // 一次性处理所有消息
    let _ = conversation_processor
        .process_turn(messages, metadata)
        .await;

    Ok(())
}
