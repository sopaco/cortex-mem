# Memo-RS - Rust Agent Memory System

一个用 Rust 编写的智能代理记忆系统，提供高性能的记忆存储、检索和管理功能。

## 🚀 功能特性

- **智能记忆管理**: 自动提取、增强和组织对话中的关键信息
- **向量化搜索**: 基于语义相似度的高效记忆检索
- **多种记忆类型**: 支持对话型、程序型和事实型记忆
- **LLM 集成**: 与大语言模型深度集成，提供智能记忆处理
- **多种接口**: 提供 CLI 工具、HTTP API 和 Rig 框架集成
- **高性能**: 基于 Rust 构建，提供出色的性能和内存安全

## 📦 项目结构

```
memo-rs/
├── memo-core/          # 核心记忆管理库
├── memo-cli/           # 命令行工具
├── memo-service/       # HTTP API 服务
├── memo-rig/           # Rig 框架集成工具
└── tests/              # 集成测试
```

## 🛠️ 安装和使用

### 环境要求

- Rust 1.70+
- Qdrant 向量数据库
- OpenAI API 密钥（或兼容的 LLM 服务）

### 环境变量配置

```bash
# LLM 配置
export OPENAI_API_KEY="your-openai-api-key"
export OPENAI_MODEL="gpt-3.5-turbo"
export EMBEDDING_MODEL="text-embedding-ada-002"

# Qdrant 配置
export QDRANT_URL="http://localhost:6334"
export QDRANT_COLLECTION="memories"

# 可选配置
export MAX_TOKENS="1000"
export TEMPERATURE="0.7"
export AUTO_ENHANCE="true"
export DEDUPLICATE="true"
```

### 构建项目

```bash
# 克隆项目
git clone <repository-url>
cd memo-rs

# 构建所有组件
cargo build --release

# 运行测试
cargo test
```

### 使用 CLI 工具

```bash
# 添加记忆
cargo run --bin memo add --content "用户喜欢喝咖啡" --user-id "user123"

# 搜索记忆
cargo run --bin memo search --query "咖啡" --user-id "user123"

# 列出记忆
cargo run --bin memo list --user-id "user123" --limit 10

# 删除记忆
cargo run --bin memo delete <memory-id>
```

### 启动 HTTP 服务

```bash
# 启动服务（默认端口 3000）
cargo run --bin memo-service

# 自定义端口
export PORT=8080
cargo run --bin memo-service
```

### HTTP API 使用示例

```bash
# 健康检查
curl http://localhost:3000/health

# 创建记忆
curl -X POST http://localhost:3000/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "用户喜欢喝咖啡",
    "user_id": "user123",
    "memory_type": "conversational"
  }'

# 搜索记忆
curl -X POST http://localhost:3000/memories/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "咖啡",
    "user_id": "user123",
    "limit": 10
  }'

# 获取记忆
curl http://localhost:3000/memories/<memory-id>

# 列出记忆
curl "http://localhost:3000/memories?user_id=user123&limit=10"
```

## 🔧 配置说明

### 记忆类型

- **Conversational**: 对话型记忆，存储对话上下文和用户交互
- **Procedural**: 程序型记忆，存储操作步骤和流程信息
- **Factual**: 事实型记忆，存储客观事实和知识信息

### 核心配置

```rust
// 记忆配置
MemoryConfig {
    auto_enhance: true,        // 自动增强记忆
    deduplicate: true,         // 去重处理
    auto_summary_threshold: 1000, // 自动摘要阈值
}

// LLM 配置
LLMConfig {
    api_base_url: "https://api.openai.com/v1",
    api_key: "your-api-key",
    model_efficient: "gpt-3.5-turbo",
    max_tokens: 1000,
    temperature: 0.7,
}

// Qdrant 配置
QdrantConfig {
    url: "http://localhost:6334",
    collection_name: "memories",
    embedding_dim: 4096,
    timeout_secs: 30,
}
```

## 🧩 Rig 框架集成

```rust
use memo_rig::{create_memory_tool, MemoryToolConfig};
use std::sync::Arc;

// 创建记忆工具
let memory_tool = create_memory_tool(
    Arc::new(memory_manager),
    Some(MemoryToolConfig {
        default_user_id: Some("user123".to_string()),
        max_search_results: 10,
        auto_enhance: true,
        ..Default::default()
    })
);

// 在 Rig 代理中使用
let agent = client
    .agent("gpt-4")
    .tool(memory_tool)
    .build();
```

## 🏗️ 架构设计

### 核心组件

1. **MemoryManager**: 记忆管理器，提供统一的记忆操作接口
2. **FactExtractor**: 事实提取器，从对话中提取关键信息
3. **MemoryUpdater**: 记忆更新器，处理记忆的合并和更新
4. **VectorStore**: 向量存储，基于 Qdrant 的语义搜索
5. **LLMClient**: LLM 客户端，提供文本生成和嵌入功能

### 数据流

```
输入文本 → 事实提取 → 记忆更新 → 向量化 → 存储
                ↓
搜索查询 → 向量检索 → 相似度排序 → 返回结果
```

## 🧪 测试

```bash
# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test integration_test

# 运行所有测试
cargo test
```

## 📝 开发指南

### 添加新的记忆类型

1. 在 `memo-core/src/types.rs` 中添加新的 `MemoryType` 变体
2. 更新 `FactExtractor` 以支持新类型的事实提取
3. 在 `MemoryUpdater` 中添加相应的处理逻辑

### 扩展 LLM 支持

1. 实现 `LLMClient` trait
2. 在 `memo-core/src/llm/mod.rs` 中注册新的客户端
3. 更新配置结构以支持新的 LLM 提供商

### 添加新的向量存储后端

1. 实现 `VectorStore` trait
2. 在 `memo-core/src/vector_store/mod.rs` 中添加新的实现
3. 更新配置以支持新的存储后端

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！请确保：

1. 代码通过所有测试
2. 遵循 Rust 编码规范
3. 添加适当的文档和测试
4. 更新相关的 README 文档

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 🔗 相关链接

- [Qdrant 文档](https://qdrant.tech/documentation/)
- [OpenAI API 文档](https://platform.openai.com/docs/)
- [Rig 框架](https://github.com/0xPlaygrounds/rig)
- [Rust 官方文档](https://doc.rust-lang.org/)
