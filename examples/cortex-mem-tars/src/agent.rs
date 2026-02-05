use anyhow::Result;
use chrono::{DateTime, Local};
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

/// Message role
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Chat message
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

/// Store conversation batch to memory
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

/// Extract user basic info from memory
pub async fn extract_user_basic_info(
    operations: Arc<MemoryOperations>,
    thread_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // Search for user-related memories
    let results = operations
        .search("user information personal facts", Some(thread_id), 10)
        .await?;

    if results.is_empty() {
        return Ok(None);
    }

    let mut context = String::from("User information from memory:\n");
    for (i, memory) in results.iter().enumerate() {
        context.push_str(&format!("{}. {}\n", i + 1, memory.content));
    }

    Ok(Some(context))
}

/// Create system prompt for the agent
pub fn create_system_prompt(
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    thread_id: &str,
) -> String {
    let base_system_prompt = if let Some(info) = user_info {
        format!(
            r#"You are an AI assistant with memory capabilities.

Current session time: {current_time}
Session ID: {thread_id}

User information:
{info}

Instructions:
- When users share important information, you can mention that you'll remember it
- Reference previous context naturally when relevant
- Be helpful, friendly, and concise in your responses
- Focus on the user's needs and questions"#,
            current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            thread_id = thread_id,
            info = info
        )
    } else {
        format!(
            r#"You are an AI assistant with memory capabilities.

Current session time: {current_time}
Session ID: {thread_id}

Instructions:
- When users share important information, you can mention that you'll remember it
- Reference previous context naturally when relevant
- Be helpful, friendly, and concise in your responses
- Focus on the user's needs and questions"#,
            current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            thread_id = thread_id
        )
    };

    if let Some(bot_prompt) = bot_system_prompt {
        format!("{}\n\nYour role:\n{}", base_system_prompt, bot_prompt)
    } else {
        base_system_prompt
    }
}

/// Simple agent reply (non-streaming for now)
pub async fn agent_reply(
    api_base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_input: &str,
    history: &[ChatMessage],
) -> Result<String, Box<dyn std::error::Error>> {
    use serde_json::json;
    
    let client = reqwest::Client::new();
    
    // Build messages
    let mut messages = vec![
        json!({"role": "system", "content": system_prompt}),
    ];
    
    // Add history
    for msg in history {
        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        };
        messages.push(json!({
            "role": role,
            "content": msg.content
        }));
    }
    
    // Add current user input
    messages.push(json!({
        "role": "user",
        "content": user_input
    }));
    
    let request_body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.7,
        "max_tokens": 4096,
        "stream": false
    });
    
    let response = client
        .post(format!("{}/chat/completions", api_base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API error: {}", error_text).into());
    }
    
    let response_json: serde_json::Value = response.json().await?;
    let content = response_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("Failed to parse response")?;
    
    Ok(content.to_string())
}

/// Store user message and generate response
pub async fn handle_user_message(
    operations: Arc<MemoryOperations>,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_input: &str,
    thread_id: &str,
    history: &[ChatMessage],
) -> Result<String, Box<dyn std::error::Error>> {
    // Store user message in memory
    operations.add_message(thread_id, "user", user_input).await?;
    
    // Get response from LLM
    let response = agent_reply(
        api_base_url,
        api_key,
        model,
        system_prompt,
        user_input,
        history,
    ).await?;
    
    // Store assistant response in memory
    operations.add_message(thread_id, "assistant", &response).await?;
    
    Ok(response)
}
