# 🔄 术语规范化：bot_id → agent_id

## 📋 问题说明

**用户反馈**：
在 Cortex Memory 框架内不应该使用 "bot_id" 的概念，而应该使用 "agent_id"。

**原因**：
- **TARS** 中的 "bot" 是 TARS 应用的业务概念
- 映射到 **Cortex Memory 框架**应该是 "agent"
- 框架层面应该使用统一的术语 "agent_id"

**架构层次**：
```
应用层 (TARS)          框架层 (Cortex Memory)
    ↓                        ↓
  Bot                      Agent
    ↓                        ↓
 bot.id      --------→    agent_id
```

---

## ✅ 修改内容

### 1. cortex-mem-rig/src/lib.rs

#### 结构体字段
```rust
// Before
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
    bot_id: Option<String>,  // ❌
}

// After
pub struct MemoryTools {
    operations: Arc<MemoryOperations>,
    agent_id: Option<String>,  // ✅
}
```

#### 构造方法
```rust
// Before
pub fn with_bot_id(operations: Arc<MemoryOperations>, bot_id: impl Into<String>) -> Self {
    Self { 
        operations,
        bot_id: Some(bot_id.into()),
    }
}

// After
pub fn with_agent_id(operations: Arc<MemoryOperations>, agent_id: impl Into<String>) -> Self {
    Self { 
        operations,
        agent_id: Some(agent_id.into()),
    }
}
```

#### 公开函数
```rust
// Before
pub fn create_memory_tools_with_bot_id(
    operations: Arc<MemoryOperations>, 
    bot_id: impl Into<String>
) -> MemoryTools

// After
pub fn create_memory_tools_with_agent_id(
    operations: Arc<MemoryOperations>, 
    agent_id: impl Into<String>
) -> MemoryTools
```

### 2. cortex-mem-rig/src/tools/mod.rs

**全局替换**：所有工具的 `bot_id` 字段改为 `agent_id`

涉及的工具：
- ✅ SearchTool
- ✅ FindTool
- ✅ LsTool
- ✅ ExploreTool
- ✅ StoreTool

**示例**（SearchTool）：
```rust
// Before
pub struct SearchTool {
    operations: Arc<MemoryOperations>,
    bot_id: Option<String>,  // ❌
}

impl SearchTool {
    pub fn new(operations: Arc<MemoryOperations>, bot_id: Option<String>) -> Self {
        Self { operations, bot_id }
    }
}

// 使用
if args.scope.is_none() && self.bot_id.is_some() {
    args.scope = Some(format!("cortex://threads/{}", self.bot_id.as_ref().unwrap()));
}

// After
pub struct SearchTool {
    operations: Arc<MemoryOperations>,
    agent_id: Option<String>,  // ✅
}

impl SearchTool {
    pub fn new(operations: Arc<MemoryOperations>, agent_id: Option<String>) -> Self {
        Self { operations, agent_id }
    }
}

// 使用
if args.scope.is_none() && self.agent_id.is_some() {
    args.scope = Some(format!("cortex://threads/{}", self.agent_id.as_ref().unwrap()));
}
```

### 3. examples/cortex-mem-tars/src/agent.rs

#### 导入语句
```rust
// Before
use cortex_mem_rig::create_memory_tools_with_bot_id;

// After
use cortex_mem_rig::create_memory_tools_with_agent_id;
```

#### 工具创建
```rust
// Before
let memory_tools = create_memory_tools_with_bot_id(operations.clone(), agent_id);

// After
let memory_tools = create_memory_tools_with_agent_id(operations.clone(), agent_id);
```

#### System Prompt 更新
```rust
// Before
你的 Bot ID：{bot_id}

记忆隔离说明：
- 每个 Bot 拥有独立的记忆空间（cortex://threads/{bot_id}）

// After
你的 Bot ID：{bot_id} (Cortex Memory Agent ID)

记忆隔离说明：
- 每个 Bot 拥有独立的记忆空间（cortex://threads/{agent_id}）
```

**说明**：
- System Prompt 中保留 "Bot ID" 是为了让 TARS 的用户理解
- 添加 "(Cortex Memory Agent ID)" 说明映射关系
- 技术细节中使用 `{agent_id}` 变量名

---

## 🎯 术语映射表

| 概念层次 | TARS 应用层 | Cortex Memory 框架层 |
|---------|------------|---------------------|
| **实体名称** | Bot | Agent |
| **标识符字段** | bot.id | agent_id |
| **内存路径** | `cortex://threads/{bot.id}` | `cortex://threads/{agent_id}` |
| **用户可见** | "你的 Bot ID: xxx" | "(Cortex Memory Agent ID)" |
| **代码层面** | bot_id (变量名) | agent_id (变量名) |

---

## 📊 修改统计

| 文件 | 修改类型 | 数量 |
|------|---------|------|
| cortex-mem-rig/src/lib.rs | 字段名、函数名、注释 | ~10 处 |
| cortex-mem-rig/src/tools/mod.rs | 字段名、参数名、变量名 | ~27 处 |
| examples/cortex-mem-tars/src/agent.rs | 导入、调用、注释 | ~8 处 |
| **总计** | | **~45 处** |

---

## ✅ 编译验证

```bash
$ cargo build -p cortex-mem-tars
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.57s
```

✅ **编译成功，无错误**

---

## 🎯 设计原则

### 1. 框架层面使用统一术语
- Cortex Memory 框架统一使用 **agent_id**
- 这是框架的核心概念，不应该被应用层术语污染

### 2. 应用层保留业务术语
- TARS 应用层可以继续使用 "Bot" 的业务概念
- 在 System Prompt 中对用户说明 "Bot ID"
- 但在传递给框架时，映射为 agent_id

### 3. 文档中明确映射关系
- 在用户可见的地方说明映射关系
- 例如："Bot ID: xxx (Cortex Memory Agent ID)"
- 帮助用户理解技术实现

---

## 📚 后续影响

### 代码层面
- ✅ 所有框架代码使用 agent_id
- ✅ 工具链正确传递 agent_id
- ✅ 记忆隔离基于 agent_id

### 文档层面
- ⚠️ 需要更新相关文档中的术语
- ⚠️ API 文档中统一使用 agent_id
- ⚠️ 示例代码中使用正确的术语

### 用户体验
- ✅ TARS 用户仍然看到 "Bot" 概念（业务层）
- ✅ 框架使用者看到 "Agent" 概念（框架层）
- ✅ 两者通过注释和文档建立清晰的映射关系

---

## 🔍 相关概念澄清

### Cortex Memory 中的核心概念

```
cortex://
  ├── agents/           # Agent 维度（为 agent 设计的记忆空间）
  ├── users/            # User 维度（为用户设计的记忆空间）
  ├── threads/          # Thread 维度（对话线程）
  └── global/           # Global 维度（全局共享）
```

**当前设计**：
- 使用 `cortex://threads/{agent_id}` 作为每个 agent 的独立空间
- 原因：底层 SessionManager 硬编码使用 threads
- 效果：每个 agent_id 对应一个独立的 thread

**未来优化**：
- 可以使用 `cortex://agents/{agent_id}` 作为 agent 的专属空间
- 需要重构 SessionManager 支持自定义 dimension
- 这样语义更清晰，符合 Cortex Memory 的设计理念

---

## 🎊 总结

### 修改前
- ❌ 框架层使用 bot_id（应用层术语）
- ❌ 概念混淆
- ❌ 不符合框架设计理念

### 修改后
- ✅ 框架层统一使用 agent_id
- ✅ 概念清晰
- ✅ 应用层和框架层职责分明
- ✅ 符合软件工程最佳实践

---

**修改时间**: 2026-02-09 15:00  
**修改者**: AI Assistant  
**影响范围**: cortex-mem-rig, cortex-mem-tars  
**编译状态**: ✅ 通过  
**破坏性变更**: 是（API 函数名变更）

**迁移指南**：
- 将所有 `create_memory_tools_with_bot_id` 改为 `create_memory_tools_with_agent_id`
- 将所有 `MemoryTools::with_bot_id` 改为 `MemoryTools::with_agent_id`
- 概念上：bot_id → agent_id
