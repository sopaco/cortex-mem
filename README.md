# Cortex-Mem V2

**基于文件系统的AI Agent记忆管理系统**

Cortex-Mem是一个高性能、模块化的记忆管理系统，采用`cortex://`虚拟URI协议，实现L0/L1/L2三层抽象架构，为AI Agent提供长期记忆存储和智能检索能力。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)

## ✨ 核心特性

- 🗂️ **虚拟文件系统**: 使用`cortex://`协议统一内存访问
- 🏗️ **三层架构**: L0抽象层(~100 tokens) → L1概览层(~2k tokens) → L2完整内容
- 🔍 **智能检索**: 基于意图分析的递归检索引擎
- 💬 **会话管理**: 完整的对话生命周期和时间轴组织
- 🧠 **记忆提取**: 自动从对话中提取facts、decisions和entities  
- 🤖 **LLM集成**: 基于rig-core的LLM客户端，支持自定义OpenAI兼容API
- 🛠️ **CLI工具**: 7个核心命令，彩色友好输出
- 🔌 **MCP服务器**: 基于rmcp实现，与Claude Desktop等AI工具无缝集成
- 📦 **零依赖存储**: 纯Markdown文件，易迁移、易备份

## 🚀 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem

# 构建所有工具
cargo build --release

# 或单独构建
cargo build --release --bin cortex-mem      # CLI工具
cargo build --release --bin cortex-mem-mcp   # MCP服务器
```

### 配置LLM

创建或编辑 `config.toml`:

```toml
[llm]
# 使用自己部署的OpenAI兼容API
api_base_url = "https://your-api-endpoint.com/v1"
api_key = "your-api-key"
model_efficient = "your-model-name"
temperature = 0.1
max_tokens = 4096
```

支持任何OpenAI兼容的LLM服务（自部署、第三方代理等）。

### CLI使用示例

```bash
# 创建会话
cortex-mem session create my-session --title "技术讨论"

# 添加消息
cortex-mem add --thread my-session "如何实现OAuth 2.0？"
cortex-mem add --thread my-session --role assistant "建议使用授权码流程"

# 搜索记忆
cortex-mem search "OAuth" --thread my-session

# 提取记忆（使用LLM）
cortex-mem session extract my-session

# 查看统计
cortex-mem stats
```

更多CLI示例见 [cortex-mem-cli/TESTING_GUIDE.md](cortex-mem-cli/TESTING_GUIDE.md)

### MCP集成（Claude Desktop）

编辑配置文件: `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "cortex-mem": {
      "command": "/path/to/cortex-mem/target/release/cortex-mem-mcp",
      "args": ["--config", "/path/to/config.toml"],
      "env": {
        "CORTEX_DATA_DIR": "/path/to/cortex-data"
      }
    }
  }
}
```

重启Claude Desktop后，Claude就能使用以下工具：
- `store_memory`: 存储记忆到cortex系统
- `query_memory`: 语义搜索记忆（计划中）
- `list_memories`: 列出指定维度的记忆
- `get_memory`: 根据URI获取记忆内容

更多MCP配置见 [cortex-mem-mcp/README.md](cortex-mem-mcp/README.md)

## 📚 架构概览

### Cortex URI协议

```
cortex://{dimension}/{id}/{category}/{subcategory}/{resource}

示例:
cortex://threads/my-session/timeline/2026-02/03/10_30_45_abc123.md
cortex://user/user-123/memories/abc123.md
cortex://repos/my-project/memories/def456.md
cortex://global/company/policies/security.md
```

### 三层抽象

**L2 - 完整内容层**
```markdown
# 对话记录
User: 如何实现OAuth 2.0？
Assistant: OAuth 2.0是一个授权框架...
[完整对话内容，可能数千tokens]
```

**L1 - 概览层** (~2k tokens)
```markdown
# 概览
本对话讨论OAuth 2.0实现，涵盖授权码流程、安全最佳实践等...

## 关键主题
- OAuth 2.0基础
- 授权码流程
...
```

**L0 - 抽象层** (~100 tokens)
```
OAuth 2.0技术讨论：授权框架、授权码流程、安全实践。
涉及技术点：PKCE、token管理、API设计。
```

### 项目结构

```
cortex-mem/
├── cortex-mem-core/         # 核心库
│   ├── filesystem/          # 文件系统 & URI
│   ├── layers/              # L0/L1/L2抽象
│   ├── retrieval/           # 检索引擎
│   ├── session/             # 会话管理
│   ├── extraction/          # 记忆提取
│   ├── llm/                # LLM客户端
│   └── index/              # 索引（SQLite）
│
├── cortex-mem-cli/          # CLI工具
├── cortex-mem-mcp/          # MCP服务器
├── examples/                # 示例代码
└── config.toml             # 配置文件
```

## 🛠️ 技术栈

### 核心依赖

- **rig-core 0.23** - LLM客户端框架
  - 支持OpenAI兼容API
  - Agent模式支持流式输出
  - 工具调用和多轮对话

- **rmcp 0.14** - Model Context Protocol实现
  - `#[tool]`宏简化工具定义
  - JSON Schema自动生成
  - stdio传输支持

- **tokio** - 异步运行时
- **serde/serde_json** - 序列化
- **rusqlite** - 全文索引
- **chrono** - 时间处理

### 维度系统

Cortex-Mem支持两个维度的记忆组织：

1. **User维度**: `cortex://user/{user_id}/memories/{memory_id}`
   - 用户个人记忆
   - 用户偏好设置
   - 用户特定上下文

2. **Repos维度**: `cortex://repos/{repos_id}/memories/{memory_id}`
   - 项目知识库
   - 代码库文档
   - 团队共享记忆

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行核心库测试
cargo test -p cortex-mem-core

# 查看测试覆盖
cargo test -- --test-threads=1 --nocapture
```

当前测试状态: **55个测试全部通过** ✅

## 📖 使用场景

### 1. AI Agent长期记忆

```rust
use cortex_mem_core::*;
use std::sync::Arc;

let fs = Arc::new(CortexFilesystem::new("./data")?);
let layer_manager = Arc::new(LayerManager::new(fs.clone()));

// 存储对话
let uri = "cortex://threads/session-1/messages/msg-1";
layer_manager.generate_all_layers(uri, content).await?;

// 检索相关记忆
let engine = RetrievalEngine::new(fs, layer_manager);
let results = engine.search("OAuth实现", &options).await?;
```

### 2. 会话管理

```rust
let session_mgr = SessionManager::new(fs, config);

// 创建会话
let session = session_mgr.create_session("thread-1").await?;

// 添加消息
session_mgr.add_message("thread-1", &message).await?;

// 提取记忆
let extractor = MemoryExtractor::new(llm_client);
let memories = extractor.extract_from_session("thread-1").await?;
```

### 3. LLM集成

```rust
use cortex_mem_core::llm::*;

// 创建LLM客户端
let config = LLMConfig {
    api_base_url: "https://your-api.com/v1".to_string(),
    api_key: "your-key".to_string(),
    model_efficient: "gpt-4".to_string(),
    temperature: 0.1,
    max_tokens: 4096,
};

let llm = LLMClient::new(config)?;

// 创建支持流式输出的Agent
let agent = llm.create_agent("You are a helpful assistant").await?;

// 简单completion
let response = llm.complete("Explain OAuth 2.0").await?;
```

## 🔄 版本历史

### V2.0.0 (Current)

**重大重构**：
- ✅ 从Qdrant迁移到基于文件的存储
- ✅ 实现L0/L1/L2三层抽象架构
- ✅ 新增`cortex://` URI协议
- ✅ 集成LLM（基于rig-core）
- ✅ 重写MCP服务器（基于rmcp）
- ✅ 55个测试全部通过
- ✅ 零编译warning

**Breaking Changes**:
- 不再依赖Qdrant向量数据库
- URI格式变更
- MCP工具签名变更

### V1.x (Legacy)

- 基于Qdrant的向量存储
- 基础MCP支持
- 简单的记忆提取

## 🗺️ Roadmap

### 短期计划
- [ ] 完善query_memory和list_memories功能
- [ ] 添加向量嵌入支持（可选）
- [ ] 性能优化和基准测试
- [ ] 更多示例和文档

### 长期计划
- [ ] Web UI界面
- [ ] 多用户支持
- [ ] 分布式部署
- [ ] 更多LLM提供商集成

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启Pull Request

## 📄 许可证

本项目采用MIT许可证 - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [rig](https://github.com/0xPlaygrounds/rig) - Rust LLM框架
- [rmcp](https://github.com/emwalker/rmcp) - Rust MCP实现
- [Model Context Protocol](https://modelcontextprotocol.io/) - MCP标准

## 📧 联系方式

- GitHub Issues: [cortex-mem/issues](https://github.com/sopaco/cortex-mem/issues)
- 项目主页: [cortex-mem](https://github.com/sopaco/cortex-mem)

---

**Built with ❤️ using Rust**
