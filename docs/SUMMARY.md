# Cortex Memory 项目文档总览

**版本**: V2.0.0  
**最后更新**: 2026-02-12  
**状态**: ✅ 生产就绪

---

## 快速导航

### 📚 核心文档

| 文档 | 路径 | 描述 |
|------|------|------|
| [架构设计](architecture/ARCHITECTURE.md) | `docs/architecture/ARCHITECTURE.md` | 系统整体架构设计 |
| [子项目模块](architecture/MODULES.md) | `docs/architecture/MODULES.md` | 各 crate 详细说明 |
| [功能流程](guides/WORKFLOWS.md) | `docs/guides/WORKFLOWS.md` | 核心功能流程 |

### 📖 其他文档

| 文档 | 路径 | 描述 |
|------|------|------|
| [项目状态](../PROJECT_STATUS.md) | `PROJECT_STATUS.md` | 当前状态和路线图 |
| [项目评估](../PROJECT_EVALUATION_REPORT.md) | `PROJECT_EVALUATION_REPORT.md` | 项目评估报告 |
| [更新日志](../CHANGELOG_2026-02-10.md) | `CHANGELOG_2026-02-10.md` | 版本更新记录 |
| [待办事项](../TODO.md) | `TODO.md` | 待办任务列表 |

---

## 项目简介

Cortex Memory 是一个高性能、模块化的 AI Agent 记忆管理系统，采用 `cortex://` 虚拟 URI 协议，实现 L0/L1/L2 三层抽象架构，为 AI Agent 提供长期记忆存储和智能检索能力。

### 核心特性

- ✅ **虚拟文件系统**: `cortex://` 协议，纯 Markdown 存储
- ✅ **三层架构**: L0/L1/L2 抽象层，Token 效率提升 80-92%
- ✅ **智能检索**: 文件系统 + 向量 + 混合搜索
- ✅ **会话管理**: 完整的对话生命周期管理
- ✅ **记忆提取**: LLM 驱动的自动提取
- ✅ **丰富工具链**: CLI、MCP、HTTP、Tools 库、Rig 集成
- ✅ **多维度存储**: session/user/agent 三种存储范围
- ✅ **租户隔离**: 支持多租户架构

---

## 项目结构

```
cortex-mem/
├── docs/                          # 📚 项目文档
│   ├── SUMMARY.md                 # 本文档 - 文档总览
│   ├── architecture/              # 架构文档
│   │   ├── ARCHITECTURE.md        # 系统架构设计
│   │   └── MODULES.md             # 子项目模块说明
│   ├── guides/                    # 使用指南
│   │   └── WORKFLOWS.md           # 功能流程
│   ├── modules/                   # 模块详细文档 (待补充)
│   └── api/                       # API 文档 (待补充)
│
├── cortex-mem-core/               # 核心库
│   └── src/
│       ├── filesystem/            # 虚拟文件系统
│       ├── session/               # 会话管理
│       ├── layers/                # 三层抽象
│       ├── retrieval/             # 检索引擎
│       ├── extraction/            # 记忆提取
│       ├── llm/                   # LLM 集成
│       ├── automation/            # 自动化
│       ├── index/                 # 全文索引
│       ├── vector_store/          # 向量存储 (可选)
│       ├── embedding/             # Embedding (可选)
│       └── search/                # 向量搜索 (可选)
│
├── cortex-mem-cli/                # 命令行工具
├── cortex-mem-mcp/                # MCP 服务器
├── cortex-mem-service/            # HTTP REST API
├── cortex-mem-tools/              # 高级工具库
├── cortex-mem-rig/                # Rig 框架集成
├── cortex-mem-config/             # 配置管理
├── cortex-mem-insights/           # Web 界面 (开发中)
│
├── examples/                      # 示例项目
│   └── cortex-mem-tars/           # TUI 示例应用
│
├── README.md                      # 项目主文档
├── README_zh.md                   # 中文文档
├── Cargo.toml                     # Workspace 配置
└── PROJECT_STATUS.md              # 项目状态
```

---

## 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem

# 基础构建
cargo build --release --workspace

# 完整构建（含向量搜索）
cargo build --release --workspace --features vector-search
```

### CLI 使用

```bash
# 创建会话
cortex-mem session create my-session --title "技术讨论"

# 添加消息
cortex-mem add --thread my-session "如何实现 OAuth 2.0？"

# 搜索记忆
cortex-mem search "OAuth"

# 查看统计
cortex-mem stats
```

### 代码使用

```rust
use cortex_mem_core::{CortexFilesystem, FilesystemOperations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化
    let filesystem = Arc::new(CortexFilesystem::new("./cortex-data"));
    filesystem.initialize().await?;
    
    // 存储记忆
    filesystem.write(
        "cortex://user/my-memory.md",
        "Hello, Cortex Memory!"
    ).await?;
    
    // 读取记忆
    let content = filesystem.read("cortex://user/my-memory.md").await?;
    println!("{}", content);
    
    Ok(())
}
```

---

## 架构概览

### 系统架构

```
应用层 (CLI / MCP / HTTP / Web)
    │
    ▼
工具层 (cortex-mem-tools / cortex-mem-rig)
    │
    ▼
核心层 (cortex-mem-core)
    │
    ├── 文件系统 (cortex://)
    ├── 会话管理
    ├── 三层抽象 (L0/L1/L2)
    ├── 检索引擎
    ├── 记忆提取
    └── LLM 集成
    │
    ▼
存储层 (Markdown / Qdrant)
```

### 核心概念

| 概念 | 说明 |
|------|------|
| `cortex://` | 虚拟 URI 协议，统一内存访问 |
| L0/L1/L2 | 三层内容抽象，优化 Token 使用 |
| session | 会话级存储，临时对话 |
| user | 用户长期记忆 |
| agent | Agent 专属记忆 |
| tenant | 租户隔离，多用户支持 |

---

## 功能模块

### 1. 虚拟文件系统 (filesystem)

- URI 解析和转换
- 文件读写操作
- 租户隔离支持

### 2. 会话管理 (session)

- 会话生命周期管理
- 消息存储和检索
- Timeline 时间轴组织
- 参与者管理

### 3. 三层抽象 (layers)

- L0 Abstract (~100 tokens)
- L1 Overview (~2000 tokens)
- L2 Detail (完整内容)
- 自动生成和缓存

### 4. 检索引擎 (retrieval)

- 意图分析
- 递归检索
- 相关性计算

### 5. 记忆提取 (extraction)

- 事实提取
- 决策记录
- 实体识别
- 用户画像

### 6. LLM 集成 (llm)

- rig-core 0.23 封装
- 支持 OpenAI 兼容 API
- 自部署 LLM 支持

### 7. 工具链

| 工具 | 类型 | 说明 |
|------|------|------|
| CLI | Binary | 命令行工具 |
| MCP | Binary | Claude Desktop 集成 |
| Service | Binary | HTTP REST API |
| Tools | Library | 高级工具库 |
| Rig | Library | Rig 框架集成 |

---

## 搜索模式

| 模式 | 说明 | 依赖 |
|------|------|------|
| filesystem | 基于文件的全文搜索 | 无 |
| vector | 基于向量的语义搜索 | Qdrant |
| hybrid | 混合搜索 | Qdrant |

---

## 文档地图

### 架构文档

- [ARCHITECTURE.md](architecture/ARCHITECTURE.md)
  - 系统架构图
  - 核心架构原则
  - 模块详细设计
  - 数据流
  - 部署架构

- [MODULES.md](architecture/MODULES.md)
  - 子项目列表
  - 每个 crate 的详细说明
  - 依赖关系
  - 使用示例

### 使用指南

- [WORKFLOWS.md](guides/WORKFLOWS.md)
  - 会话管理流程
  - 消息存储流程
  - 三层抽象生成
  - 记忆提取流程
  - 检索和搜索流程
  - 租户隔离流程
  - 完整使用场景

### 项目文档

- [README.md](../README.md) - 项目介绍和快速开始
- [PROJECT_STATUS.md](../PROJECT_STATUS.md) - 项目状态和路线图
- [PROJECT_EVALUATION_REPORT.md](../PROJECT_EVALUATION_REPORT.md) - 项目评估
- [CHANGELOG_2026-02-10.md](../CHANGELOG_2026-02-10.md) - 更新日志
- [TODO.md](../TODO.md) - 待办事项

---

## 开发指南

### 构建

```bash
# 开发构建
cargo build --workspace

# 发布构建
cargo build --release --workspace

# 带向量搜索
cargo build --release --workspace --features vector-search
```

### 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定包测试
cargo test -p cortex-mem-core
```

### 代码检查

```bash
# 格式化
cargo fmt --all

# 静态检查
cargo clippy --all-targets --all-features
```

---

## 贡献指南

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

---

## 许可证

本项目采用 [MIT License](../LICENSE)

---

## 联系方式

- GitHub Issues: [cortex-mem/issues](https://github.com/sopaco/cortex-mem/issues)
- Discussions: [cortex-mem/discussions](https://github.com/sopaco/cortex-mem/discussions)

---

**Built with ❤️ using Rust, Axum, and SvelteKit**
