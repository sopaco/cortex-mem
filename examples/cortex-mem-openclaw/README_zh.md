# Cortex Memory Plugin for OpenClaw

[English](./README.md) | 中文

为 OpenClaw 提供层级语义记忆能力的插件，支持 L0/L1/L2 三层检索。

## 功能特性

- **层级语义搜索**：L0（摘要）→ L1（概览）→ L2（完整内容）逐层检索
- **自动向量化**：消息自动生成向量嵌入，支持语义相似度搜索
- **会话隔离**：支持多会话独立记忆空间
- **HTTP API**：通过 cortex-mem-service REST API 与核心服务通信

## 前置条件

1. **cortex-mem-service** 运行中
   ```bash
   # 在 cortex-mem 项目根目录
   cargo run -p cortex-mem-service -- --data-dir ./cortex-data
   ```

2. **Qdrant 向量数据库**（可选，用于向量搜索）
3. **Embedding 服务**（OpenAI 兼容 API）

## 安装

### 方式一：本地链接安装（开发模式）

```bash
# 在 cortex-mem 项目根目录
cd examples/cortex-mem-openclaw

# 安装依赖并构建
npm install
npm run build

# 链接到 OpenClaw
openclaw plugins install --link $(pwd)
```

### 方式二：npm 发布后安装

```bash
openclaw plugins install @cortex-mem/openclaw-plugin
```

## 配置

在 OpenClaw 配置文件（`~/.openclaw/openclaw.json` 或项目目录）中添加：

```json
{
  "plugins": {
    "entries": {
      "cortex-mem": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://127.0.0.1:8085",
          "tenantId": "tenant_claw",
          "defaultSessionId": "default",
          "searchLimit": 10,
          "minScore": 0.6
        }
      }
    }
  }
}
```

### 配置说明

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `serviceUrl` | string | `http://127.0.0.1:8085` | cortex-mem-service 的 HTTP 端点 |
| `tenantId` | string | `tenant_claw` | 租户隔离标识，用于 Qdrant collection 与文件系统隔离 |
| `defaultSessionId` | string | `default` | 默认会话 ID |
| `searchLimit` | integer | 10 | 搜索结果最大数量 |
| `minScore` | number | 0.6 | 最小相关性分数阈值 |

## 工具说明

### cortex_search

层级语义搜索，返回相关记忆片段。

```json
{
  "query": "用户偏好设置",
  "scope": "session-123",  // 可选，限定搜索范围
  "limit": 10,
  "min_score": 0.6
}
```

返回：
```json
{
  "results": [
    {
      "uri": "cortex://session/abc/timeline/2026-03/11/10_30_00_xxx.md",
      "score": 0.89,
      "snippet": "用户偏好深色主题..."
    }
  ],
  "total": 1
}
```

### cortex_recall

分层召回记忆，支持指定返回层级。

```json
{
  "query": "项目架构决策",
  "layers": ["L0", "L1"],  // L0=摘要, L1=概览, L2=完整内容
  "scope": "session-123",
  "limit": 5
}
```

### cortex_add_memory

向指定会话添加记忆。

```json
{
  "content": "用户选择了 PostgreSQL 作为主数据库",
  "role": "assistant",  // user/assistant/system
  "session_id": "session-123"  // 可选，默认使用 defaultSessionId
}
```

### cortex_list_sessions

列出所有记忆会话。

```json
{}
```

返回：
```json
{
  "sessions": [
    {
      "thread_id": "session-123",
      "status": "active",
      "message_count": 42,
      "created_at": "2026-03-11T10:00:00Z"
    }
  ]
}
```

## 架构

```
OpenClaw Gateway
       │
       ▼
cortex-mem-plugin (TypeScript)
       │
       ▼ HTTP REST
cortex-mem-service (Rust)
       │
       ├─► Qdrant (向量存储)
       ├─► CortexFilesystem (文件存储)
       └─► LLM (层级生成)
```

## 开发

```bash
# 安装依赖
npm install

# 开发模式（监视编译）
npm run dev

# 构建
npm run build

# 运行测试
npm test
```

## 许可证

MIT
