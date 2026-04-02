# MemClaw

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
- **跨平台**：支持 Windows x64、macOS Apple Silicon 和 Linux x64

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
         ├── cortex_search        → 分层语义搜索
         ├── cortex_recall        → 召回上下文
         ├── cortex_add_memory    → 存储记忆
         ├── cortex_commit_session → 提交并提取
         ├── cortex_migrate       → 迁移现有记忆
         ├── cortex_maintenance   → 定期维护
         ├── cortex_ls            → 浏览记忆文件系统
         ├── cortex_get_abstract  → L0 快速预览
         ├── cortex_get_overview  → L1 中等详情
         ├── cortex_get_content   → L2 完整内容
         └── cortex_explore       → 智能探索
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
          "llmModel": "gpt-5-mini",
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

分层语义搜索，支持精细控制返回内容。

**关键参数：**
- `return_layers`: `["L0"]` (默认，约 100 tokens), `["L0","L1"]` (约 2100 tokens), `["L0","L1","L2"]` (完整内容)

```json
{
  "query": "数据库架构决策",
  "limit": 5,
  "min_score": 0.6,
  "return_layers": ["L0"]
}
```

如需更多上下文，使用 `return_layers: ["L0","L1"]`。如需完整内容，使用 `["L0","L1","L2"]`。

### cortex_recall

召回带有更多上下文的记忆（摘要 + 完整内容）。

```json
{
  "query": "用户代码风格偏好",
  "limit": 10
}
```

### cortex_add_memory

存储消息以供后续检索，支持可选的元数据。

```json
{
  "content": "用户偏好 TypeScript 严格模式",
  "role": "assistant",
  "session_id": "default",
  "metadata": {
    "tags": ["preference", "typescript"],
    "importance": "high"
  }
}
```

**参数说明：**
- `content`: 消息内容（必填）
- `role`: `"user"`、`"assistant"` 或 `"system"`（默认：user）
- `session_id`: 会话/线程 ID（不指定则使用默认值）
- `metadata`: 可选元数据，如标签、重要性或自定义字段

### cortex_commit_session

提交会话并触发记忆提取管道（耗时 30-60 秒）。

```json
{
  "session_id": "default"
}
```

> **重要提示**：请在自然的检查点主动调用此工具，不要等到对话结束。理想时机：完成重要任务后、话题转换时、或积累足够对话内容后。

### cortex_ls

列出目录内容，像浏览虚拟文件系统一样浏览记忆空间。

```json
{
  "uri": "cortex://session",
  "recursive": false,
  "include_abstracts": false
}
```

**参数说明：**
- `uri`: 要列出的目录 URI（默认：`cortex://session`）
- `recursive`: 是否递归列出子目录
- `include_abstracts`: 是否显示 L0 摘要以快速预览

**常用 URI：**
- `cortex://session` - 列出所有会话
- `cortex://session/{session_id}` - 浏览特定会话
- `cortex://session/{session_id}/timeline` - 查看时间线消息
- `cortex://user/{user_id}/preferences` - 查看用户偏好（提取的记忆）
- `cortex://user/{user_id}/entities` - 查看用户实体（人物、项目等）
- `cortex://agent/{agent_id}/cases` - 查看 Agent 问题解决案例

### cortex_get_abstract

获取 L0 摘要层（约 100 tokens），用于快速相关性检查。

```json
{
  "uri": "cortex://session/abc123/timeline/2024-01-15_001.md"
}
```

用于在阅读更多内容前快速判断内容是否相关。

### cortex_get_overview

获取 L1 概览层（约 2000 tokens），包含核心信息和上下文。

```json
{
  "uri": "cortex://session/abc123/timeline/2024-01-15_001.md"
}
```

当摘要不够详细但不需要完整内容时使用。

### cortex_get_content

获取 L2 完整内容层 - 完整的原始内容。

```json
{
  "uri": "cortex://session/abc123/timeline/2024-01-15_001.md"
}
```

仅在需要完整、未处理的内容时使用。

### cortex_explore

智能探索，结合搜索和浏览进行引导式发现。

```json
{
  "query": "认证流程",
  "start_uri": "cortex://session",
  "return_layers": ["L0"]
}
```

返回带有相关性分数的探索路径和匹配结果。

### cortex_migrate

从 OpenClaw 原生记忆迁移到 MemClaw。初始设置时运行一次即可。

### cortex_maintenance

对 MemClaw 数据执行定期维护（清理、重建索引、确保所有层级生成）。

```json
{
  "dryRun": false,
  "commands": ["prune", "reindex", "ensure-all"]
}
```

**参数说明：**
- `dryRun`: 预览更改但不执行（默认：false）
- `commands`: 要运行的维护命令（默认：全部）

此工具每 3 小时自动运行一次。当搜索结果不完整或过时时可手动调用。

## 快速决策流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     如何访问记忆                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  你知道信息在哪里吗？                                             │
│       │                                                          │
│       ├── 是 ──► 使用直接分层访问                                 │
│       │           cortex_ls → cortex_get_abstract/overview/content│
│       │                                                          │
│       └── 否 ──► 你知道在找什么吗？                                │
│                    │                                             │
│                    ├── 是 ──► 使用语义搜索                        │
│                    │            cortex_search                     │
│                    │                                             │
│                    └── 否 ──► 使用智能探索                        │
│                                 cortex_explore                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

| 场景 | 工具 |
|------|------|
| 跨所有会话查找信息 | `cortex_search` |
| 浏览记忆结构 | `cortex_ls` |
| 快速检查 URI 相关性 | `cortex_get_abstract` |
| 获取相关 URI 的更多详情 | `cortex_get_overview` |
| 需要完整原始内容 | `cortex_get_content` |
| 有目的地探索 | `cortex_explore` |
| 保存重要信息 | `cortex_add_memory` |
| 完成任务/话题 | `cortex_commit_session` |
| 首次使用且有现有记忆 | `cortex_migrate` |
| 数据维护 | `cortex_maintenance` |

更多关于工具选择、会话生命周期和最佳实践的详细指南，请参阅 [技能文档](skills/memclaw/SKILL.md)。

## 故障排查

### 插件无法工作

### 服务无法启动

1. 检查端口 6333、6334、8085 是否可用
2. 验证 LLM 和 Embedding 凭证是否正确配置
3. 运行 `openclaw skills` 检查插件状态

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
