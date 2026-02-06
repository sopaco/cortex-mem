# 🔧 TARS Memory Capabilities Fix Report

## 📊 问题分析

经过仔细对比新旧版本，发现 **新版本缺少了完整的记忆功能集成**：

### ❌ 当前问题

1. **缺少 Rig Agent**: 新版本使用简单的 HTTP 调用而非带工具的 Agent
2. **缺少记忆工具**: 没有集成 `cortex-mem-rig` 的记忆工具
3. **System Prompt 不完整**: 缺少记忆相关的提示词
4. **无法使用工具**: Agent 无法主动调用记忆存储/检索工具

### ✅ 旧版本的正确实现

旧版本使用了：
```rust
// 1. 创建带记忆工具的 Agent
let memory_tools = create_memory_tools(memory_manager, config, memory_tool_config);

let agent = llm_client
    .completion_model(model)
    .into_agent_builder()
    .tool(memory_tools.store_memory())      // 存储记忆
    .tool(memory_tools.query_memory())      // 查询记忆
    .tool(memory_tools.list_memories())     // 列出记忆
    .tool(memory_tools.get_memory())        // 获取单个记忆
    .preamble(&system_prompt)
    .build();

// 2. 详细的 System Prompt
你是一个拥有记忆功能的智能AI助手。你可以访问和使用记忆工具来检索、存储和管理用户信息。

重要说明：
- 你的身份标识（agent_id）：{agent_id}
- 你服务的用户标识（user_id）：{user_id}
- 当你调用记忆工具时，必须明确传入 user_id 和 agent_id 参数
- 在需要时可以自主使用memory工具搜索其他相关记忆
- 当用户提供新的重要信息时，可以主动使用memory工具存储

// 3. 流式响应处理
agent_reply_with_memory_retrieval_streaming(
    &rig_agent,
    memory_manager,
    user_input,
    conversations,
    stream_sender,
).await
```

---

## 🔧 修复方案

### 1. 添加依赖

**文件**: `Cargo.toml`
```toml
cortex-mem-rig = { path = "../../cortex-mem-rig" }
```

### 2. 重写 agent.rs

已创建新版本，主要改动：
- ✅ 添加 `create_memory_agent()` 函数
- ✅ 集成四个记忆工具（store/query/list/get）
- ✅ 完整的中文 system prompt
- ✅ 添加 `agent_reply_with_memory_streaming()` 流式响应

### 3. 修改 app.rs

需要修改的部分：
- ✅ 添加 `rig_agent: Option<RigAgent<CompletionModel>>` 字段
- ✅ 添加 `user_id: String` 字段（用于记忆工具）
- ✅ 移除 `system_prompt` 字段（由 agent 管理）
- ⚠️ 修改 `send_message()` 方法使用 rig agent

---

## 🎯 核心差异对比

| 功能 | 旧版本（正确） | 新版本（缺失） |
|------|--------------|--------------|
| Agent 类型 | `RigAgent<CompletionModel>` | 简单 HTTP 调用 |
| 记忆工具 | 4个工具（store/query/list/get） | ❌ 无 |
| System Prompt | 详细的中文提示 + agent_id/user_id | 简单的英文提示 |
| 工具调用 | ✅ 自主调用 | ❌ 无法调用 |
| 流式响应 | ✅ 支持 | ❌ 不支持 |

---

## 📝 完整修复步骤

### Step 1: 更新 Cargo.toml ✅
已完成，添加了 `cortex-mem-rig` 依赖

### Step 2: 重写 agent.rs ✅
已完成，新文件包含：
- `create_memory_agent()` - 创建带记忆工具的 Agent
- `extract_user_basic_info()` - 提取用户基本信息
- `agent_reply_with_memory_streaming()` - 流式响应处理
- `store_conversations_batch()` - 批量存储对话

### Step 3: 修改 app.rs ⏳
需要手动修改 `send_message()` 方法，因为文件太大无法自动替换

**关键改动**:
```rust
// 1. 在初次发送消息时创建 rig agent
if self.rig_agent.is_none() {
    let agent = create_memory_agent(
        operations.clone(),
        &self.llm_config.api_base_url,
        &self.llm_config.api_key,
        &self.llm_config.model,
        user_info.as_deref(),
        bot_prompt,
        &self.thread_id,
        &self.user_id,
    ).await?;
    self.rig_agent = Some(agent);
}

// 2. 使用 rig agent 生成响应
if let Some(rig_agent) = &self.rig_agent {
    agent_reply_with_memory_streaming(
        &rig_agent,
        &user_input,
        &conversations,
        msg_tx.clone(),
    ).await
}
```

---

## 🚀 预期效果

修复后，Agent 将能够：

1. **主动存储记忆**: 当用户提供重要信息时，自动调用 `store_memory` 工具
2. **主动检索记忆**: 需要上下文时，自动调用 `query_memory` 工具  
3. **列出相关记忆**: 使用 `list_memories` 工具获取特定类型的记忆
4. **获取具体记忆**: 使用 `get_memory` 工具获取单个记忆详情

示例对话：
```
用户: 我最喜欢吃四川菜
Agent: [调用 store_memory 工具存储这个偏好]
      好的，我记住了您喜欢四川菜！

用户: 推荐一家餐厅
Agent: [调用 query_memory 查询用户偏好]
      根据您喜欢四川菜的偏好，我推荐...
```

---

## ⚠️ 注意事项

1. 确保 `cortex-mem-rig` 项目已正确构建
2. LLM 必须支持 tool calling（如 GPT-4、Claude 等）
3. System Prompt 包含了 user_id 和 agent_id，确保记忆隔离
4. 流式响应可以看到 agent 的工具调用过程

---

## 📚 相关文件

- ✅ `examples/cortex-mem-tars/Cargo.toml` - 已更新
- ✅ `examples/cortex-mem-tars/src/agent.rs` - 已重写
- ⏳ `examples/cortex-mem-tars/src/app.rs` - 需要手动修改第306-435行

---

**状态**: 部分完成，需要手动修改 app.rs 的 send_message() 方法
