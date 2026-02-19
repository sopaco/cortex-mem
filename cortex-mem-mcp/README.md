# Cortex Memory MCP Server

`cortex-mem-mcp` 是一个基于 [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) 的服务器，使 AI 助手能够与 Cortex Memory 系统进行交互，实现持久化记忆存储和检索。

## 🧠 功能概述

Cortex Memory MCP 服务器提供四个核心工具，让 AI 助手能够：

- 📝 **存储记忆**: 将对话中的关键信息保存到长期记忆
- 🔍 **查询记忆**: 通过多种搜索模式检索相关记忆
- 📋 **列出记忆**: 浏览已存储的记忆条目
- 📄 **获取记忆**: 读取特定记忆的完整内容

## 🛠️ MCP 工具

### 1. `store_memory`

存储新的记忆到 Cortex Memory 系统中。

#### 参数

| 参数 | 类型 | 必需 | 默认值 | 描述 |
|------|------|------|--------|------|
| `content` | string | ✅ | - | 要存储的记忆内容 |
| `thread_id` | string | ❌ | "default" | 会话ID，用于组织相关记忆 |
| `role` | string | ❌ | "user" | 消息角色: "user", "assistant", "system" |

#### 示例

```json
{
  "content": "用户偏好使用深色主题，并且喜欢使用 Vim 键位绑定",
  "thread_id": "user-preferences",
  "role": "user"
}
```

#### 响应

```json
{
  "success": true,
  "uri": "cortex://threads/user-preferences/timeline/2024/01/15/14_30_45_abc123.md",
  "message_id": "2024-01-15T14:30:45Z-abc123"
}
```

### 2. `query_memory`

智能搜索记忆，支持多种搜索模式和过滤条件。

#### 参数

| 参数 | 类型 | 必需 | 默认值 | 描述 |
|------|------|------|--------|------|
| `query` | string | ✅ | - | 搜索查询字符串 |
| `thread_id` | string | ❌ | - | 限制搜索到此会话 |
| `limit` | number | ❌ | 10 | 最大结果数量 |
| `scope` | string | ❌ | "session" | 搜索范围: "session", "user", "agent" |

#### 搜索范围说明

- **`session`**: 仅搜索会话记忆
- **`user`**: 搜索用户相关的记忆
- **`agent`**: 搜索 AI 助手的记忆

#### 示例

```json
{
  "query": "Rust OAuth 实现方法",
  "thread_id": "technical-discussions",
  "limit": 5,
  "scope": "session"
}
```

#### 响应

```json
{
  "success": true,
  "query": "Rust OAuth 实现方法",
  "results": [
    {
      "uri": "cortex://threads/tech-disc/timeline/2024/01/10/09_15_30_def456.md",
      "score": 0.92,
      "snippet": "...讨论了使用 OAuth2 客户端库实现 Rust 应用中的身份验证..."
    }
  ],
  "total": 1
}
```

### 3. `list_memories`

列出指定范围内的记忆内容。

#### 参数

| 参数 | 类型 | 必需 | 默认值 | 描述 |
|------|------|------|--------|------|
| `uri` | string | ❌ | "threads" | 要列出的 URI 路径 |
| `limit` | number | ❌ | 50 | 最大条目数 |
| `include_abstracts` | boolean | ❌ | true | 是否包含摘要 |

#### 支持的 URI 模式

- `"threads"` - 列出所有会话
- `"users/{user-id}"` - 列出指定用户的记忆
- `"agents/{agent-id}"` - 列出指定 agent 的记忆
- `"threads/{thread-id}/timeline"` - 列出会话的时间线

#### 示例

```json
{
  "uri": "threads",
  "limit": 20,
  "include_abstracts": true
}
```

#### 响应

```json
{
  "success": true,
  "uri": "threads",
  "entries": [
    {
      "name": "user-preferences",
      "uri": "cortex://threads/user-preferences",
      "is_directory": true,
      "size": 2048,
      "abstract_text": "用户偏好设置和选项"
    }
  ],
  "total": 1
}
```

### 4. `get_memory`

获取特定记忆的完整内容。

#### 参数

| 参数 | 类型 | 必需 | 默认值 | 描述 |
|------|------|------|--------|------|
| `uri` | string | ✅ | - | 记忆的完整 URI |

#### 示例

```json
{
  "uri": "cortex://threads/user-preferences/timeline/2024/01/15/14_30_45_abc123.md"
}
```

#### 响应

```json
{
  "success": true,
  "uri": "cortex://threads/user-preferences/timeline/2024/01/15/14_30_45_abc123.md",
  "content": "# Message\n\n用户偏好使用深色主题，并且喜欢使用 Vim 键位绑定。\n\n---\n*Timestamp: 2024-01-15T14:30:45Z*\n*Role: user*"
}
```

## 🚀 安装与配置

### 构建要求

- Rust 1.70 或更高版本
- 跨平台支持：Linux、macOS、Windows

### 基础构建（仅文件系统搜索）

```bash
# 克隆仓库
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem

# 构建服务器
cargo build --release --bin cortex-mem-mcp

# 二进制位置
./target/release/cortex-mem-mcp
```

### 完整构建（包含向量搜索）

```bash
# 启用 vector-search 功能
cargo build --release --bin cortex-mem-mcp --features vector-search
```

### 配置 Claude Desktop

编辑 Claude Desktop 配置文件：

**macOS**:
```bash
open ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

**Windows**:
```bash
notepad %APPDATA%\Claude\claude_desktop_config.json
```

添加以下配置：

```json
{
  "mcpServers": {
    "cortex-memory": {
      "command": "/path/to/cortex-mem-mcp",
      "args": [
        "--config", "/path/to/config.toml",
        "--tenant", "default"
      ],
      "env": {
        "RUST_LOG": "info",
        "LLM_API_KEY": "your-api-key"
      }
    }
  }
}
```

### 配置选项

#### 命令行参数

| 参数 | 描述 | 默认值 |
|------|------|--------|
| `--config` | 配置文件路径 | `config.toml` |
| `--tenant` | 租户 ID | `default` |
| `--verbose` | 启用详细日志 | - |
| `--log-level` | 日志级别 | `info` |

#### 配置文件 (config.toml)

```toml
[cortex]
# 数据目录
data_dir = "/path/to/cortex-data"

[llm]
# LLM API 配置
api_base_url = "https://api.openai.com/v1"
api_key = "${LLM_API_KEY}"
model_efficient = "gpt-4o-mini"

[embedding]
# 嵌入配置（如果启用向量搜索）
api_base_url = "https://api.openai.com/v1"
api_key = "${EMBEDDING_API_KEY}"
model_name = "text-embedding-3-small"
batch_size = 10

[qdrant]
# 向量数据库（如果启用向量搜索）
url = "http://localhost:6333"
collection_name = "cortex_memories"
```

## 🔄 MCP 工作流

### 典型记忆工作流

1. **对话开始**: Claude 检索相关记忆
```javascript
// Claude 查询用户偏好
await query_memory({
  query: "用户偏好",
  scope: "user",
  limit: 5
});
```

2. **存储新信息**: 将对话中关键信息存储
```javascript
// Claude 存储新的发现
await store_memory({
  content: "用户提到他们正在学习 Rust 异步编程",
  thread_id: "learning-journey",
  role: "user"
});
```

3. **对话结束**: 生成摘要并存储
```javascript
// Claude 总结讨论要点
await store_memory({
  content: "讨论了 Rust 的 async/await、Pin 和 Future，用户理解了基本概念",
  thread_id: "rust-async-discussion",
  role: "assistant"
});
```

### 高级搜索策略

结合多种搜索模式获取最佳结果：

```javascript
// 1. 先从会话中搜索
const sessionResults = await query_memory({
  query: "Rust 错误处理",
  scope: "session",
  limit: 5
});

// 2. 如果需要更多上下文，搜索用户记忆
if (sessionResults.results.length < 3) {
  const userResults = await query_memory({
    query: "Rust 错误处理",
    scope: "user",
    limit: 5
  });
  // 合并结果
  sessionResults.results.push(...userResults.results);
}

// 3. 获取完整内容
const fullContent = await get_memory({
  uri: sessionResults.results[0].uri
});
```

## 🔧 故障排除

### 常见问题

#### 1. 连接失败

**错误**: `Failed to connect to MCP server`

**解决方案**:
1. 检查 Claude Desktop 配置文件路径
2. 验证二进制文件路径和权限
3. 查看日志输出

```bash
# 查看详细日志
./cortex-mem-mcp --verbose --log-level debug
```

#### 2. 记忆存储失败

**错误**: `Failed to store memory`

**解决方案**:
1. 检查数据目录权限
2. 验证 LLM API 配置
3. 确认磁盘空间

```bash
# 检查目录权限
ls -la ./cortex-data
chmod 755 ./cortex-data
```

#### 3. 搜索无结果

**错误**: `Search returned empty results`

**解决方案**:
1. 检查是否有记忆存储
2. 验证搜索查询格式
3. 确认搜索范围

```javascript
// 测试搜索
await list_memories({
  uri: "threads",
  limit: 50
});
```

### 调试模式

启用详细日志进行问题诊断：

```bash
# 启用调试模式
RUST_LOG=debug ./cortex-mem-mcp --verbose

# 查看所有日志
tail -f ~/.local/share/cortex-mem/logs/mcp.log
```

## 🛣️ 路线图

计划中的功能改进：

- [ ] 流式记忆存储（适用于长对话）
- [ ] 记忆优先级和过期机制
- [ ] 批量记忆操作
- [ ] 记忆关联链接
- [ ] 多语言支持
- [ ] 记忆可视化工具

## 📚 示例项目

查看以下示例了解完整实现：

- [`examples/basic-memory-bot`](../examples/basic-memory-bot/) - 基础记忆机器人
- [`examples/multi-agent-memory`](../examples/multi-agent-memory/) - 多代理记忆共享

## 🔗 相关资源

- [Cortex Memory 主文档](../README.md)
- [Cortex Memory 核心](../cortex-mem-core/README.md)
- [Cortex Memory 工具](../cortex-mem-tools/README.md)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Claude Desktop MCP 文档](https://docs.anthropic.com/claude/docs/mcp)

## 🤝 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

MIT 许可证 - 详见 [LICENSE](../../LICENSE) 文件

---

**Built with ❤️ using Rust and Model Context Protocol**