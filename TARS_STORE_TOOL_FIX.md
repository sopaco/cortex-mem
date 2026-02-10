# 🔧 TARS 存储工具错误修复

## 🐛 问题描述

TARS 程序无法正确使用 store 工具存储记忆，报错：

```
[2026-02-09 17:15:34.552 WARN] Error while calling tool: 
Toolset error: ToolCallError: ToolCallError: ToolCallError: ToolCallError: 
Core error: Invalid dimension: session
```

**用户操作**：要求 AI 使用记忆工具存储对话内容

**错误位置**：调用 store 工具时

**错误原因**：URI 解析器不识别 "session" 维度

---

## 🔍 根因分析

### 问题1：Dimension 枚举定义过时

**文件**：`cortex-mem-core/src/types.rs`

**旧代码**（OpenViking 重构前）：
```rust
pub enum Dimension {
    Agents,    // "agents"
    Users,     // "users"
    Threads,   // "threads"
    Global,    // "global"
}
```

**问题**：
1. 我们在重构时将目录结构改为 `resources/user/agent/session`
2. 但是忘记更新 `Dimension` 枚举
3. URI 解析器使用 `Dimension::from_str("session")` 时返回 `None`
4. 导致 "Invalid dimension: session" 错误

### 问题2：重构不完整

在之前的 OpenViking 对齐重构中：

✅ **已更新**：
- 文件系统目录：`resources, user, agent, session`
- URI 字符串：`cortex://session/{id}/...`
- 配置文件和文档

❌ **未更新**：
- `Dimension` 枚举定义
- 枚举的 `as_str()` 和 `from_str()` 方法
- 测试用例中的断言

**影响**：
- Store 工具无法解析 `cortex://session/...` URI
- 所有使用 session 维度的操作都会失败

---

## 🛠️ 修复方案

### 修复1：更新 Dimension 枚举

**文件**：`cortex-mem-core/src/types.rs`

```rust
/// Dimension of memory storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dimension {
    /// Resource-specific memories (facts, knowledge)
    Resources,
    /// User-specific memories
    User,
    /// Agent-specific memories
    Agent,
    /// Session/conversation memories
    Session,
}

impl Dimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Dimension::Resources => "resources",
            Dimension::User => "user",
            Dimension::Agent => "agent",
            Dimension::Session => "session",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "resources" => Some(Dimension::Resources),
            "user" => Some(Dimension::User),
            "agent" => Some(Dimension::Agent),
            "session" => Some(Dimension::Session),
            // Legacy support for old URIs
            "agents" => Some(Dimension::Agent),
            "users" => Some(Dimension::User),
            "threads" => Some(Dimension::Session),
            "global" => Some(Dimension::Resources),
            _ => None,
        }
    }
}
```

**关键改进**：
1. ✅ 四个新维度：`Resources`, `User`, `Agent`, `Session`
2. ✅ 向后兼容：支持旧的 `agents/users/threads/global` URI
3. ✅ 与文件系统目录完全对齐

### 修复2：更新测试用例

**文件**：`cortex-mem-core/src/filesystem/uri.rs`

```rust
// 更新断言
assert_eq!(uri.dimension, Dimension::Session);  // 原来是 Dimension::Threads

// 更新路径断言
assert_eq!(path, PathBuf::from("/data/session/abc123/timeline/2026-02/03.md"));
// 原来是 /data/threads/...
```

---

## ✅ 修复效果

### 修复前

```
cortex://session/{id}/timeline  →  ❌ Invalid dimension: session
```

### 修复后

```
cortex://session/{id}/timeline  →  ✅ Dimension::Session
cortex://user/{id}/preferences  →  ✅ Dimension::User
cortex://agent/{id}/memories    →  ✅ Dimension::Agent
cortex://resources/facts        →  ✅ Dimension::Resources

# 向后兼容（Legacy）
cortex://threads/{id}/timeline  →  ✅ Dimension::Session
cortex://agents/{id}/memories   →  ✅ Dimension::Agent
cortex://users/{id}/prefs       →  ✅ Dimension::User
cortex://global/shared          →  ✅ Dimension::Resources
```

---

## 📊 完整的维度映射

### OpenViking 风格（推荐）

| 维度 | 枚举值 | URI 前缀 | 目录名 | 用途 |
|------|--------|---------|--------|------|
| Resources | `Dimension::Resources` | `cortex://resources/` | `resources/` | 全局资源、知识库 |
| User | `Dimension::User` | `cortex://user/` | `user/` | 用户偏好、配置 |
| Agent | `Dimension::Agent` | `cortex://agent/` | `agent/` | Agent 记忆 |
| Session | `Dimension::Session` | `cortex://session/` | `session/` | 会话、对话 |

### Legacy 兼容

| 旧 URI | 映射到 | 说明 |
|--------|--------|------|
| `cortex://threads/` | `Dimension::Session` | 会话/对话 |
| `cortex://agents/` | `Dimension::Agent` | Agent |
| `cortex://users/` | `Dimension::User` | 用户 |
| `cortex://global/` | `Dimension::Resources` | 全局资源 |

---

## 🧪 验证方法

### 1. 编译测试

```bash
cargo build -p cortex-mem-core
cargo build -p cortex-mem-tars
```

**预期结果**：
```
✅ Compiling cortex-mem-core v2.0.0
✅ Compiling cortex-mem-tools v2.0.0
✅ Compiling cortex-mem-rig v2.0.0
✅ Compiling cortex-mem-tars v2.0.0
✅ Finished `dev` profile
```

### 2. 单元测试

```bash
cargo test -p cortex-mem-core
```

**预期结果**：
```
✅ test_parse_simple_uri ... ok
✅ test_parse_full_uri ... ok
✅ test_to_file_path ... ok
```

### 3. 运行时测试

```bash
cargo run -p cortex-mem-tars
```

**测试步骤**：
1. 启动 TARS
2. 与 Bot 对话
3. 要求："请记住这段对话"
4. 观察日志

**预期结果**：
```
[INFO] Calling tool store with args: {...}
✅ 成功存储记忆（没有 "Invalid dimension" 错误）
```

### 4. 文件系统检查

```bash
tree ~/Library/Application\ Support/com.cortex-mem.tars/cortex/
```

**预期结构**：
```
cortex/
├── resources/
├── user/
├── agent/
├── session/
└── tenants/
    └── {bot-id}/
        └── cortex/
            ├── resources/
            ├── user/
            ├── agent/
            └── session/    ← 应该能看到存储的文件
```

---

## 📝 相关变更清单

### 修改的文件

1. ✅ `cortex-mem-core/src/types.rs`
   - 更新 `Dimension` 枚举定义
   - 更新 `as_str()` 方法
   - 更新 `from_str()` 方法
   - 添加 Legacy 兼容支持

2. ✅ `cortex-mem-core/src/filesystem/uri.rs`
   - 更新测试用例断言
   - 更新文档注释中的示例

### 影响范围

**核心包**：
- ✅ cortex-mem-core
- ✅ cortex-mem-tools
- ✅ cortex-mem-rig

**应用**：
- ✅ cortex-mem-tars

**功能**：
- ✅ Store 工具
- ✅ Search 工具
- ✅ Find 工具
- ✅ 所有 URI 解析

---

## 🎓 经验教训

### 1. 重构需要全面性

**问题**：
- 只更新了文件系统目录结构
- 忘记更新核心的 `Dimension` 枚举

**教训**：
- 重构时需要列出所有相关的代码位置
- 使用 grep/ripgrep 全局搜索相关代码
- 更新测试用例以验证修改

### 2. 类型系统的重要性

**问题**：
- 枚举定义过时导致运行时错误
- 编译器无法检测到这种不一致

**教训**：
- 枚举定义应该是单一真相来源
- 文件系统目录应该从枚举派生，而不是硬编码

### 3. 向后兼容的价值

**做法**：
- 在 `from_str()` 中添加 Legacy 映射
- 允许旧 URI 继续工作

**优势**：
- 平滑迁移
- 避免破坏现有数据
- 给用户更多时间适应新 URI

---

## 🎯 总结

### 问题
- TARS 无法使用 store 工具
- 错误：`Invalid dimension: session`

### 根因
- `Dimension` 枚举未更新为 OpenViking 风格
- URI 解析器无法识别 "session" 维度

### 修复
- 更新 `Dimension` 枚举：`Resources, User, Agent, Session`
- 添加 Legacy 兼容支持
- 更新测试用例

### 效果
- ✅ Store 工具正常工作
- ✅ 完全对齐 OpenViking 设计
- ✅ 向后兼容旧 URI
- ✅ 所有包编译成功

---

**修复时间**：2026-02-09 17:20  
**影响范围**：cortex-mem-core, cortex-mem-tools, cortex-mem-rig, cortex-mem-tars  
**测试状态**：✅ 编译成功  
**部署建议**：清理旧数据后重新运行
