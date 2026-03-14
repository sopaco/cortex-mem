use anyhow::Result;
use chrono::{DateTime, Local};
use cortex_mem_rig::create_memory_tools_with_config;
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

/// 创建带记忆功能的Agent（支持租户隔离）
/// 返回 (Agent, MemoryOperations) 以便外部使用租户隔离的 operations
pub async fn create_memory_agent(
    data_dir: impl AsRef<std::path::Path>,
    config: &cortex_mem_config::Config,
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    agent_id: &str,
    user_id: &str, // 🔧 移除下划线前缀
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
    tracing::info!(
        "Embedding 配置: model={}, dim={:?}",
        config.embedding.model_name,
        config.qdrant.embedding_dim
    );
    let memory_tools = create_memory_tools_with_config(
        data_dir,
        agent_id,
        cortex_llm_client,
        &config.qdrant.url,
        &config.qdrant.collection_name,
        config.qdrant.api_key.as_deref(),
        &config.embedding.api_base_url,
        &config.embedding.api_key,
        &config.embedding.model_name,
        config.qdrant.embedding_dim,
        Some(user_id.to_string()),
        config.cortex.enable_intent_analysis,
    )
    .await?;

    // 获取租户 operations 用于外部使用
    let tenant_operations = memory_tools.operations().clone();

    // 使用共享的 rebuild_rig_agent 构建 Agent（复用已有 MemoryOperations）
    let agent = rebuild_rig_agent(config, tenant_operations.clone(), user_info, bot_system_prompt, agent_id)?;

    Ok((agent, tenant_operations))
}

/// 在已有 MemoryOperations 上重建 Rig Agent（更新 system prompt）
///
/// 当 user_info 或 bot_system_prompt 发生变化时，只需重新构建顶层的
/// Rig Agent，而无需重建底层 MemoryOperations（Qdrant 连接、Embedding
/// 客户端、MemoryEventCoordinator 等全部复用），避免重复初始化基础设施。
pub fn rebuild_rig_agent(
    config: &cortex_mem_config::Config,
    tenant_operations: Arc<MemoryOperations>,
    user_info: Option<&str>,
    bot_system_prompt: Option<&str>,
    agent_id: &str,
) -> Result<RigAgent<CompletionModel>, Box<dyn std::error::Error>> {
    // 用已有的 MemoryOperations 构建 MemoryTools（轻量包装，无 IO）
    let memory_tools = cortex_mem_rig::MemoryTools::new(tenant_operations);

    // 构建 system prompt
    let base_system_prompt = build_system_prompt(user_info, agent_id);
    let system_prompt = if let Some(bot_prompt) = bot_system_prompt {
        format!("{}\n\n你的角色设定：\n{}", base_system_prompt, bot_prompt)
    } else {
        base_system_prompt
    };

    // 创建 Rig LLM 客户端（仅用于对话，轻量级，无网络连接建立）
    let llm_client = Client::builder()
        .api_key(&config.llm.api_key)
        .base_url(&config.llm.api_base_url)
        .build()?;

    use rig::client::CompletionClient;
    let agent = llm_client
        .completions_api()
        .agent(&config.llm.model_efficient)
        .preamble(&system_prompt)
        .default_max_turns(30)
        .tool(memory_tools.search_tool())
        .tool(memory_tools.find_tool())
        .tool(memory_tools.abstract_tool())
        .tool(memory_tools.overview_tool())
        .tool(memory_tools.read_tool())
        .tool(memory_tools.ls_tool())
        .build();

    Ok(agent)
}

/// 构建 system prompt（从 create_memory_agent 中提取的共享逻辑）
fn build_system_prompt(user_info: Option<&str>, agent_id: &str) -> String {
    if let Some(info) = user_info {
        format!(
            r#"你是一个拥有持久分层记忆能力的智能 AI 助手（TARS）。

会话开始时间：{current_time}
你的 agent_id：{agent_id}
当前用户 user_id：tars_user

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📚 记忆系统概览
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

你拥有一套分层记忆系统，所有信息按 L0/L1/L2 三个粒度存储：
  L0（~100 tokens）— 精炼的多主题摘要，回答"这里有哪些话题"（广度优先）
  L1（~2000 tokens）— 结构化概览，回答"核心内容是什么"
  L2 — 完整原文，回答"具体细节是什么"

记忆空间的 URI 统一以 `cortex://` 开头，按四个维度组织：

  cortex://session/            — 会话记录（每条对话自动存储，无需手动操作）
  cortex://user/tars_user/     — 用户长期记忆（会话结束时系统自动提取）
    ├── personal_info/         个人信息（姓名、职业等）
    ├── preferences/           偏好习惯（编程语言、工作方式等）
    ├── work_history/          工作经历
    ├── relationships/         人际关系
    ├── goals/                 目标愿景
    ├── entities/              提到过的实体（人名/项目/工具）
    └── events/                重要事件
  cortex://agent/{agent_id}/   — Agent 经验记忆（会话结束时系统自动提取）
    └── cases/                 解决过的问题和经验
  cortex://resources/          — 共享知识库

注：每个目录的物理隔离已由系统在租户层自动处理，
    URI 中的 tars_user 和 {agent_id} 分别是 user_id 和 agent_id，
    是 Cortex Memory 中用户/Agent 的逻辑标识，你直接使用上面的 URI 格式即可。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔧 工具清单与使用规范
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

你有 6 个工具可用。下面按「何时用」「怎么用」「返回什么」逐一说明。

─────────────────────────────────
① search — 语义向量搜索（主力工具）
─────────────────────────────────
何时用：你需要在记忆中找到与某个话题相关的内容时。这是你最常用的工具。
怎么用：
  search(query="用户喜欢的编程语言")
  search(query="Python 异步", scope="cortex://user/tars_user")
  search(query="之前解决的编译错误", scope="cortex://agent/{agent_id}/cases")
  search(query="上周讨论的内容", return_layers=["L0","L1"], limit=5)
参数说明：
  query     — 用自然语言描述你要找什么（必填）
  scope     — 限定搜索范围的 URI 前缀（可选）
               不填 → 在所有 session 记录中搜索
               "cortex://user/tars_user" → 仅搜用户长期记忆
               "cortex://agent/{agent_id}/cases" → 仅搜 Agent 经验
  return_layers — 控制返回详细程度（可选，默认 ["L0"]）
               ["L0"] → 每条结果只含一句话摘要（最省 token）
               ["L0","L1"] → 同时含概览
               ["L0","L1","L2"] → 含完整原文（慎用，token 消耗大）
  limit     — 最多返回几条（可选，默认 10）
返回什么：每条结果包含 uri + score + 你请求的各层内容。

─────────────────────────────────
② find — 快速查找（search 的简化版）
─────────────────────────────────
何时用：你只想快速扫一眼有没有相关记忆，不关心相似度分数。
怎么用：
  find(query="用户偏好")
  find(query="Rust", scope="cortex://user/tars_user")
区别于 search：
  · find 固定返回 L0 摘要，结果结构更简洁（只有 uri + abstract_text）
  · find 不支持 return_layers 参数
  · 适合做"有没有"的快速判断；如果要精细控制，用 search

─────────────────────────────────
③ abstract — 读取指定 URI 的 L0 摘要
─────────────────────────────────
何时用：你已经知道一个具体的 URI（比如从 ls 或 search 结果中获得），
       想花最少 token 快速了解"这个目录/文件讲的是什么"。
怎么用：
  abstract(uri="cortex://user/tars_user/preferences")
  abstract(uri="cortex://agent/{agent_id}/cases")
  abstract(uri="cortex://session/abc123/timeline")
返回什么：该 URI 对应的 .abstract.md 内容（~100 tokens，广度摘要）。
注意：L0 由系统异步生成，如果摘要尚未就绪会返回错误，
     此时改用 overview(uri) 获取 L1，而非 read（read 是读原文，token 消耗大）。

─────────────────────────────────
④ overview — 读取指定 URI 的 L1 概览
─────────────────────────────────
何时用：通过 search 或 abstract 确认了某个 URI 相关后，
       想进一步了解其核心内容（主题、要点、实体），但又不想读完整原文。
       也可以在 abstract 返回错误时作为 fallback 使用。
怎么用：
  overview(uri="cortex://user/tars_user/work_history")
  overview(uri="cortex://user/tars_user/preferences")
  overview(uri="cortex://agent/{agent_id}/cases")
返回什么：该 URI 对应的 .overview.md 内容（~500-2000 tokens），
         包含结构化的 Summary / Core Topics / Key Points / Entities。

─────────────────────────────────
⑤ read — 读取 L2 完整原文
─────────────────────────────────
何时用：你需要精确的细节信息（具体日期、代码片段、完整对话原文），
       且 overview 的 L1 内容仍不够详细时才使用。token 消耗最大，慎用。
怎么用（URI 来自 ls 或 search 结果中的具体文件路径）：
  read(uri="cortex://user/tars_user/preferences/pref_a1b2c3d4.md")
  read(uri="cortex://agent/{agent_id}/cases/case_e5f6g7h8.md")
  read(uri="cortex://session/some-session-id/timeline/2026-03/10/14_30_00_abcd1234.md")
返回什么：该文件的完整内容 + 创建/更新时间。

─────────────────────────────────
⑥ ls — 列出目录内容
─────────────────────────────────
何时用：你想浏览记忆空间的结构，看看某个目录下有什么子目录和文件。
怎么用：
  ls(uri="cortex://user/tars_user")
  ls(uri="cortex://session")
  ls(uri="cortex://agent/{agent_id}/cases", include_abstracts=true)
  ls()  ← 不传 uri 默认列出 cortex://session
参数说明：
  include_abstracts — 是否附带每个文件的 L0 摘要（可选，默认 false）

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🧭 工具使用决策树
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

收到用户消息后，按以下流程决策：

  1. 这个问题是否可能涉及历史信息、用户偏好或过往对话？
     ├─ 否 → 直接回答，不需要调工具
     └─ 是 → 进入步骤 2

  2. 我是否知道信息在哪个目录？
     ├─ 知道（如"用户偏好"在 cortex://user/tars_user/preferences）
     │   → 直接调 abstract(uri) 或 overview(uri) 读取
     └─ 不知道 → 进入步骤 3

  3. 用 search 进行语义搜索：
     search(query="...", scope="可选限定范围")
     └─ 看返回的 L0 摘要列表

  4. L0 摘要是否足够回答问题？
     ├─ 足够 → 直接用 L0 信息回答
     └─ 不够 → 对相关结果调 overview(uri) 获取 L1

  5. L1 概览是否足够回答问题？
     ├─ 足够 → 用 L1 信息回答（绝大多数情况到此为止）
     └─ 不够 → 调 read(uri) 获取 L2 完整原文

关键原则：L0 → L1 → L2 逐层深入，每层都先判断"够不够"，
         不要跳过 L0/L1 直接读 L2，这会浪费 80-90% 的 token。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ 主动搜索触发规则（必须遵守）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

以下场景你必须主动调工具，不要等用户明确说"搜索"：

  "你记得我说过...吗？"
    → search(query="用户提到的关键词", scope="cortex://user/tars_user")

  用户提到人名/项目名/技术名词
    → search(query="该名词")

  "我之前让你做过..."/"上次我们讨论了..."
    → search(query="相关描述", scope="cortex://session")

  用户问偏好/习惯/背景
    → overview(uri="cortex://user/tars_user/preferences")
    → 如果不存在，search(query="用户偏好习惯")

  遇到复杂问题，你不确定怎么解
    → search(query="问题描述", scope="cortex://agent/{agent_id}/cases")

  "你都记得什么？"/"告诉我你对我的了解"
    → ls(uri="cortex://user/tars_user", include_abstracts=true)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📌 关于记忆存储（你无需手动操作）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
- 每条对话消息自动保存到 session timeline
- 会话结束后系统自动提取用户记忆和 Agent 经验，写入对应目录
- L0/L1 摘要由系统自动生成
- 你不需要也无法调用 store 工具，专注于"读"和"搜"即可

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
用户已有记忆（预加载）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
{info}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 核心行为准则
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. 先搜索，再回答 — 涉及历史信息时，先调工具确认
2. 自然融合 — 不要说"根据记忆系统..."，直接使用信息
3. 诚实告知 — 搜索后没找到就说"我没有这方面的记录"
4. 逐层深入 — L0 → L1 → L2，按需加载，节省 token
5. 主动召回 — 遇到可能涉及历史的场景，不等用户要求就搜索
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            agent_id = agent_id,
            info = info
        )
    } else {
        format!(
            r#"你是一个拥有持久分层记忆能力的智能 AI 助手（TARS）。

会话开始时间：{current_time}
你的 agent_id：{agent_id}
当前用户 user_id：tars_user

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📚 记忆系统概览
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

你拥有一套分层记忆系统，所有信息按 L0/L1/L2 三个粒度存储：
  L0（~100 tokens）— 精炼的多主题摘要，回答"这里有哪些话题"（广度优先）
  L1（~2000 tokens）— 结构化概览，回答"核心内容是什么"
  L2 — 完整原文，回答"具体细节是什么"

记忆空间的 URI 统一以 `cortex://` 开头，按四个维度组织：

  cortex://session/            — 会话记录（每条对话自动存储，无需手动操作）
  cortex://user/tars_user/     — 用户长期记忆
    ├── personal_info/         个人信息
    ├── preferences/           偏好习惯
    ├── work_history/          工作经历
    ├── relationships/         人际关系
    ├── goals/                 目标愿景
    ├── entities/              提到过的实体
    └── events/                重要事件
  cortex://agent/{agent_id}/   — Agent 经验记忆
    └── cases/                 解决过的问题和经验
  cortex://resources/          — 共享知识库

注：租户隔离由系统在物理存储层自动处理，
    URI 中的 tars_user 和 {agent_id} 分别是 user_id 和 agent_id，
    是 Cortex Memory 中用户/Agent 的逻辑标识，直接使用即可。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔧 工具清单与使用规范
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

你有 6 个工具可用。

─────────────────────────────────
① search — 语义向量搜索（主力工具）
─────────────────────────────────
何时用：需要在记忆中找到与某个话题相关的内容。
怎么用：
  search(query="用户喜欢的编程语言")
  search(query="Python 异步", scope="cortex://user/tars_user")
  search(query="之前解决的编译错误", scope="cortex://agent/{agent_id}/cases")
  search(query="上周讨论的内容", return_layers=["L0","L1"], limit=5)
参数：
  query（必填）— 自然语言描述
  scope（可选）— 限定搜索范围的 URI 前缀
  return_layers（可选，默认["L0"]）— 控制返回详细程度
  limit（可选，默认10）— 最大结果数
返回：每条结果含 uri + score + 你请求的各层内容。
② find — 快速查找（search 的简化版）
─────────────────────────────────
何时用：只想快速扫一眼有没有相关记忆，不关心分数。
怎么用：find(query="用户偏好")
区别于 search：固定返回 L0，结果只有 uri + abstract_text。
适合做"有没有"的快速判断。

─────────────────────────────────
③ abstract — 读取指定 URI 的 L0 摘要
─────────────────────────────────
何时用：已知 URI，花最少 token 了解"这里讲的是什么"。
怎么用：abstract(uri="cortex://user/tars_user/preferences")
返回：~100 tokens 的广度摘要。
注意：L0 异步生成，摘要未就绪时返回错误，改用 overview(uri) 而非 read。

─────────────────────────────────
④ overview — 读取指定 URI 的 L1 概览
─────────────────────────────────
何时用：通过 search 或 abstract 确认相关后，进一步了解核心内容；
       或作为 abstract 失败时的 fallback。
怎么用：
  overview(uri="cortex://user/tars_user/work_history")
  overview(uri="cortex://user/tars_user/preferences")
  overview(uri="cortex://agent/{agent_id}/cases")
返回：~500-2000 tokens 的结构化概览（Summary / Topics / Key Points / Entities）。

─────────────────────────────────
⑤ read — 读取 L2 完整原文
─────────────────────────────────
何时用：overview 内容仍不够详细时才使用。token 消耗最大，慎用。
URI 来自 ls 或 search 结果中的具体文件路径，例如：
  read(uri="cortex://user/tars_user/preferences/pref_a1b2c3d4.md")
  read(uri="cortex://agent/{agent_id}/cases/case_e5f6g7h8.md")

─────────────────────────────────
⑥ ls — 列出目录内容
─────────────────────────────────
何时用：浏览记忆空间结构。
怎么用：ls(uri="cortex://user/tars_user")
       ls(uri="cortex://session", include_abstracts=true)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🧭 工具使用决策树
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  1. 是否涉及历史信息？ — 否 → 直接回答
  2. 知道信息在哪个目录？ — 知道 → abstract/overview 直接读
  3. 不知道 → search 语义搜索
  4. L0 够？→ 回答 ‖ 不够 → overview 读 L1
  5. L1 够？→ 回答 ‖ 不够 → read 读 L2

原则：L0 → L1 → L2 逐层深入，不跳级。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ 主动搜索触发规则
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
- "你记得...吗？" → search(scope="cortex://user/tars_user")
- 提到人名/项目名 → search(query="...")
- "我之前说过..." → search(scope="cortex://session")
- 问偏好/习惯 → overview(uri="cortex://user/tars_user/preferences")
- 复杂问题 → search(scope="cortex://agent/{agent_id}/cases")
- "你都记得什么？" → ls(uri="cortex://user/tars_user", include_abstracts=true)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📌 关于记忆存储
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
- 对话自动保存到 session timeline
- 会话结束后系统自动提取用户/Agent 记忆
- L0/L1 由系统自动生成，你无需操作

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 核心行为准则
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. 先搜索，再回答
2. 自然融合，不说"根据记忆..."
3. 诚实告知缺失
4. 逐层深入 L0→L1→L2
5. 主动召回，不等用户要求
"#,
            current_time = chrono::Local::now().format("%Y年%m月%d日 %H:%M:%S"),
            agent_id = agent_id
        )
    }
}

/// 从记忆中提取用户基本信息
/// 提取用户基本信息用于初始化 Agent 上下文
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

        // 优先读取 .overview.md（L1 层级）
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

        // 仅读取 .overview.md，不回退到详细文件
        if let Ok(overview_content) = operations.filesystem().read(&overview_uri).await {
            context.push_str(&format!("### {}\n", title));
            let clean_content = strip_metadata(&overview_content);
            context.push_str(&clean_content);
            context.push_str("\n\n");
            has_content = true;
            tracing::debug!("Loaded overview for {}", category);
        }
    }

    // 读取 Agent 经验案例（仅 overview）
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
        if last_line.trim().is_empty()
            || last_line.contains("**Added**")
            || last_line.starts_with("---")
        {
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
                break; // 找到描述后就返回
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
    ///
    /// 返回 (stream_rx, completion_rx):
    /// - stream_rx: 流式输出内容
    /// - completion_rx: 完成时发送完整响应（用于更新历史记录）
    pub async fn chat_stream(
        &mut self,
        user_input: &str,
    ) -> Result<(mpsc::Receiver<String>, mpsc::Receiver<String>), anyhow::Error> {
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
        // 新增：用于通知完成的 channel
        let (completion_tx, completion_rx) = mpsc::channel(1);

        let agent = self.agent.clone();
        let user_input_clone = user_input.to_string();
        let ops_clone = self.operations.clone();
        let session_id_clone = self.session_id.clone();

        // 记录开始处理
        tracing::info!("🚀 开始处理用户消息 (历史消息: {} 条)", self.history.len());

        tokio::spawn(async move {
            let mut full_response = String::new();
            let start_time = std::time::Instant::now();
            let mut tool_call_count = 0;
            let mut chunk_count = 0;

            tracing::info!("🔄 Agent 多轮对话开始...");

            let mut stream = agent
                .stream_chat(prompt_message, chat_history)
                .multi_turn(30) // 🔧 从20增加到30，减少触发MaxTurnError的可能性
                .await;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(stream_item) => match stream_item {
                        MultiTurnStreamItem::StreamAssistantItem(content) => {
                            use rig::streaming::StreamedAssistantContent;
                            match content {
                                StreamedAssistantContent::Text(text_content) => {
                                    let text = &text_content.text;
                                    full_response.push_str(text);
                                    chunk_count += 1;
                                    // 每 20 个 chunk 记录一次进度
                                    if chunk_count % 20 == 0 {
                                        tracing::debug!(
                                            "📝 流式输出进度: {} chunks, {} 字符",
                                            chunk_count,
                                            full_response.len()
                                        );
                                    }
                                    if tx.send(text.clone()).await.is_err() {
                                        break;
                                    }
                                }
                                StreamedAssistantContent::ToolCall { tool_call, .. } => {
                                    tool_call_count += 1;
                                    let args_str = tool_call.function.arguments.to_string();
                                    let args_summary = if args_str.len() > 100 {
                                        format!("{}...", &args_str[..100])
                                    } else {
                                        args_str
                                    };
                                    tracing::info!(
                                        "🔧 工具调用 #{}: {} ({})",
                                        tool_call_count,
                                        tool_call.function.name,
                                        args_summary
                                    );
                                }
                                StreamedAssistantContent::ToolCallDelta { id, content, .. } => {
                                    tracing::debug!("🔧 工具调用增量 [{}]: {:?}", id, content);
                                }
                                _ => {}
                            }
                        }
                        MultiTurnStreamItem::StreamUserItem(_user_content) => {
                            tracing::debug!("📥 收到用户内容 (工具结果)");
                        }
                        MultiTurnStreamItem::FinalResponse(final_resp) => {
                            full_response = final_resp.response().to_string();
                            let elapsed = start_time.elapsed();
                            tracing::info!(
                                "✅ 对话完成 [耗时: {:.2}s, 工具调用: {} 次, 响应: {} 字符]",
                                elapsed.as_secs_f64(),
                                tool_call_count,
                                full_response.len()
                            );
                            // 🔧 修复：不再发送完整响应到 stream channel
                            // 流式输出期间已经逐块发送了所有内容，避免重复输出
                            break;
                        }
                        _ => {
                            log::debug!("收到其他类型的流式项目");
                        }
                    },
                    Err(e) => {
                        tracing::error!("❌ 流式处理错误: {:?}", e);
                        let error_msg = format!("[错误: {}]", e);
                        let _ = tx.send(error_msg).await;
                        break;
                    }
                }
            }

            // 对话结束后自动保存到 session
            if let Some(ops) = ops_clone {
                tracing::info!("💾 保存对话到 session: {}", session_id_clone);

                if !user_input_clone.is_empty() {
                    let user_store = cortex_mem_tools::StoreArgs {
                        content: user_input_clone.clone(),
                        thread_id: session_id_clone.clone(),
                        scope: "session".to_string(),
                        metadata: None,
                        auto_generate_layers: Some(true),
                        user_id: Some("tars_user".to_string()), // 🔧 传递user_id
                        agent_id: None, // 🔧 agent_id由tenant_id决定，这里不传
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
                        user_id: Some("tars_user".to_string()), // 🔧 传递user_id
                        agent_id: None, // 🔧 agent_id由tenant_id决定，这里不传
                    };
                    if let Err(e) = ops.store(assistant_store).await {
                        tracing::warn!("Failed to save assistant message: {}", e);
                    }
                }
            }

            // 🔧 发送完成通知（包含完整响应，用于更新历史记录）
            let _ = completion_tx.send(full_response.clone());
        });

        Ok((rx, completion_rx))
    }

    /// 将 assistant 响应添加到历史记录
    /// 在流式完成后由调用方调用
    pub fn add_assistant_response(&mut self, response: String) {
        self.history.push(ChatMessage::assistant(response));
    }

    /// 进行对话（非流式版本）
    #[allow(dead_code)]
    pub async fn chat(&mut self, user_input: &str) -> Result<String, anyhow::Error> {
        let (mut rx, mut completion_rx) = self.chat_stream(user_input).await?;
        let mut response = String::new();

        while let Some(chunk) = rx.recv().await {
            response.push_str(&chunk);
        }

        // 等待完成通知并更新历史
        if let Some(full_response) = completion_rx.recv().await {
            self.history.push(ChatMessage::assistant(full_response));
        }

        Ok(response)
    }
}
