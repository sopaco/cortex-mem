# MemClaw

[![Version](https://img.shields.io/badge/version-0.9.16-blue)](https://github.com/sopaco/cortex-mem)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

OpenClaw 的分层语义记忆插件，支持 L0/L1/L2 三层检索、自动服务管理，以及从 OpenClaw 原生记忆迁移。

## 概述

MemClaw 是一个 OpenClaw 插件，利用 Cortex Memory 的分层记忆架构提供高级语义记忆能力。它通过智能的分层检索来存储、搜索和召回记忆，在速度和上下文之间取得平衡。

## 特性

- **三层记忆架构**：L0（摘要）、L1（概览）和 L2（完整）三层，实现智能检索
- **自动服务管理**：自动启动 Qdrant 向量数据库和 cortex-mem-service
- **语义搜索**：基于向量相似度的全层级记忆搜索
- **会话管理**：创建、列出和关闭记忆会话
- **迁移支持**：一键从 OpenClaw 原生记忆迁移
- **便捷配置**：直接通过 OpenClaw 插件设置配置 LLM/Embedding
- **跨平台**：支持 Windows x64 和 macOS Apple Silicon

## 架构

### 记忆层级

| 层级 | Token 数量 | 内容 | 作用 |
|------|-----------|------|------|
| **L0（摘要）** | ~100 | 高层摘要 | 快速筛选 |
| **L1（概览）** | ~2000 | 要点 + 上下文 | 上下文精炼 |
| **L2（完整）** | 完整 | 原始内容 | 精确匹配 |

搜索引擎内部查询所有三个层级，返回包含 `snippet` 和 `content` 的统一结果。

### 系统组件

```
OpenClaw + MemClaw Plugin
         │
         ├── cortex_search    → 搜索记忆
         ├── cortex_recall    → 召回上下文
         ├── cortex_add_memory → 存储记忆
         ├── cortex_list_sessions → 列出会话
         ├── cortex_close_session → 关闭并提取
         └── cortex_migrate   → 迁移现有记忆
                    │
                    ▼
         cortex-mem-service (端口 8085)
                    │
                    ▼
         Qdrant (端口 6334)
```

## 安装

### 环境要求

| 要求 | 详情 |
|------|------|
| **平台** | Windows x64, macOS Apple Silicon |
| **Node.js** | ≥ 20.0.0 |
| **OpenClaw** | 已安装并配置 |

### 安装插件

```bash
openclaw plugins install @memclaw/memclaw
```

### 本地开发安装

开发者如需使用本地版本或开发插件：

```bash
# 克隆仓库
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem/examples/@memclaw/plugin

# 安装依赖
bun install

# 构建插件
bun run build
```

**方式 A：使用 plugins.load.paths**

```json
{
  "plugins": {
    "load": {
      "paths": ["/path/to/cortex-mem/examples/@memclaw/plugin"]
    },
    "entries": {
      "memclaw": { "enabled": true }
    }
  }
}
```

**方式 B：符号链接到扩展目录**

```bash
mkdir -p ~/.openclaw/extensions
ln -sf "$(pwd)" ~/.openclaw/extensions/memclaw
```

然后在 `openclaw.json` 中启用：

```json
{
  "plugins": {
    "entries": {
      "memclaw": { "enabled": true }
    }
  }
}
```

代码修改后，执行 `bun run build` 重新构建，然后重启 OpenClaw。

## 配置

### 插件配置

直接通过 `openclaw.json` 中的 OpenClaw 插件设置配置 MemClaw：

```json
{
  "plugins": {
    "entries": {
      "memclaw": {
        "enabled": true,
        "config": {
          "serviceUrl": "http://localhost:8085",
          "tenantId": "tenant_claw",
          "autoStartServices": true,
          "llmApiBaseUrl": "https://api.openai.com/v1",
          "llmApiKey": "your-llm-api-key",
          "llmModel": "gpt-4o-mini",
          "embeddingApiBaseUrl": "https://api.openai.com/v1",
          "embeddingApiKey": "your-embedding-api-key",
          "embeddingModel": "text-embedding-3-small"
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

> **注意**：设置 `memorySearch.enabled: false` 以禁用 OpenClaw 内置记忆搜索，改用 MemClaw。

### 配置选项

| 选项 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `serviceUrl` | string | `http://localhost:8085` | Cortex Memory 服务 URL |
| `tenantId` | string | `tenant_claw` | 租户 ID，用于数据隔离 |
| `autoStartServices` | boolean | `true` | 自动启动 Qdrant 和服务 |
| `defaultSessionId` | string | `default` | 记忆操作的默认会话 |
| `searchLimit` | number | `10` | 默认搜索结果数量 |
| `minScore` | number | `0.6` | 最小相关度分数 (0-1) |
| `qdrantPort` | number | `6334` | Qdrant 端口 (gRPC) |
| `servicePort` | number | `8085` | cortex-mem-service 端口 |
| `llmApiBaseUrl` | string | `https://api.openai.com/v1` | LLM API 端点 URL |
| `llmApiKey` | string | - | LLM API 密钥（必填） |
| `llmModel` | string | `gpt-5-mini` | LLM 模型名称 |
| `embeddingApiBaseUrl` | string | `https://api.openai.com/v1` | Embedding API 端点 URL |
| `embeddingApiKey` | string | - | Embedding API 密钥（必填） |
| `embeddingModel` | string | `text-embedding-3-small` | Embedding 模型名称 |

### 通过 UI 配置

你也可以通过 OpenClaw UI 配置插件：

1. 打开 OpenClaw 设置（`openclaw.json` 或通过 UI）
2. 导航到 插件 → MemClaw → 配置
3. 填写 LLM 和 Embedding 相关的必填字段
4. 保存并**重启 OpenClaw Gateway** 使配置生效

## 可用工具

### cortex_search

使用 L0/L1/L2 三层检索在所有记忆中进行语义搜索。

```json
{
  "query": "数据库架构决策",
  "limit": 5,
  "min_score": 0.6
}
```

### cortex_recall

召回带有更多上下文的记忆（摘要 + 完整内容）。

```json
{
  "query": "用户代码风格偏好",
  "limit": 10
}
```

### cortex_add_memory

存储消息以供后续检索。

```json
{
  "content": "用户偏好 TypeScript 严格模式",
  "role": "assistant",
  "session_id": "default"
}
```

### cortex_list_sessions

列出所有记忆会话及其状态和消息数量。

### cortex_close_session

关闭会话并触发记忆提取管道（耗时 30-60 秒）。

```json
{
  "session_id": "default"
}
```

> **重要提示**：请在自然的检查点主动调用此工具，不要等到对话结束。理想时机：完成重要任务后、话题转换时、或积累足够对话内容后。

### cortex_migrate

从 OpenClaw 原生记忆迁移到 MemClaw。初始设置时运行一次即可。

### cortex_maintenance

对 MemClaw 数据执行定期维护（清理、重建索引、确保所有层级生成）。

## 快速决策流程

| 场景 | 工具 |
|------|------|
| 需要查找信息 | `cortex_search` |
| 需要更多上下文 | `cortex_recall` |
| 保存重要信息 | `cortex_add_memory` |
| 完成任务/话题 | `cortex_close_session` |
| 首次使用且有现有记忆 | `cortex_migrate` |

更多关于工具选择、会话生命周期和最佳实践的详细指南，请参阅 [技能文档](skills/memclaw/SKILL.md)。

## 故障排查

### 插件无法工作

1. **检查配置**：打开 OpenClaw 设置，验证 MemClaw 插件配置，特别是 LLM 和 Embedding 设置
2. **重启 OpenClaw Gateway**：配置更改需要重启网关才能生效
3. **验证服务**：运行 `cortex_list_sessions` 检查服务是否响应

### 服务无法启动

1. 检查端口 6333、6334、8085 是否可用
2. 验证 LLM 和 Embedding 凭证是否正确配置
3. 运行 `openclaw skills` 检查插件状态

### 搜索无结果

1. 运行 `cortex_list_sessions` 验证会话是否存在
2. 降低 `min_score` 阈值（默认：0.6）
3. 确保已存储记忆（运行 `cortex_close_session` 提取记忆）

### 迁移失败

1. 确保 OpenClaw 工作区存在于 `~/.openclaw/workspace`
2. 验证记忆文件存在于 `~/.openclaw/workspace/memory/`

## CLI 参考

高级用户可直接使用 cortex-mem-cli：

```bash
# 列出会话
cortex-mem-cli --config config.toml --tenant tenant_claw session list

# 确保所有层级已生成
cortex-mem-cli --config config.toml --tenant tenant_claw layers ensure-all

# 重建向量索引
cortex-mem-cli --config config.toml --tenant tenant_claw vector reindex
```

## 文档

- **[技能文档](skills/memclaw/SKILL.md)** — Agent 技能指南，含故障排查
- **[最佳实践](skills/memclaw/references/best-practices.md)** — 工具选择、会话生命周期、搜索策略
- **[工具参考](skills/memclaw/references/tools.md)** — 详细工具参数和示例

## 许可证

MIT