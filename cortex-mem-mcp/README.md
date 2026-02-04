# Cortex-Mem MCP Server

Model Context Protocol (MCP) server for Cortex-Mem memory management system.

## 功能

提供4个MCP工具，让AI助手能够读写长期记忆：

### 1. `store_memory`
存储新的记忆到系统中。

**参数**:
- `content` (required): 记忆内容
- `thread_id` (optional): 会话ID，默认为"default"
- `role` (optional): 消息角色 (user/assistant/system)
- `tags` (optional): 标签数组

**示例**:
```json
{
  "content": "用户喜欢使用Rust编程",
  "thread_id": "user-prefs",
  "role": "user"
}
```

### 2. `query_memory`
智能搜索记忆，支持多种搜索模式。

**参数**:
- `query` (required): 搜索查询
- `mode` (optional): 搜索模式 - "filesystem" (默认), "vector", "hybrid"
- `thread_id` (optional): 限定搜索范围
- `limit` (optional): 最大结果数，默认10
- `min_score` (optional): 最小相关性分数(0-1)，默认0.3

**搜索模式说明**:
- `filesystem`: 全文关键词搜索（总是可用）
- `vector`: 语义相似度搜索（需要编译时启用`vector-search` feature）
- `hybrid`: 结合两种搜索（需要编译时启用`vector-search` feature）

**示例**:
```json
{
  "query": "Rust OAuth实现",
  "mode": "filesystem",
  "limit": 5
}
```

### 3. `list_memories`
列出指定范围的记忆。

**参数**:
- `thread_id` (optional): 会话ID
- `dimension` (optional): 维度(threads/agents/users/global)
- `include_metadata` (optional): 是否包含元数据

**示例**:
```json
{
  "thread_id": "user-prefs"
}
```

### 4. `get_memory`
获取特定记忆内容。

**参数**:
- `uri` (required): 记忆URI

**示例**:
```json
{
  "uri": "cortex://threads/my-session/timeline/2026-02/03/10_30_45_abc123.md"
}
```

## 安装

### 基础构建（仅文件系统搜索）

```bash
cargo build --release --bin cortex-mem-mcp
```

### 完整构建（包含向量搜索）

```bash
cargo build --release --bin cortex-mem-mcp --features vector-search
```

### 配置Claude Desktop

编辑配置文件:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

添加以下配置:

```json
{
  "mcpServers": {
    "cortex-mem": {
      "command": "/path/to/cortex-mem/target/release/cortex-mem-mcp",
      "env": {
        "CORTEX_DATA_DIR": "/path/to/your/cortex-data"
      }
    }
  }
}
```

### 配置Cline/Roo-Cline

在 MCP settings 中添加:

```json
{
  "cortex-mem": {
    "command": "/path/to/cortex-mem/target/release/cortex-mem-mcp",
    "env": {
      "CORTEX_DATA_DIR": "/path/to/your/cortex-data"
    }
  }
}
```

## 环境变量

- `CORTEX_DATA_DIR`: 数据存储目录（默认: `./cortex-data`）

## 通信协议

MCP server使用JSON-RPC 2.0通过stdio通信。

**请求示例**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "store_memory",
    "arguments": {
      "content": "This is a test memory"
    }
  }
}
```

**响应示例**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Memory stored successfully\nURI: cortex://threads/default/timeline/...\nID: abc-123-..."
      }
    ]
  }
}
```

## 测试

### 手动测试

```bash
# 启动服务器（会监听stdin）
./target/release/cortex-mem-mcp

# 发送测试请求（另一个终端）
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/release/cortex-mem-mcp
```

### 与Claude Desktop集成测试

1. 配置claude_desktop_config.json
2. 重启Claude Desktop
3. 在对话中要求Claude使用记忆功能
4. 检查 `~/Library/Logs/Claude/mcp*.log` 查看日志

## 开发

### 目录结构

```
cortex-mem-mcp/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs      # MCP服务器入口和stdio传输
    └── server.rs    # MCP工具实现
```

### 添加新工具

在 `server.rs` 中:

1. 在 `handle_tools_list` 添加工具定义
2. 在 `handle_tools_call` 添加工具调用分支
3. 实现工具函数 `async fn tool_xxx(&self, args) -> std::result::Result<String, String>`

## License

MIT
