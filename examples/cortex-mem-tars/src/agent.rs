use anyhow::Result;
use chrono::{DateTime, Local};
use cortex_mem_rig::create_memory_tools_with_tenant_and_vector;
use cortex_mem_tools::MemoryOperations;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::{
    agent::Agent as RigAgent,
    completion::Message,
    message::Text,
    providers::openai::{Client, CompletionModel},
    streaming::StreamingChat,
};
use std::sync::Arc;
use tokio::sync::mpsc;

/// 消息角色
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
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
    
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content.into())
    }
}

/// 创建带记忆功能的Agent（OpenViking 风格 + 租户隔离）
/// 返回 (Agent, MemoryOperations) 以便外部使用租户隔离的 operations
pub async fn create_memory_agent(
    data_dir: impl AsRef<std::path::Path>,
    config: &cortex_mem_config::Config,
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    agent_id: &str,
    user_id: &str,  // 🔧 移除下划线前缀
) -> Result<(RigAgent<CompletionModel>, Arc<MemoryOperations>), Box<dyn std::error::Error>> {
    // 创建 cortex LLMClient 用于 L0/L1 生成
    let llm_config = cortex_mem_core::llm::LLMConfig {
        api_base_url: config.llm.api_base_url.clone(),
        api_key: config.llm.api_key.clone(),
        model_efficient: config.llm.model_efficient.clone(),
        temperature: 0.1,
        max_tokens: 4096,
    };
    let cortex_llm_client: Arc<dyn cortex_mem_core::llm::LLMClient> =
        Arc::new(cortex_mem_core::llm::LLMClientImpl::new(llm_config)?);

    // 使用向量搜索版本（唯一支持的版本）
    tracing::info!("🔍 使用向量搜索功能");
    tracing::info!("Embedding 配置: model={}, dim={:?}", config.embedding.model_name, config.qdrant.embedding_dim);
    let memory_tools = create_memory_tools_with_tenant_and_vector(
        data_dir,
        agent_id,
        cortex_llm_client,
        &config.qdrant.url,
        &config.qdrant.collection_name,
        &config.embedding.api_base_url,
        &config.embedding.api_key,
        &config.embedding.model_name,
        config.qdrant.embedding_dim,
        Some(user_id.to_string()),  // 🆕 传递真实的user_id
    )
    .await?;

    // 获取租户 operations 用于外部使用
    let tenant_operations = memory_tools.operations().clone();

    // 创建 Rig LLM 客户端用于 Agent 对话
    let llm_client = Client::builder()
        .api_key(&config.llm.api_key)
        .base_url(&config.llm.api_base_url)
        .build()?;

    // 构建 system prompt（OpenViking 风格）
    let base_system_prompt = if let Some(info) = user_info {
        format!(
            r#"你是一个拥有分层记忆功能的智能 AI 助手。

此会话发生的初始时间：{current_time}

你的 Bot ID：{bot_id}

记忆工具说明（OpenViking 风格分层访问）：

🔑 **URI 格式规范（非常重要！）**
- 所有 URI 必须使用 `cortex://` 前缀，**禁止使用 `memory://`**
- ✅ 正确示例：`cortex://user/tars_user/`
- ❌ 错误示例：`memory://me/SkyronJ/`（常见错误！）

📍 URI 路径结构：
- `cortex://user/{{user_id}}/` - 用户记忆目录
- `cortex://user/{{user_id}}/profile.json` - 用户档案
- `cortex://agent/{{agent_id}}/` - Agent 记忆目录
- `cortex://session/{{session_id}}/` - 特定会话
- `cortex://resources/` - 知识库

🔍 搜索工具：
- search(query, options): 智能搜索记忆
  - return_layers: ["L0"] (默认) | ["L0", "L1"] | ["L0", "L1", "L2"]
  - scope: 搜索范围（可选）
    * 可以指定搜索范围：
      - "cortex://user/" - 用户记忆
      - "cortex://agent/" - Agent 记忆
      - "cortex://session/{{session_id}}/" - 特定会话
      - "cortex://resources/" - 知识库
  - 示例：search(query="Python 装饰器", return_layers=["L0"])

- find(query): 快速查找，返回 L0 摘要
  - 自动在记忆空间中搜索
  - 例如：find(query="用户偏好")

📖 分层访问工具（按需加载）：
- abstract(uri): 获取 L0 摘要（~100 tokens）- 快速判断相关性
  - 示例：abstract(uri="cortex://user/tars_user/")
- overview(uri): 获取 L1 概览（~2000 tokens）- 理解核心信息
  - 示例：overview(uri="cortex://session/abc123/")
- read(uri): 获取 L2 完整内容 - 仅在必须了解详细信息时使用

📂 文件系统工具：
- ls(uri, options): 列出目录内容
  - include_abstracts: 是否包含文件摘要
  - 用于浏览记忆结构
  - ✅ 示例：ls(uri="cortex://user/tars_user/")
  - ❌ 错误：ls(uri="memory://me/SkyronJ/")

⚠️ **常见错误提醒**：
- 不要使用 `memory://` 前缀，必须用 `cortex://`
- user_id 是分配的用户标识符，不是"me"或用户名
- 访问用户记忆用 `cortex://user/{{user_id}}/`，不是 `cortex://me/`

📍 **主动召回原则**（关键）：
当用户的问题可能涉及历史信息、用户偏好或之前的对话内容时，你必须**主动**调用记忆工具。

**必须主动搜索的场景**：
- 用户问"你记得...吗？"、"告诉我你都记得什么？" → 立即调用 search 或 ls
- 用户提到人名、地点、事件、项目名 → 立即调用 search(query="人名/事件") 查找相关记忆
- 用户询问历史对话、之前的讨论 → 立即调用 search 或 find
- 用户的问题涉及用户偏好、习惯、背景 → 立即调用 search 查找用户记忆
- 你不确定如何回答，或感觉记忆中可能有相关信息 → 先调用 search 确认

**搜索策略**：
1. 优先使用 search 查找相关记忆，默认只返回 L0 摘要
2. 根据 L0 摘要判断相关性，需要更多信息时调用 overview 获取 L1
3. 仅在必须了解完整细节时调用 read 获取 L2
4. 这种渐进式加载可以大幅减少 token 消耗（节省 80-90%）

记忆隔离说明：
- 每个 Bot 拥有独立的租户空间（物理隔离）
- 记忆组织采用 OpenViking 架构：
  - cortex://resources/ - 知识库
  - cortex://user/ - 用户记忆
  - cortex://agent/ - Agent 记忆
  - cortex://session/ - 会话记录
- 对话内容会自动保存到 session，你无需关心存储

📍 **Agent经验召回**（重要）：
你可以主动搜索之前处理过的类似问题的经验案例：
- 使用 search(query="问题描述", scope="cortex://agent/{bot_id}/cases") 搜索相关经验
- Agent cases 包含了之前遇到的问题、解决方案和经验教训
- 遇到复杂问题时，优先搜索是否有相关经验可以借鉴

用户基本信息：
{info}

重要指令：
- 你是一个**主动**使用记忆的 AI 助手，不要等待用户明确说"搜索"才去查找记忆！
- 遇到任何可能涉及历史信息的问题，**先搜索，再回答**
- 自然地融入记忆信息，避免生硬地说"根据记忆..."
- 如果搜索后没有找到相关信息，诚实告知用户
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            bot_id = agent_id,
            info = info
        )
    } else {
        format!(
            r#"你是一个拥有分层记忆功能的智能 AI 助手。

此会话发生的初始时间：{current_time}

你的 Bot ID：{bot_id}

记忆工具说明（OpenViking 风格分层访问）：

🔑 **URI 格式规范（非常重要！）**
- 所有 URI 必须使用 `cortex://` 前缀，**禁止使用 `memory://`**
- ✅ 正确示例：`cortex://user/tars_user/`
- ❌ 错误示例：`memory://me/SkyronJ/`（常见错误！）

📍 URI 路径结构：
- `cortex://user/{{user_id}}/` - 用户记忆目录
- `cortex://user/{{user_id}}/profile.json` - 用户档案
- `cortex://agent/{{agent_id}}/` - Agent 记忆目录
- `cortex://session/{{session_id}}/` - 特定会话
- `cortex://resources/` - 知识库

🔍 搜索工具：
- search(query, options): 智能搜索记忆
  - return_layers: ["L0"] (默认) | ["L0", "L1"] | ["L0", "L1", "L2"]
  - scope: 搜索范围（可选）
  - 示例：search(query="Python 装饰器", return_layers=["L0"])

- find(query): 快速查找，返回 L0 摘要
  - 自动在记忆空间中搜索
  - 例如：find(query="用户偏好")

📖 分层访问工具（按需加载）：
- abstract(uri): L0 摘要（~100 tokens）- 快速判断相关性
  - 示例：abstract(uri="cortex://user/tars_user/")
- overview(uri): L1 概览（~2000 tokens）- 理解核心信息
  - 示例：overview(uri="cortex://session/abc123/")
- read(uri): L2 完整内容 - 仅在必要时使用

📂 文件系统工具：
- ls(uri): 列出目录内容
  - ✅ 示例：ls(uri="cortex://user/tars_user/")
  - ❌ 错误：ls(uri="memory://me/SkyronJ/")

⚠️ **常见错误提醒**：
- 不要使用 `memory://` 前缀，必须用 `cortex://`
- user_id 是分配的用户标识符，不是"me"或用户名
- 访问用户记忆用 `cortex://user/{{user_id}}/`，不是 `cortex://me/`

📍 **主动召回原则**（关键）：
当用户的问题可能涉及历史信息、用户偏好或之前的对话内容时，你必须**主动**调用记忆工具。

**必须主动搜索的场景**：
- 用户问"你记得...吗？"、"告诉我你都记得什么？" → 立即调用 search 或 ls
- 用户提到人名、地点、事件、项目名 → 立即调用 search(query="人名/事件") 查找
- 用户询问历史对话、之前的讨论 → 立即调用 search 或 find
- 你不确定如何回答 → 先调用 search 确认记忆中是否有相关信息

**搜索策略**：
1. 优先使用 search，默认返回 L0 摘要
2. 根据 L0 判断相关性，需要时调用 overview 获取 L1
3. 仅在必须时调用 read 获取 L2 完整内容
4. 渐进式加载可节省 80-90% token

重要指令：
- 你是一个**主动**使用记忆的 AI 助手，不要等待用户明确说"搜索"才去查找记忆！
- 遇到任何可能涉及历史信息的问题，**先搜索，再回答**
- 对话内容会自动保存到 session，你无需关心存储

记忆隔离说明：
- 每个 Bot 拥有独立的租户空间（物理隔离）
- 你的记忆不会与其他 Bot 共享
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            bot_id = agent_id
        )
    };

    // 追加机器人系统提示词
    let system_prompt = if let Some(bot_prompt) = bot_system_prompt {
        format!("{}\n\n你的角色设定：\n{}", base_system_prompt, bot_prompt)
    } else {
        base_system_prompt
    };

    // 构建带有 OpenViking 风格记忆工具的 agent
    use rig::client::CompletionClient;
    let completion_model = llm_client
        .completions_api()  // Use completions API to get CompletionModel
        .agent(&config.llm.model_efficient)
        .preamble(&system_prompt)
        .default_max_turns(30)  // 🔧 设置默认max_turns为30，避免频繁触发MaxTurnError
        // 搜索工具（最常用）
        .tool(memory_tools.search_tool())
        .tool(memory_tools.find_tool())
        // 分层访问工具
        .tool(memory_tools.abstract_tool())
        .tool(memory_tools.overview_tool())
        .tool(memory_tools.read_tool())
        // 文件系统工具
        .tool(memory_tools.ls_tool())
        .build();

    Ok((completion_model, tenant_operations))
}

/// 从记忆中提取用户基本信息
/// 🆕 提取用户基本信息用于初始化 Agent 上下文
/// 
/// 优化策略：
/// - 优先读取目录的 .overview.md（L1 层级）
/// - 如果没有 overview，回退到读取个别文件
/// - 大幅减少初始化时的 token 消耗（节省 80-90%）
pub async fn extract_user_basic_info(
    operations: Arc<MemoryOperations>,
    user_id: &str,
    _agent_id: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use cortex_mem_core::FilesystemOperations;

    tracing::info!("Loading user memories (L1 overviews) for user: {}", user_id);
    
    let mut context = String::new();
    context.push_str("## 用户记忆\n\n");
    let mut has_content = false;

    // 📋 核心信息类别（完整读取或使用 overview）
    let core_categories = vec![
        ("personal_info", "个人信息"),
        ("work_history", "工作经历"),
        ("preferences", "偏好习惯"),
    ];
    
    for (category, title) in core_categories {
        let category_uri = format!("cortex://user/{}/{}", user_id, category);
        let overview_uri = format!("{}/.overview.md", category_uri);
        
        // 🆕 优先读取 .overview.md（L1 层级）
        if let Ok(overview_content) = operations.filesystem().read(&overview_uri).await {
            context.push_str(&format!("### {}\n", title));
            // 移除 **Added** 时间戳
            let clean_content = strip_metadata(&overview_content);
            context.push_str(&clean_content);
            context.push_str("\n\n");
            has_content = true;
            tracing::debug!("Loaded overview for {}", category);
        } else {
            // 回退：读取个别文件
            if let Ok(entries) = operations.filesystem().list(&category_uri).await {
                if !entries.is_empty() {
                    context.push_str(&format!("### {}\n", title));
                    for entry in entries {
                        if entry.name.ends_with(".md") && !entry.name.starts_with('.') {
                            if let Ok(content) = operations.filesystem().read(&entry.uri).await {
                                let summary = extract_markdown_summary(&content);
                                if !summary.is_empty() {
                                    context.push_str(&format!("- {}\n", summary));
                                    has_content = true;
                                }
                            }
                        }
                    }
                    context.push_str("\n");
                }
            }
        }
    }

    // 📋 次要信息类别（仅使用 overview，不回退）
    let secondary_categories = vec![
        ("relationships", "人际关系"),
        ("goals", "目标愿景"),
        ("entities", "相关实体"),
        ("events", "重要事件"),
    ];
    
    for (category, title) in secondary_categories {
        let category_uri = format!("cortex://user/{}/{}", user_id, category);
        let overview_uri = format!("{}/.overview.md", category_uri);
        
        // 🆕 仅读取 .overview.md，不回退到详细文件
        if let Ok(overview_content) = operations.filesystem().read(&overview_uri).await {
            context.push_str(&format!("### {}\n", title));
            let clean_content = strip_metadata(&overview_content);
            context.push_str(&clean_content);
            context.push_str("\n\n");
            has_content = true;
            tracing::debug!("Loaded overview for {}", category);
        }
    }

    // 🆕 读取 Agent 经验案例（仅 overview）
    let cases_uri = format!("cortex://agent/{}/cases", _agent_id);
    let cases_overview_uri = format!("{}/.overview.md", cases_uri);
    
    if let Ok(overview_content) = operations.filesystem().read(&cases_overview_uri).await {
        context.push_str("### Agent经验案例\n");
        let clean_content = strip_metadata(&overview_content);
        context.push_str(&clean_content);
        context.push_str("\n\n");
        has_content = true;
        tracing::debug!("Loaded overview for agent cases");
    }

    if !has_content {
        tracing::info!("No user memories found for user: {}", user_id);
        return Ok(None);
    }

    tracing::info!("Loaded user memories (L1 overviews) for user: {}", user_id);
    Ok(Some(context))
}

/// 移除 **Added** 时间戳等元数据
fn strip_metadata(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    
    // 移除末尾的 **Added** 行
    while let Some(last_line) = lines.last() {
        if last_line.trim().is_empty() || last_line.contains("**Added**") || last_line.starts_with("---") {
            lines.pop();
        } else {
            break;
        }
    }
    
    lines.join("\n").trim().to_string()
}

/// 从markdown文件中提取关键摘要信息
fn extract_markdown_summary(content: &str) -> String {
    let mut summary = String::new();
    let mut in_content = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // 跳过空行
        if trimmed.is_empty() {
            continue;
        }
        
        // 提取标题（去掉#号）
        if trimmed.starts_with('#') {
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() && summary.is_empty() {
                summary.push_str(title);
            }
        }
        // 提取Description字段
        else if trimmed.starts_with("**Description**:") || trimmed.starts_with("**描述**:") {
            let desc = trimmed
                .trim_start_matches("**Description**:")
                .trim_start_matches("**描述**:")
                .trim();
            if !desc.is_empty() {
                if !summary.is_empty() {
                    summary.push_str(": ");
                }
                summary.push_str(desc);
                break;  // 找到描述后就返回
            }
        }
        // 提取普通内容行（不是markdown格式的）
        else if !trimmed.starts_with("**") && !trimmed.starts_with("##") && !in_content {
            if !summary.is_empty() {
                summary.push_str(": ");
            }
            summary.push_str(trimmed);
            in_content = true;
            // 只取第一行内容
            if summary.len() > 10 {
                break;
            }
        }
    }
    
    // 限制长度
    if summary.len() > 200 {
        summary.truncate(197);
        summary.push_str("...");
    }
    
    summary
}

/// Agent多轮对话处理器 - 支持流式输出和多轮工具调用
pub struct AgentChatHandler {
    agent: RigAgent<CompletionModel>,
    history: Vec<ChatMessage>,
    operations: Option<Arc<MemoryOperations>>,
    session_id: String,
}

impl AgentChatHandler {
    pub fn new(agent: RigAgent<CompletionModel>) -> Self {
        Self {
            agent,
            history: Vec::new(),
            operations: None,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create with memory operations for auto-saving conversations
    pub fn with_memory(
        agent: RigAgent<CompletionModel>,
        operations: Arc<MemoryOperations>,
        session_id: String,
    ) -> Self {
        Self {
            agent,
            history: Vec::new(),
            operations: Some(operations),
            session_id,
        }
    }

    #[allow(dead_code)]
    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    /// 进行对话（流式版本，支持多轮工具调用）
    pub async fn chat_stream(
        &mut self,
        user_input: &str,
    ) -> Result<mpsc::Receiver<String>, anyhow::Error> {
        self.history.push(ChatMessage::user(user_input));

        let chat_history: Vec<Message> = self
            .history
            .iter()
            .filter_map(|msg| match msg.role {
                MessageRole::User => Some(Message::User {
                    content: rig::OneOrMany::one(rig::completion::message::UserContent::Text(
                        Text {
                            text: msg.content.clone(),
                        },
                    )),
                }),
                MessageRole::Assistant => Some(Message::Assistant {
                    id: None,
                    content: rig::OneOrMany::one(rig::completion::message::AssistantContent::Text(
                        Text {
                            text: msg.content.clone(),
                        },
                    )),
                }),
                MessageRole::System => None, // 系统消息不参与对话
            })
            .collect();

        let prompt_message = Message::User {
            content: rig::OneOrMany::one(rig::completion::message::UserContent::Text(Text {
                text: user_input.to_string(),
            })),
        };

        let (tx, rx) = mpsc::channel(100);

        let agent = self.agent.clone();
        let user_input_clone = user_input.to_string();
        let ops_clone = self.operations.clone();
        let session_id_clone = self.session_id.clone();

        tokio::spawn(async move {
            let mut full_response = String::new();

            let mut stream = agent
                .stream_chat(prompt_message, chat_history)
                .multi_turn(30)  // 🔧 从20增加到30，减少触发MaxTurnError的可能性
                .await;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(stream_item) => {
                        match stream_item {
                            MultiTurnStreamItem::StreamAssistantItem(content) => {
                                use rig::streaming::StreamedAssistantContent;
                                match content {
                                    StreamedAssistantContent::Text(text_content) => {
                                        let text = &text_content.text;
                                        full_response.push_str(text);
                                        if tx.send(text.clone()).await.is_err() {
                                            break;
                                        }
                                    }
                                    StreamedAssistantContent::ToolCall { .. } => {
                                        log::debug!("调用工具中...");
                                    }
                                    _ => {}
                                }
                            }
                            MultiTurnStreamItem::FinalResponse(final_resp) => {
                                full_response = final_resp.response().to_string();
                                let _ = tx.send(full_response.clone()).await;
                                break;
                            }
                            _ => {
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

            // 对话结束后自动保存到 session
            if let Some(ops) = ops_clone {
                if !user_input_clone.is_empty() {
                    let user_store = cortex_mem_tools::StoreArgs {
                        content: user_input_clone.clone(),
                        thread_id: session_id_clone.clone(),
                        scope: "session".to_string(),
                        metadata: None,
                        auto_generate_layers: Some(true),
                        user_id: Some("tars_user".to_string()),  // 🔧 传递user_id
                        agent_id: None,  // 🔧 agent_id由tenant_id决定，这里不传
                    };
                    if let Err(e) = ops.store(user_store).await {
                        tracing::warn!("Failed to save user message: {}", e);
                    }
                }

                if !full_response.is_empty() {
                    let assistant_store = cortex_mem_tools::StoreArgs {
                        content: full_response.clone(),
                        thread_id: session_id_clone.clone(),
                        scope: "session".to_string(),
                        metadata: None,
                        auto_generate_layers: Some(true),
                        user_id: Some("tars_user".to_string()),  // 🔧 传递user_id
                        agent_id: None,  // 🔧 agent_id由tenant_id决定，这里不传
                    };
                    if let Err(e) = ops.store(assistant_store).await {
                        tracing::warn!("Failed to save assistant message: {}", e);
                    }
                }
            }
        });

        Ok(rx)
    }

    /// 进行对话（非流式版本）
    #[allow(dead_code)]
    pub async fn chat(&mut self, user_input: &str) -> Result<String, anyhow::Error> {
        let mut rx = self.chat_stream(user_input).await?;
        let mut response = String::new();

        while let Some(chunk) = rx.recv().await {
            response.push_str(&chunk);
        }

        self.history.push(ChatMessage::assistant(response.clone()));

        Ok(response)
    }
}