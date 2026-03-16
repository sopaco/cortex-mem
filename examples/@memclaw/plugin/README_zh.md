# MemClaw

OpenClaw 的分层语义内存插件，支持 L0/L1/L2 三层检索、自动服务管理，并可从 OpenClaw 原生内存迁移。

## 概述

MemClaw 是一个 OpenClaw 插件，利用 Cortex Memory 的分层内存架构提供高级语义内存功能。它以智能分层检索方式存储、搜索和召回记忆，兼顾速度与上下文。

## 功能特性

- **三层内存架构**：L0（摘要）、L1（概览）和 L2（完整）层次，实现智能检索
- **自动服务管理**：自动启动 Qdrant 向量数据库和 cortex-mem-service
- **语义搜索**：基于向量的相似性搜索，跨所有内存层
- **会话管理**：创建、列出和关闭内存会话
- **迁移支持**：一键从 OpenClaw 原生内存迁移
- **跨平台支持**：Windows x64 和 macOS Apple Silicon

## 架构

### 内存层次

| 层次 | Token 数 | 内容 | 作用 |
|------|----------|------|------|
| **L0（摘要）** | ~100 | 高层次摘要 | 快速过滤 |
| **L1（概览）** | ~2000 | 关键点 + 上下文 | 上下文优化 |
| **L2（完整）** | 完整内容 | 原始内容 | 精确匹配 |

搜索引擎内部查询所有三层，返回统一的包含 `snippet`（摘要）和 `content`（完整内容）的结果。

### 系统组件

```
OpenClaw + MemClaw 插件
         │
         ├── cortex_search    → 搜索记忆
         ├── cortex_recall    → 带上下文召回
         ├── cortex_add_memory → 存储记忆
         ├── cortex_list_sessions → 列出会话
         ├── cortex_close_session → 关闭并提取
         └── cortex_migrate   → 迁移现有内存
                    │
                    ▼
         cortex-mem-service（端口 8085）
                    │
                    ▼
         Qdrant（端口 6334）
```

## 安装

### 环境要求

| 要求 | 说明 |
|------|------|
| **平台** | Windows x64、macOS Apple Silicon |
| **Node.js** | ≥ 22.0.0 |
| **OpenClaw** | 已安装并配置完成 |

### 安装插件

```bash
openclaw plugins install memclaw
```

### 本地开发安装

适用于开发者使用本地版本或进行插件开发：

```bash
# 克隆仓库
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem/examples/memclaw

# 安装依赖
bun install

# 构建插件
bun run build

# 创建符号链接到插件目录
# 这样 OpenClaw 会使用本地版本
mkdir -p ~/.openclaw/plugins
ln -sf "$(pwd)" ~/.openclaw/plugins/memclaw
```

然后在 `openclaw.json` 中配置本地插件路径：

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "path": "./plugins/memclaw"
      }
    }
  }
}
```

代码修改后，使用 `bun run build` 重新构建，然后重启 OpenClaw。

### 配置 OpenClaw

编辑 `openclaw.json`：

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://localhost:8085",
          "tenantId": "tenant_claw",
          "autoStartServices": true
        }
      }
    }
  },
  "agents": {
    "defaults": {
      "memorySearch": { "enabled": false }
    }
  }
}
```

> **注意**：将 `memorySearch.enabled` 设置为 `false` 以禁用 OpenClaw 内置的内存搜索，改为使用 MemClaw。

### 配置 LLM

首次运行时，MemClaw 会创建配置文件：

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\memclaw\config.toml` |
| macOS | `~/Library/Application Support/memclaw/config.toml` |

编辑配置文件，填写必要字段：

```toml
[llm]
api_key = "xxx"  # 必填：您的 LLM API 密钥

[embedding]
api_key = "xxx"  # 必填：您的嵌入 API 密钥（可与 llm.api_key 相同）
```

然后重启 OpenClaw。

## 可用工具

### cortex_search

使用 L0/L1/L2 分层检索进行语义搜索。

```json
{
  "query": "数据库架构决策",
  "limit": 5,
  "min_score": 0.6
}
```

### cortex_recall

带更多上下文召回记忆（摘要 + 完整内容）。

```json
{
  "query": "用户对代码风格的偏好",
  "limit": 10
}
```

### cortex_add_memory

存储消息以便后续检索。

```json
{
  "content": "用户更喜欢 TypeScript 严格模式",
  "role": "assistant",
  "session_id": "default"
}
```

### cortex_list_sessions

列出所有内存会话，显示状态和消息数量。

### cortex_close_session

关闭会话并触发内存提取流程（需要 30-60 秒）。

```json
{
  "session_id": "default"
}
```

### cortex_migrate

从 OpenClaw 原生内存迁移到 MemClaw。在初始设置时运行一次。

## 配置选项

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `serviceUrl` | string | `http://localhost:8085` | Cortex Memory 服务地址 |
| `tenantId` | string | `tenant_claw` | 租户 ID，用于数据隔离 |
| `autoStartServices` | boolean | `true` | 自动启动 Qdrant 和服务 |
| `defaultSessionId` | string | `default` | 内存操作的默认会话 |
| `searchLimit` | number | `10` | 默认搜索结果数量 |
| `minScore` | number | `0.6` | 最小相关性分数（0-1） |

## 快速决策流程

1. **需要查找内容** → `cortex_search`
2. **需要更多上下文** → `cortex_recall`
3. **保存重要信息** → `cortex_add_memory`
4. **对话完成** → `cortex_close_session`
5. **首次设置** → `cortex_migrate`

## 故障排除

### 服务无法启动

1. 检查端口 6333、6334、8085 是否可用
2. 验证 config.toml 中的 `api_key` 字段已填写
3. 运行 `openclaw skills` 检查插件状态

### 搜索无结果

1. 运行 `cortex_list_sessions` 验证会话是否存在
2. 降低 `min_score` 阈值（默认值：0.6）
3. 使用 `cortex-mem-cli stats` 检查服务健康状态

### 迁移失败

1. 确保 OpenClaw 工作区存在于 `~/.openclaw/workspace`
2. 验证内存文件存在于 `~/.openclaw/workspace/memory/`

## CLI 参考

高级用户可直接使用 cortex-mem-cli：

```bash
# 列出会话
cortex-mem-cli --config config.toml --tenant tenant_claw session list

# 确保所有层次都已生成
cortex-mem-cli --config config.toml --tenant tenant_claw layers ensure-all

# 重建向量索引
cortex-mem-cli --config config.toml --tenant tenant_claw vector reindex
```

## 许可证

MIT
