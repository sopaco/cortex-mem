# 🔧 Bot 记忆隔离路径修复报告

## 📋 问题描述

用户报告了两个问题：

### 问题1：agents 文件夹为空
- 位置：`/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars`
- 现象：threads 文件夹有记忆文件，但 agents 文件夹是空的
- 预期：agents 文件夹应该有各个 bot 的记忆

### 问题2：Agent 调用记忆失败
- 截图显示：TARS AI 说"记忆存储系统似乎无法访问我的专属记忆空间"
- 原因：搜索和存储的路径不一致

---

## 🔍 根本原因分析

### 问题根源：路径不一致

在之前的修复中，我设计了这样的隔离机制：

```rust
// SearchTool/FindTool - 搜索路径
scope = "cortex://agents/{bot_id}"

// StoreTool - 存储路径
thread_id = "{bot_id}"
// 但实际存储到：cortex://threads/{bot_id}
```

**结果**：
- ✅ 存储成功：数据写入 `cortex://threads/{bot_id}`
- ❌ 搜索失败：在 `cortex://agents/{bot_id}` 搜索（空的）
- ❌ 记忆隔离失效：不同 bot 存储在同一个 dimension

### 架构设计的问题

**SessionManager 的硬编码路径**：
```rust
// session/manager.rs:174
let metadata_uri = format!("cortex://threads/{}/.session.json", thread_id);
```

**MessageStorage 的硬编码路径**：
```rust
// session/message.rs
// 所有消息都存储到 cortex://threads/{thread_id}
```

这些底层模块都硬编码了 `cortex://threads` dimension，无法灵活切换到其他 dimension。

---

## ✅ 解决方案

### 方案选择

**原计划**：使用 `cortex://agents/{bot_id}` 作为隔离空间
**修正方案**：使用 `cortex://threads/{bot_id}` 作为隔离空间

**理由**：
1. 底层 SessionManager 和 MessageStorage 都使用 `cortex://threads`
2. 修改底层模块代价大，风险高
3. `threads` dimension 本来就是为对话线程设计的
4. 每个 bot_id 作为独立的 thread_id，天然实现隔离

### 实现方案

**统一使用 `cortex://threads/{bot_id}`**：

```
cortex://threads/
  ├── {bot_id_1}/          # Bot 1 的专属记忆空间
  │   ├── .session.json
  │   ├── timeline/
  │   │   └── messages/
  │   └── ...
  ├── {bot_id_2}/          # Bot 2 的专属记忆空间
  │   ├── .session.json
  │   ├── timeline/
  │   │   └── messages/
  │   └── ...
  └── ...
```

---

## 🛠️ 具体修改

### 1. 修改 SearchTool

**文件**：`cortex-mem-rig/src/tools/mod.rs`

**变更**：
```rust
// Before
if args.scope.is_none() && self.bot_id.is_some() {
    args.scope = Some(format!("cortex://agents/{}", self.bot_id.as_ref().unwrap()));
}

// After
if args.scope.is_none() && self.bot_id.is_some() {
    args.scope = Some(format!("cortex://threads/{}", self.bot_id.as_ref().unwrap()));
}
```

### 2. 修改 FindTool

**文件**：`cortex-mem-rig/src/tools/mod.rs`

**变更**：
```rust
// Before
if args.scope.is_none() && self.bot_id.is_some() {
    args.scope = Some(format!("cortex://agents/{}", self.bot_id.as_ref().unwrap()));
}

// After
if args.scope.is_none() && self.bot_id.is_some() {
    args.scope = Some(format!("cortex://threads/{}", self.bot_id.as_ref().unwrap()));
}
```

### 3. 更新 System Prompt

**文件**：`examples/cortex-mem-tars/src/agent.rs`

**变更**：
```rust
// Before
记忆隔离说明：
- 每个 Bot 拥有独立的记忆空间（cortex://agents/{bot_id}）
- 你的记忆不会与其他 Bot 共享
- 所有搜索和存储默认在你的专属空间内进行

// After
记忆隔离说明：
- 每个 Bot 拥有独立的记忆空间（cortex://threads/{bot_id}）
- 你的记忆不会与其他 Bot 共享
- 所有搜索和存储默认在你的专属空间内进行
```

**详细的 prompt 更新**：
```diff
- scope: 搜索范围（默认为你的专属记忆空间 cortex://agents/{bot_id}）
+ scope: 搜索范围（默认为你的专属记忆空间 cortex://threads/{bot_id}）
```

---

## 📊 修改后的工作流程

### 1. Store 存储流程

```
用户输入: "记住我喜欢咖啡"
↓
StoreTool.call(content="记住我喜欢咖啡", thread_id=bot_id)
↓
MemoryOperations.store()
↓
SessionManager.create_session(thread_id=bot_id)
  - 创建 cortex://threads/{bot_id}/.session.json
↓
MessageStorage.save_message(thread_id=bot_id, message)
  - 存储到 cortex://threads/{bot_id}/timeline/messages/{timestamp}.md
↓
LayerManager.generate_all_layers()
  - 生成 L0: cortex://threads/{bot_id}/timeline/messages/{timestamp}.L0.md
  - 生成 L1: cortex://threads/{bot_id}/timeline/messages/{timestamp}.L1.md
```

### 2. Search 搜索流程

```
用户查询: "我喜欢什么？"
↓
SearchTool.call(query="喜好", scope=None, bot_id=bot_id)
↓
自动注入: scope = "cortex://threads/{bot_id}"
↓
MemoryOperations.search(scope="cortex://threads/{bot_id}")
↓
RetrievalEngine.search() - 在 cortex://threads/{bot_id} 下递归搜索
↓
返回结果: 找到 "我喜欢咖啡" 的 L0 摘要
```

### 3. Bot 隔离效果

**Bot A (bot_id = "93136eaf-3ac3-4cc0-8f45-28a7a28a8e66")**:
- 存储：`cortex://threads/93136eaf-3ac3-4cc0-8f45-28a7a28a8e66/...`
- 搜索：`cortex://threads/93136eaf-3ac3-4cc0-8f45-28a7a28a8e66/...`
- ✅ 一致！

**Bot B (bot_id = "另一个UUID")**:
- 存储：`cortex://threads/{另一个UUID}/...`
- 搜索：`cortex://threads/{另一个UUID}/...`
- ✅ 完全隔离！

---

## 🎯 预期行为

### 测试场景1：Bot A 存储记忆

```bash
# Bot A (ID: 93136eaf-3ac3-4cc0-8f45-28a7a28a8e66)
用户: "记住我喜欢咖啡"
Agent: 调用 store(content="用户喜欢咖啡")
```

**预期结果**：
```
/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/
└── threads/
    └── 93136eaf-3ac3-4cc0-8f45-28a7a28a8e66/
        ├── .session.json
        └── timeline/
            └── messages/
                ├── 2026-02-09_14-18-59_user.md
                ├── 2026-02-09_14-18-59_user.L0.md
                └── 2026-02-09_14-18-59_user.L1.md
```

### 测试场景2：Bot A 搜索记忆

```bash
用户: "我喜欢什么？"
Agent: 调用 search(query="喜好")
```

**预期结果**：
- ✅ 搜索范围自动设置为 `cortex://threads/93136eaf-3ac3-4cc0-8f45-28a7a28a8e66`
- ✅ 找到之前存储的"用户喜欢咖啡"
- ✅ 返回 L0 摘要

### 测试场景3：Bot B 搜索记忆

```bash
# Bot B (ID: 另一个UUID)
用户: "我喜欢什么？"
Agent: 调用 search(query="喜好")
```

**预期结果**：
- ✅ 搜索范围自动设置为 `cortex://threads/{另一个UUID}`
- ✅ 找不到 Bot A 的记忆
- ✅ 隔离生效

---

## 🔍 文件系统布局

### 修复前（问题状态）

```
/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/
├── threads/
│   └── 93136eaf-3ac3-4cc0-8f45-28a7a28a8e66/
│       └── timeline/
│           └── messages/
│               └── 2026-02-09_14-18-59_user.md  ✅ 有数据
└── agents/
    └── (空)  ❌ 搜索这里，找不到
```

### 修复后（预期状态）

```
/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/
└── threads/
    ├── 93136eaf-3ac3-4cc0-8f45-28a7a28a8e66/  (Bot A)
    │   ├── .session.json
    │   └── timeline/
    │       └── messages/
    │           ├── 2026-02-09_14-18-59_user.md
    │           ├── 2026-02-09_14-18-59_user.L0.md
    │           └── 2026-02-09_14-18-59_user.L1.md
    └── {另一个UUID}/  (Bot B)
        ├── .session.json
        └── timeline/
            └── messages/
                └── ...
```

---

## ✅ 验证步骤

### 1. 编译验证

```bash
$ cargo build -p cortex-mem-tars
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.98s
```

✅ **编译成功，无错误**

### 2. 运行验证

**步骤1**：启动 TARS
```bash
cd examples/cortex-mem-tars
cargo run
```

**步骤2**：创建或选择 Bot
- 记录 Bot ID

**步骤3**：存储记忆
```
用户: "记住我喜欢喝咖啡"
```

**步骤4**：验证存储
```bash
ls -la "/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/threads/{bot_id}/timeline/messages/"
```

**预期**：看到新创建的消息文件和 L0/L1 层

**步骤5**：搜索记忆
```
用户: "我喜欢什么？"
```

**预期**：Agent 能够找到并返回之前存储的记忆

**步骤6**：测试隔离
- 切换到另一个 Bot
- 搜索相同的内容
- 确认找不到第一个 Bot 的记忆

---

## 📝 与之前设计的对比

| 方面 | 原设计（agents） | 修正方案（threads） |
|------|-----------------|---------------------|
| **存储路径** | `cortex://threads/{bot_id}` | `cortex://threads/{bot_id}` |
| **搜索路径** | `cortex://agents/{bot_id}` ❌ | `cortex://threads/{bot_id}` ✅ |
| **路径一致性** | ❌ 不一致 | ✅ 一致 |
| **底层支持** | ❌ 需要修改底层模块 | ✅ 无需修改 |
| **语义清晰度** | ⭐⭐⭐⭐☆ | ⭐⭐⭐⭐⭐ |
| **实现复杂度** | 高（需要修改多个模块） | 低（只修改工具层） |
| **隔离效果** | ✅ （如果实现正确） | ✅ 相同 |

---

## 🎊 优势总结

### 1. 路径一致性
- ✅ 存储和搜索都在 `cortex://threads/{bot_id}`
- ✅ 不会出现"存了找不到"的问题

### 2. 语义合理性
- ✅ `threads` dimension 本来就是为对话线程设计的
- ✅ 每个 bot 作为一个独立的 thread，语义清晰
- ✅ 符合 Cortex Memory 的原始设计意图

### 3. 实现简洁性
- ✅ 无需修改底层 SessionManager/MessageStorage
- ✅ 只需修改工具层的 scope 注入逻辑
- ✅ 风险低，改动小

### 4. 向后兼容
- ✅ 现有数据已经在 `cortex://threads/{bot_id}`
- ✅ 无需数据迁移
- ✅ 立即生效

---

## 🔧 后续优化建议

### 短期（可选）
1. 添加 `agents` dimension 支持（如果确实需要）
2. 提供数据迁移工具（threads ↔ agents）

### 中期（建议）
1. 重构 SessionManager，支持自定义 dimension
2. 抽象路径生成逻辑到统一的 URIBuilder
3. 添加集成测试验证隔离效果

### 长期（高级功能）
1. 支持 bot 间的记忆共享机制
2. 实现跨 dimension 的搜索
3. 添加记忆访问权限控制

---

## 📚 文档更新

需要更新以下文档：

1. **BOT_MEMORY_ISOLATION_FIX.md**
   - 修正 scope 路径从 `cortex://agents/{bot_id}` 到 `cortex://threads/{bot_id}`
   - 更新 Bot 隔离机制说明

2. **PROJECT_EVALUATION_REPORT.md**
   - 修正 cortex-mem-rig 工具的 scope 说明
   - 更新 Bot 隔离目录结构示例

3. **README.md** (如果有提到)
   - 确保示例代码正确

---

**修改时间**: 2026-02-09 14:35  
**修改作者**: AI Assistant  
**影响文件**: 
- cortex-mem-rig/src/tools/mod.rs (SearchTool, FindTool)
- examples/cortex-mem-tars/src/agent.rs (System Prompt)

**编译状态**: ✅ 通过  
**测试状态**: ⏳ 待用户验证

---

## 🎯 用户操作指南

### 重启 TARS 并测试

1. **停止当前运行的 TARS**（如果在运行）

2. **重新编译**
   ```bash
   cd /Users/jiangmeng/workspace/SAW/cortex-mem
   cargo build -p cortex-mem-tars
   ```

3. **启动 TARS**
   ```bash
   cd examples/cortex-mem-tars
   cargo run
   ```

4. **测试记忆存储**
   ```
   用户: "记住我喜欢喝咖啡"
   ```
   
   **预期**: TARS 成功存储记忆

5. **测试记忆搜索**
   ```
   用户: "我喜欢什么？"
   ```
   
   **预期**: TARS 能够找到并回复"你喜欢咖啡"

6. **验证文件系统**
   ```bash
   ls -la "/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/threads/"
   ```
   
   **预期**: 看到你的 Bot ID 对应的文件夹，里面有记忆文件

### 清理旧数据（可选）

如果你想从零开始测试：

```bash
rm -rf "/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/threads/"
```

然后重新启动 TARS，重新创建 Bot 和记忆。

---

**问题已修复！现在存储和搜索路径一致，Bot 记忆隔离应该正常工作。**
