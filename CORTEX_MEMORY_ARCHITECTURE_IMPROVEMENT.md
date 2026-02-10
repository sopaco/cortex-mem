# 🔄 Cortex Memory 架构改进方案（基于 OpenViking 设计）

## 📋 分析：OpenViking vs Cortex Memory

### OpenViking 的设计理念

OpenViking 采用了**基于认知模式的三类上下文**设计：

```
viking://
├── resources/          # 资源 - 用户添加的知识
├── user/               # 用户 - 用户层面的数据
├── agent/              # Agent - Agent 层面的数据
└── session/{id}/       # 会话 - 会话层面的数据
```

**核心设计原则**：

1. **认知映射**：基于人类认知模式设计
   - **Resource**（资源）：外部知识，用户驱动，静态
   - **Memory**（记忆）：内部认知，Agent 驱动，动态
   - **Skill**（技能）：可调用能力，相对静态

2. **职责清晰**：
   - `resources/` - 用户添加的外部知识（文档、代码库等）
   - `user/` - 用户的个人信息和 Agent 学习到的用户记忆
   - `agent/` - Agent 的技能、指令和学习到的模式
   - `session/` - 特定会话的消息和上下文

3. **单例模式**：
   - `user/` 和 `agent/` 是**单例**，没有 ID 后缀
   - `session/{session_id}/` 才是多实例

### Cortex Memory 当前设计

```
cortex://
├── agents/{agent_id}/     # 多 Agent 设计
├── users/{user_id}/       # 多用户设计
├── threads/{thread_id}/   # 多线程设计
└── global/                # 全局共享
```

**问题**：
- ❌ 混淆了"维度"和"实例"
- ❌ 缺少 `resources` 维度
- ❌ `agents/{agent_id}` 既要表示 agent 维度，又要表示 agent 实例

---

## 🎯 核心设计差异

| 维度 | OpenViking | Cortex Memory | 问题 |
|------|-----------|--------------|------|
| **资源** | `resources/{project}/` | ❌ 缺失 | 无法存储用户添加的知识库 |
| **用户** | `user/` (单例) | `users/{user_id}/` | 混淆了维度和实例 |
| **Agent** | `agent/` (单例) | `agents/{agent_id}/` | 混淆了维度和实例 |
| **会话** | `session/{session_id}/` | `threads/{thread_id}/` | 语义不同 |
| **多实例** | Session 是多实例 | 所有都是多实例 | 设计理念不同 |

---

## 💡 合理的改进方案

### 方案A：完全对齐 OpenViking（推荐用于单用户单 Agent 场景）

**适用场景**：
- 单用户个人助手（如 TARS）
- 每个部署只服务一个用户和一个 Agent
- 不需要多租户支持

**设计**：

```
cortex://
├── resources/{project}/      # 用户添加的知识库
│   ├── rust-docs/
│   ├── project-a/
│   └── api-reference/
│
├── user/                     # 当前用户的数据（单例）
│   ├── profile.md
│   └── memories/
│       ├── preferences/      # 用户偏好
│       ├── entities/         # 实体记忆（人、项目）
│       └── events/           # 事件记录
│
├── agent/                    # 当前 Agent 的数据（单例）
│   ├── skills/               # 技能定义
│   ├── memories/
│   │   ├── cases/           # 学习的案例
│   │   └── patterns/        # 学习的模式
│   └── instructions/         # Agent 指令
│
└── session/{session_id}/     # 会话数据（多实例）
    ├── .abstract.md
    ├── .overview.md
    ├── messages.json
    └── timeline/
```

**优点**：
- ✅ 语义清晰，符合认知模型
- ✅ 单例模式简化了路径
- ✅ 分离了知识（resources）和记忆（memories）
- ✅ 完全对齐 OpenViking，便于学习和借鉴

**缺点**：
- ❌ 不支持多用户
- ❌ 不支持多 Agent
- ❌ 需要大幅重构现有代码

---

### 方案B：混合方案（推荐用于多用户多 Agent 场景）

**适用场景**：
- 多用户平台
- 多 Agent 系统
- 需要隔离和多租户支持

**设计**：

```
cortex://
├── resources/{project}/          # 全局资源（所有人共享）
│   ├── rust-docs/
│   └── api-reference/
│
├── users/{user_id}/              # 多用户支持
│   ├── profile.md
│   ├── resources/                # 用户私有资源
│   │   └── {project}/
│   └── memories/
│       ├── preferences/
│       ├── entities/
│       └── events/
│
├── agents/{agent_id}/            # 多 Agent 支持
│   ├── skills/
│   ├── memories/
│   │   ├── cases/
│   │   └── patterns/
│   └── instructions/
│
└── sessions/{session_id}/        # 会话（关联 user + agent）
    ├── .meta.json                # { user_id, agent_id, ... }
    ├── .abstract.md
    ├── .overview.md
    └── timeline/
```

**优点**：
- ✅ 支持多用户和多 Agent
- ✅ 保留了 resources 维度
- ✅ sessions 语义比 threads 更清晰
- ✅ 向后兼容（可以从当前 threads 迁移）

**缺点**：
- ⚠️ 仍然混淆了"维度"和"实例"
- ⚠️ 与 OpenViking 不完全一致

---

### 方案C：Cortex Memory 特色方案（推荐）

**核心思想**：
- 借鉴 OpenViking 的**三类上下文**（Resource、Memory、Skill）
- 保留 Cortex Memory 的**多实例支持**
- 引入**命名空间**概念，清晰区分"类型"和"实例"

**设计**：

```
cortex://
├── resources/                    # 资源维度（知识和规则）
│   ├── global/{project}/         # 全局共享资源
│   └── users/{user_id}/{project}/  # 用户私有资源
│
├── memories/                     # 记忆维度（Agent 的认知）
│   ├── users/{user_id}/          # 用户记忆
│   │   ├── profile/
│   │   ├── preferences/
│   │   ├── entities/
│   │   └── events/
│   └── agents/{agent_id}/        # Agent 记忆
│       ├── cases/
│       └── patterns/
│
├── skills/                       # 技能维度（可调用能力）
│   ├── global/{skill_name}/      # 全局技能
│   └── agents/{agent_id}/{skill_name}/  # Agent 私有技能
│
└── sessions/{session_id}/        # 会话维度（对话上下文）
    ├── .meta.json                # { user_id, agent_id, ... }
    ├── .abstract.md
    ├── .overview.md
    └── timeline/
```

**核心改进**：

1. **引入三类上下文**：
   - `resources/` - 知识和规则（对应 OpenViking 的 Resource）
   - `memories/` - Agent 的认知（对应 OpenViking 的 Memory）
   - `skills/` - 可调用能力（对应 OpenViking 的 Skill）

2. **命名空间隔离**：
   - 每个维度下再分 `global/`、`users/`、`agents/`
   - 清晰表达"谁拥有这个资源/记忆/技能"

3. **会话作为桥梁**：
   - `sessions/` 作为用户和 Agent 交互的场所
   - 在 `.meta.json` 中记录参与者

**优点**：
- ✅ 借鉴了 OpenViking 的认知模型
- ✅ 支持多用户和多 Agent
- ✅ 语义清晰，职责明确
- ✅ 保留了 Cortex Memory 的灵活性

**缺点**：
- ⚠️ 需要大幅重构
- ⚠️ 路径更长

---

## 🎯 推荐方案对比

| 场景 | 推荐方案 | 理由 |
|------|---------|------|
| **TARS（单用户单 Agent）** | 方案A | 最简单，完全对齐 OpenViking |
| **多 Agent 平台** | 方案C | 语义清晰，扩展性好 |
| **快速迁移** | 方案B | 改动最小，向后兼容 |

---

## 🔧 TARS 的具体改进建议（基于方案A）

### 当前 TARS 的问题

```
cortex://threads/611c2cdf-c70d-40df-a3f8-f4931b04f0b5/
└── timeline/
    └── 2026-02/09/
        └── 07_10_55_56bd7f97.md
```

**问题**：
- ❌ 使用 `threads/{agent_id}` 表示 agent 空间
- ❌ 语义混淆（thread 应该是对话，不是 agent）
- ❌ 所有内容都堆在 timeline 下

### 改进后的结构（方案A）

```
cortex://
├── resources/                    # 用户添加的知识库
│   ├── rust-docs/
│   └── cortex-mem-project/
│
├── user/                         # 用户数据（单例）
│   ├── profile.md
│   └── memories/
│       ├── preferences/
│       │   └── coding-style.md   # "我喜欢 Rust"
│       ├── entities/
│       │   └── SkyronJ.md        # 关于 SkyronJ 的记忆
│       └── events/
│           └── 2026-02-09-离职协商.md
│
├── agent/                        # Agent 数据（单例）
│   ├── skills/
│   │   ├── web-search/
│   │   └── code-analysis/
│   ├── memories/
│   │   ├── cases/
│   │   │   └── bug-fix-pattern-001.md
│   │   └── patterns/
│   │       └── rust-best-practice.md
│   └── instructions/
│       └── system-prompt.md
│
└── session/611c2cdf-c70d-40df-a3f8-f4931b04f0b5/
    ├── .abstract.md              # L0: 一句话摘要
    ├── .overview.md              # L1: 会话概览
    ├── .meta.json                # 会话元数据
    └── timeline/
        └── 2026-02/09/
            └── 07_10_55_56bd7f97.md
```

### 使用方式对比

**当前方式**（错误）：

```rust
// 所有内容都存到 threads/{agent_id}/timeline/
let uri = format!("cortex://threads/{}/timeline/...", agent_id);
```

**改进后方式**（正确）：

```rust
// 1. 用户告诉 Agent 关于 SkyronJ 的信息 → 存到 user/memories/entities/
let entity_uri = "cortex://user/memories/entities/SkyronJ.md";
filesystem.write(&entity_uri, &entity_content).await?;

// 2. Agent 学习到 Rust 最佳实践 → 存到 agent/memories/patterns/
let pattern_uri = "cortex://agent/memories/patterns/rust-best-practice.md";
filesystem.write(&pattern_uri, &pattern_content).await?;

// 3. 对话消息 → 存到 session/{session_id}/timeline/
let session_id = "611c2cdf-c70d-40df-a3f8-f4931b04f0b5";
let msg_uri = format!("cortex://session/{}/timeline/2026-02/09/15_10_55.md", session_id);
filesystem.write(&msg_uri, &message).await?;

// 4. 用户添加 Rust 文档 → 存到 resources/
let resource_uri = "cortex://resources/rust-docs/std-lib.md";
filesystem.write(&resource_uri, &docs_content).await?;
```

---

## 📊 迁移路径

### 短期（修复 TARS，保持兼容）

**目标**：不破坏现有数据，但改进语义

```rust
// 1. 添加别名机制
impl CortexFilesystem {
    pub async fn resolve_uri(&self, uri: &str) -> String {
        // threads/{agent_id} → session/{agent_id}
        if uri.starts_with("cortex://threads/") {
            uri.replace("cortex://threads/", "cortex://session/")
        } else {
            uri.to_string()
        }
    }
}

// 2. 文档中明确说明
// "注意：TARS 使用 session/{agent_id} 作为会话空间"
// "未来将迁移到完整的 resources/user/agent/session 架构"
```

### 中期（引入新维度，双模式运行）

**目标**：支持新架构，同时兼容旧数据

```rust
// 1. 引入新的存储维度
pub enum CortexDimension {
    Resources,  // cortex://resources/
    User,       // cortex://user/
    Agent,      // cortex://agent/
    Session,    // cortex://session/{session_id}/
    // Legacy
    Threads,    // cortex://threads/ (deprecated)
}

// 2. 提供迁移工具
pub async fn migrate_from_threads_to_session(
    filesystem: &CortexFilesystem,
    agent_id: &str,
    session_id: &str
) -> Result<()> {
    // 将 threads/{agent_id}/* 迁移到 session/{session_id}/*
}

// 3. 新功能使用新架构
// - 用户记忆 → cortex://user/memories/
// - Agent 技能 → cortex://agent/skills/
// - 资源 → cortex://resources/
```

### 长期（完全迁移到新架构）

**目标**：完全采用 OpenViking 风格的架构

```rust
// 1. 移除旧的 threads 维度
// 2. 所有代码使用新的 resources/user/agent/session 架构
// 3. 提供完整的迁移脚本
// 4. 更新所有文档
```

---

## 🎊 总结

### OpenViking 的核心启示

1. **认知映射**：
   - Resource（资源）- 用户添加的知识
   - Memory（记忆）- Agent 学习的认知
   - Skill（技能）- 可调用的能力

2. **单例模式**：
   - `user/` 和 `agent/` 是单例（针对当前上下文）
   - 只有 `session/{session_id}/` 是多实例

3. **语义清晰**：
   - 每个维度职责明确
   - 路径即语义

### Cortex Memory 应该怎么做

**立即行动**（TARS）：
1. ✅ 将 `threads/{agent_id}` 改为 `session/{agent_id}`
2. ✅ 在文档中说明这是会话空间，不是 agent 空间
3. ✅ 计划未来迁移到完整架构

**中期规划**：
1. ✅ 引入 `resources/` 维度（用户添加的知识库）
2. ✅ 引入 `user/memories/` 维度（用户记忆）
3. ✅ 引入 `agent/skills/` 维度（Agent 技能）
4. ✅ 提供迁移工具和双模式支持

**长期目标**：
1. ✅ 完全采用基于认知的三类上下文架构
2. ✅ 支持多用户和多 Agent（通过命名空间）
3. ✅ 提供完整的文档和示例

---

## 🔍 对于 TARS 的具体建议

### 最小改动方案（推荐）

**当前**：
```
cortex://threads/{agent_id}/timeline/...
```

**改为**：
```
cortex://session/{agent_id}/timeline/...
```

**理由**：
- ✅ 语义更准确（session 而非 thread）
- ✅ 改动最小（只需修改路径字符串）
- ✅ 为未来迁移做准备

### 代码修改

```rust
// cortex-mem-core/src/session/message.rs
pub async fn save_message(&self, session_id: &str, message: &Message) -> Result<String> {
    let uri = format!(
        "cortex://session/{}/timeline/{}/{}/{}",  // ✅ 改为 session
        session_id, year_month, day, filename
    );
    // ...
}

// cortex-mem-rig/src/tools/mod.rs
if args.scope.is_none() && self.agent_id.is_some() {
    args.scope = Some(format!("cortex://session/{}", self.agent_id.as_ref().unwrap()));  // ✅ 改为 session
}
```

**影响**：
- 需要重新初始化数据目录
- 或者提供数据迁移脚本

---

**方案创建时间**: 2026-02-09 15:50  
**作者**: AI Assistant  
**基于**: OpenViking 设计文档分析  
**推荐**: 方案A（TARS）/ 方案C（通用平台）
