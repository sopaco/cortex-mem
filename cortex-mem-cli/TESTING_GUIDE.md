# Cortex-Mem CLI 测试指南

## 📋 目录
1. [快速开始](#快速开始)
2. [配置说明](#配置说明)
3. [基础命令测试](#基础命令测试)
4. [完整工作流演示](#完整工作流演示)
5. [常见问题](#常见问题)

---

## 🚀 快速开始

### 1. 构建CLI

```bash
# 进入项目目录
cd cortex-mem

# 构建CLI工具
cargo build --release --bin cortex-mem

# 或者直接运行（开发模式）
cargo run --bin cortex-mem -- --help
```

### 2. 查看帮助信息

```bash
# 查看主帮助
cargo run --bin cortex-mem -- --help

# 查看特定命令帮助
cargo run --bin cortex-mem -- add --help
cargo run --bin cortex-mem -- search --help
cargo run --bin cortex-mem -- session --help
```

---

## ⚙️ 配置说明

### 数据目录配置

CLI支持自定义数据存储位置：

```bash
# 默认数据目录（当前目录下的 cortex-data）
cargo run --bin cortex-mem -- stats

# 自定义数据目录
cargo run --bin cortex-mem -- --data-dir /path/to/your/data stats

# 使用环境变量（可选）
export CORTEX_DATA_DIR="/path/to/your/data"
```

### 日志配置

```bash
# 启用详细日志
cargo run --bin cortex-mem -- --verbose add --thread test "Hello"

# 或使用环境变量
export RUST_LOG=debug
cargo run --bin cortex-mem -- stats
```

---

## 🧪 基础命令测试

### Test 1: 查看统计信息

```bash
# 查看初始状态（会自动初始化文件系统）
cargo run --bin cortex-mem -- stats
```

**预期输出**:
```
📊 Cortex-Mem Statistics

📁 Dimensions:
  Threads: 0
  Agents: 0
  Users: 0
  Global: 0

📝 Content:
  Messages: ~0

💾 Storage:
  Data directory: ./cortex-data
```

---

### Test 2: 创建会话

```bash
# 创建一个新会话
cargo run --bin cortex-mem -- session create my-first-session --title "测试会话"
```

**预期输出**:
```
📝 Creating session: my-first-session
  Title: 测试会话
✓ Session created successfully
  Thread ID: my-first-session
  Status: Active
  Created: 2026-02-03 XX:XX:XX UTC
```

---

### Test 3: 添加消息

```bash
# 添加用户消息
cargo run --bin cortex-mem -- add --thread my-first-session "你好，这是我的第一条消息"

# 添加助手回复
cargo run --bin cortex-mem -- add --thread my-first-session --role assistant "你好！很高兴为你服务。"

# 添加系统消息
cargo run --bin cortex-mem -- add --thread my-first-session --role system "会话已开始"
```

**预期输出**（每次）:
```
✓ Message added successfully
  Thread: my-first-session
  Role: User
  URI: cortex://threads/my-first-session/timeline/2026-02/03/XX_XX_XX_xxxxxxxx.md
  ID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

---

### Test 4: 列出内容

```bash
# 列出会话根目录
cargo run --bin cortex-mem -- list --thread my-first-session

# 列出时间轴目录
cargo run --bin cortex-mem -- list --thread my-first-session/timeline

# 列出所有线程
cargo run --bin cortex-mem -- list --dimension threads
```

**预期输出**（示例）:
```
📋 Listing memories from: cortex://threads/my-first-session

✓ Found 2 items:

📁 Directories (1):
  • timeline/

📄 Files (1):
  • .session.json
    xxx bytes
```

---

### Test 5: 查看具体消息

```bash
# 获取消息URI（从add命令输出复制）
cargo run --bin cortex-mem -- get "cortex://threads/my-first-session/timeline/2026-02/03/XX_XX_XX_xxxxxxxx.md"
```

**预期输出**:
```
🔍 Getting memory: cortex://threads/...

────────────────────────────────────────────────────────────────────────────────
# 👤 User

**ID**: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`  
**Timestamp**: 2026-02-03 XX:XX:XX UTC

## Content

你好，这是我的第一条消息
────────────────────────────────────────────────────────────────────────────────

ℹ Metadata:
  Size: xxx bytes
```

---

### Test 6: 搜索消息

```bash
# 搜索包含特定关键词的消息
cargo run --bin cortex-mem -- search "第一条" --thread my-first-session

# 全局搜索
cargo run --bin cortex-mem -- search "消息"

# 限制结果数量和最小分数
cargo run --bin cortex-mem -- search "消息" -n 3 -s 0.5
```

**预期输出**:
```
🔍 Searching for: 第一条
  Scope: my-first-session

✓ Found 1 results:

1. cortex://threads/my-first-session/timeline/.../XX_XX_XX_xxxxxxxx.md (score: 0.85)
   你好，这是我的第一条消息

📊 Retrieval trace:
  • IntentAnalysis: 2 candidates (Xms)
  • L0Scan: 1 candidates (Xms)
  • L1Exploration: 1 candidates (Xms)
  • ResultAggregation: 1 candidates (Xms)
  Total: XXms
```

---

### Test 7: 会话管理

```bash
# 列出所有会话
cargo run --bin cortex-mem -- session list

# 提取记忆（注意：需要LLM配置，当前使用placeholder）
cargo run --bin cortex-mem -- session extract my-first-session

# 关闭会话
cargo run --bin cortex-mem -- session close my-first-session
```

**session list 预期输出**:
```
📋 Listing all sessions

✓ Found 1 sessions:

• my-first-session
  Status: Active
  Messages: 3
  Title: 测试会话
```

**session extract 预期输出**:
```
🧠 Extracting memories from session: my-first-session
✓ Extraction complete
  Facts: 0
  Decisions: 0
  Entities: 0
  Total: 0
  Saved to: cortex://threads/my-first-session/extractions/YYYYMMDD_HHMMSS.md

注意：当前使用placeholder LLM实现，实际提取为空
```

**session close 预期输出**:
```
🔒 Closing session: my-first-session
✓ Session closed successfully
  Thread ID: my-first-session
  Status: Closed
  Closed: 2026-02-03 XX:XX:XX UTC
  Messages: 3
```

---

### Test 8: 删除消息

```bash
# 删除特定消息（使用get命令获取的URI）
cargo run --bin cortex-mem -- delete "cortex://threads/my-first-session/timeline/2026-02/03/XX_XX_XX_xxxxxxxx.md"
```

**预期输出**:
```
🗑️ Deleting memory: cortex://threads/...
✓ Memory deleted successfully
```

---

## 🎯 完整工作流演示

这是一个完整的对话记录和记忆管理工作流：

```bash
# Step 1: 创建技术讨论会话
cargo run --bin cortex-mem -- session create tech-discussion --title "OAuth实现讨论"

# Step 2: 记录对话
cargo run --bin cortex-mem -- add --thread tech-discussion \
  "我们需要为新项目实现OAuth 2.0认证，有什么建议吗？"

cargo run --bin cortex-mem -- add --thread tech-discussion --role assistant \
  "建议使用标准的OAuth 2.0授权码流程，配合JWT令牌。这是最安全的方式。"

cargo run --bin cortex-mem -- add --thread tech-discussion \
  "refresh token应该如何处理？"

cargo run --bin cortex-mem -- add --thread tech-discussion --role assistant \
  "实现refresh token轮换机制，每次使用后自动更新。设置合理的过期时间，比如7天。"

cargo run --bin cortex-mem -- add --thread tech-discussion \
  "好的，决定使用PostgreSQL存储token。"

# Step 3: 搜索相关讨论
cargo run --bin cortex-mem -- search "OAuth token" --thread tech-discussion

# Step 4: 查看会话内容
cargo run --bin cortex-mem -- list --thread tech-discussion

# Step 5: 提取关键决策和事实
cargo run --bin cortex-mem -- session extract tech-discussion

# Step 6: 查看统计
cargo run --bin cortex-mem -- stats

# Step 7: 关闭会话
cargo run --bin cortex-mem -- session close tech-discussion

# Step 8: 验证会话列表
cargo run --bin cortex-mem -- session list
```

---

## 📂 文件系统结构

执行上述测试后，数据目录结构如下：

```
cortex-data/
├── threads/
│   └── my-first-session/
│       ├── .session.json                    # 会话元数据
│       ├── timeline/
│       │   └── 2026-02/
│       │       └── 03/
│       │           ├── 14_30_45_abc123.md   # 消息1
│       │           ├── 14_31_02_def456.md   # 消息2
│       │           └── 14_31_15_ghi789.md   # 消息3
│       └── extractions/
│           └── 20260203_143200.md           # 提取的记忆
├── agents/      # Agent维度（暂未使用）
├── users/       # User维度（暂未使用）
└── global/      # 全局维度（暂未使用）
```

---

## 🔍 高级测试

### 测试多会话场景

```bash
# 创建多个会话
cargo run --bin cortex-mem -- session create project-a --title "项目A讨论"
cargo run --bin cortex-mem -- session create project-b --title "项目B讨论"
cargo run --bin cortex-mem -- session create brainstorm --title "头脑风暴"

# 分别添加内容
cargo run --bin cortex-mem -- add --thread project-a "项目A使用React"
cargo run --bin cortex-mem -- add --thread project-b "项目B使用Vue"
cargo run --bin cortex-mem -- add --thread brainstorm "考虑使用微服务架构"

# 全局搜索
cargo run --bin cortex-mem -- search "项目"

# 查看所有会话
cargo run --bin cortex-mem -- session list

# 查看统计
cargo run --bin cortex-mem -- stats
```

---

## ❓ 常见问题

### Q1: 找不到cortex-mem命令

**A**: 使用cargo run方式运行，或者构建后使用：
```bash
cargo build --release --bin cortex-mem
./target/release/cortex-mem --help
```

### Q2: 数据存储在哪里？

**A**: 默认在当前目录的 `cortex-data/` 文件夹。可以通过 `--data-dir` 参数自定义。

### Q3: 如何清空所有数据？

**A**: 直接删除数据目录：
```bash
rm -rf cortex-data/
```

### Q4: 消息URI太长，怎么办？

**A**: 可以使用shell变量或文件保存：
```bash
# 保存URI
URI=$(cargo run --bin cortex-mem -- add --thread test "Hello" 2>&1 | grep "URI:" | cut -d' ' -f4)

# 使用URI
cargo run --bin cortex-mem -- get "$URI"
```

### Q5: 搜索没有结果？

**A**: 检查：
1. 消息是否已添加（使用list命令）
2. 搜索范围是否正确（--thread参数）
3. 最小分数是否太高（-s参数）

### Q6: 记忆提取为空？

**A**: 当前使用placeholder LLM实现。要启用真实提取：
1. 配置OpenAI API密钥
2. 修改 `cortex-mem-core/src/llm/client.rs` 实现真实LLM调用

---

## 🎨 输出颜色说明

CLI使用颜色和图标增强可读性：

- ✓ 绿色：操作成功
- ✗ 红色：错误信息
- 📋 蓝色：列表/信息
- ⚠️ 黄色：警告
- 📊 青色：统计/元数据

---

## 📝 快速参考

### 常用命令速查表

| 命令 | 用途 | 示例 |
|------|------|------|
| `stats` | 查看统计 | `cargo run --bin cortex-mem -- stats` |
| `session create` | 创建会话 | `cargo run --bin cortex-mem -- session create my-session` |
| `add` | 添加消息 | `cargo run --bin cortex-mem -- add --thread my-session "Hello"` |
| `list` | 列出内容 | `cargo run --bin cortex-mem -- list --thread my-session` |
| `search` | 搜索 | `cargo run --bin cortex-mem -- search "keyword"` |
| `get` | 查看消息 | `cargo run --bin cortex-mem -- get "cortex://..."` |
| `session extract` | 提取记忆 | `cargo run --bin cortex-mem -- session extract my-session` |
| `session close` | 关闭会话 | `cargo run --bin cortex-mem -- session close my-session` |
| `session list` | 会话列表 | `cargo run --bin cortex-mem -- session list` |
| `delete` | 删除 | `cargo run --bin cortex-mem -- delete "cortex://..."` |

---

## 🚀 下一步

1. **测试完成后**，可以查看生成的文件：
   ```bash
   tree cortex-data/
   cat cortex-data/threads/my-first-session/.session.json
   ```

2. **集成到你的工作流**：
   - 编写shell脚本自动化常用操作
   - 创建alias简化命令
   - 配合其他工具使用

3. **探索高级功能**：
   - 自定义数据目录
   - 配置LLM实现真实记忆提取
   - 集成到CI/CD流程

---

## 📚 相关文档

- [Phase 4 Part 1 实现报告](../cortex-mem-2-planning/phase4-part1-cli-report.md)
- [Phase 3 会话管理报告](../cortex-mem-2-planning/phase3-implementation-report.md)
- [Phase 2 检索系统报告](../cortex-mem-2-planning/phase2-implementation-report.md)

---

**Enjoy using Cortex-Mem CLI! 🎉**
