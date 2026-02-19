# Cortex Memory Rig Integration

`cortex-mem-rig` 提供与 [Rig](https://github.com/coreylowman/rig) AI 框架的集成，使 AI 代理能够通过工具调用与 Cortex Memory 系统进行交互。

## 🧠 概述

Cortex Memory Rig 实现了三层架构访问工具，允许 AI 代理高效地检索和操作记忆：

### 三层访问架构

| 层级 | 大小 | 用途 | 工具 |
|------|------|------|------|
| **L0 Abstract** | ~100 tokens | 快速相关性判断 | `abstract_tool` |
| **L1 Overview** | ~500-2000 tokens | 部分上下文理解 | `overview_tool` |
| **L3 Full** | 完整内容 | 深度分析和处理 | `read_tool` |

### 核心工具集

- 📊 **分层访问工具**: `abstract()`, `overview()`, `read()`
- 🔍 **搜索工具**: `search()`, `find()`
- 📁 **文件系统工具**: `ls()`, `explore()`, `store()`

## 🚀 快速开始

### 安装

```toml
[dependencies]
cortex-mem-rig = { path = "../cortex-mem-rig" }
cortex-mem-tools = { path = "../cortex-mem-tools" }
rig-core = "0.31"
```

### 基本使用

```rust
use cortex_mem_rig::MemoryTools;
use cortex_mem_tools::MemoryOperations;
use rig::agents::Agent;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建记忆操作
    let operations = Arc::new(MemoryOperations::from_data_dir("./cortex-data").await?);
    
    // 创建 Rig 工具集
    let memory_tools = MemoryTools::new(operations);
    
    // 创建 agent 并附加工具
    let agent = Agent::new("gpt-4o-mini")
        .preamble("你是一个具有持久记忆的 AI 助手。")
        .tool(memory_tools.abstract_tool())
        .tool(memory_tools.overview_tool())
        .tool(memory_tools.search_tool())
        .build();
    
    // 使用 agent...
    
    Ok(())
}
```

## 📚 API 参考

### MemoryTools

主要的工具集合类，提供对不同层级工具的访问。

```rust
impl MemoryTools {
    pub fn new(operations: Arc<MemoryOperations>) -> Self

    // 三层访问工具
    pub fn abstract_tool(&self) -> AbstractTool
    pub fn overview_tool(&self) -> OverviewTool
    pub fn read_tool(&self) -> ReadTool
    
    // 搜索工具
    pub fn search_tool(&self) -> SearchTool
    pub fn find_tool(&self) -> FindTool
    
    // 文件系统工具
    pub fn ls_tool(&self) -> LsTool
    pub fn explore_tool(&self) -> ExploreTool
    pub fn store_tool(&self) -> StoreTool
    
    // 获取底层操作
    pub fn operations(&self) -> &Arc<MemoryOperations>
}
```

### 分层访问工具

#### AbstractTool

获取内容的 L0 抽象摘要（约 100 tokens），用于快速判断相关性。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct AbstractArgs {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AbstractResponse {
    pub uri: String,
    pub abstract_text: String,
    pub layer: String,
    pub token_count: usize,
}

impl Tool for AbstractTool {
    const NAME: &'static str = "abstract";
    // ...
}
```

**示例使用**:
```rust
let result = agent.prompt(
    "获取cortex://users/user-123/preferences.md的摘要"
).await?;
```

#### OverviewTool

获取内容的 L1 概览（约 500-2000 tokens），用于部分上下文理解。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct OverviewArgs {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OverviewResponse {
    pub uri: String,
    pub overview_text: String,
    pub layer: String,
    pub token_count: usize,
    pub sections: Vec<String>,
}

impl Tool for OverviewTool {
    const NAME: &'static str = "overview";
    // ...
}
```

#### ReadTool

获取完整内容（L3），用于深度分析。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadArgs {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResponse {
    pub uri: String,
    pub content: String,
    pub layer: String,
    pub token_count: usize,
    pub sections: Vec<String>,
}

impl Tool for ReadTool {
    const NAME: &'static str = "read";
    // ...
}
```

### 搜索工具

#### SearchTool

执行智能搜索，支持多种模式。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    pub thread: Option<String>,
    pub scope: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub uri: String,
    pub snippet: String,
    pub score: f32,
    pub abstract_text: Option<String>,
}

impl Tool for SearchTool {
    const NAME: &'static str = "search";
    // ...
}
```

#### FindTool

查找特定类型的记忆或内容。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct FindArgs {
    pub query: String,
    pub filters: Option<FindFilters>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindFilters {
    pub dimensions: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub date_range: Option<DateRange>,
}

impl Tool for FindTool {
    const NAME: &'static str = "find";
    // ...
}
```

### 文件系统工具

#### LsTool

列出目录内容。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct LsArgs {
    pub uri: String,
    pub recursive: Option<bool>,
    pub include_abstracts: Option<bool>,
}

impl Tool for LsTool {
    const NAME: &'static str = "ls";
    // ...
}
```

#### ExploreTool

探索结构化的记忆内容。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ExploreArgs {
    pub uri: String,
    pub depth: Option<usize>,
    pub filters: Option<ExploreFilters>,
}

impl Tool for ExploreTool {
    const NAME: &'static str = "explore";
    // ...
}
```

#### StoreTool

存储新记忆。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreArgs {
    pub content: String,
    pub thread_id: Option<String>,
    pub role: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl Tool for StoreTool {
    const NAME: &'static str = "store";
    // ...
}
```

## 🛠️ Agent 集成

### 完整示例

```rust
use rig::providers::openai::{Client, completion::CompletionModel};
use cortex_mem_rig::MemoryTools;
use cortex_mem_tools::MemoryOperations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 OpenAI 客户端
    let openai_client = Client::from_env()?;
    
    // 创建记忆操作
    let operations = Arc::new(MemoryOperations::from_data_dir("./cortex-data").await?);
    
    // 创建工具集
    let memory_tools = MemoryTools::new(operations);
    
    // 创建 Agent
    let agent = openai_client
        .completion_model(CompletionModel::Gpt4Omini)
        .agent("记忆助手")
        .preamble("你是一个具有长期记忆的 AI 助手。你可以存储和检索用户信息。")
        .tool(memory_tools.abstract_tool())
        .tool(memory_tools.overview_tool())
        .tool(memory_tools.read_tool())
        .tool(memory_tools.search_tool())
        .tool(memory_tools.store_tool())
        .build();
    
    // 对话示例
    let response = agent.prompt(
        "请先搜索关于用户偏好的信息，然后存储用户喜欢使用深色主题的偏好。"
    ).await?;
    
    println!("Agent 响应: {}", response);
    
    Ok(())
}
```

### 链式工具调用

```rust
// Agent 会自动进行链式调用
let response = agent.prompt(
    "1. 搜索用户之前关于编程语言偏好的讨论\n\
     2. 获取最相关讨论的概览\n\
     3. 如果需要，读取完整内容\n\
     4. 基于结果提供个性化建议"
).await?;
```

## 🎯 最佳实践

### 分层访问模式

1. **首先使用 abstract()** 快速判断相关性
2. **如果相关，使用 overview()** 获取更多上下文
3. **仅在必要时使用 read()** 获取完整内容

```rust
// Agent 的内部思考模式可能如下：
// 1. 用户询问关于 Rust 的问题
// 2. 搜索 "Rust programming"
// 3. 对每个结果使用 abstract() 检查相关性
// 4. 对相关的使用 overview() 获取更多上下文
// 5. 对最终需要的文档使用 read() 获取完整内容
```

### 搜索优化

```rust
// 限定搜索范围
agent.prompt("在 'tech-discussions' 会话中搜索 Rust 相关内容").await?;

// 使用精确查询
agent.prompt("查找与 'async/await' 相关的具体实现示例").await?;

// 结合分层访问
agent.prompt(
    "搜索 '错误处理'，对前3个结果获取摘要，然后对最相关的获取概览"
).await?;
```

## 🔧 高级配置

### 自定义工具

```rust
use cortex_mem_rig::tools::AbstractTool;

impl AbstractTool {
    pub fn with_custom_token_limit(operations: Arc<MemoryOperations>, limit: usize) -> Self {
        // 自定义 token 限制
        Self { operations, token_limit: Some(limit) }
    }
}
```

### 工具组合

```rust
// 创建专门的工具组合
let retrieval_tools = MemoryToolsBuilder::new(operations)
    .with_tiered_access()   // L0, L1, L3 工具
    .with_search()          // 搜索工具
    .with_filesystem()      // 文件系统工具
    .build();

let write_tools = MemoryToolsBuilder::new(operations)
    .with_store()           // 存储工具
    .with_search()          // 用于验证的搜索
    .build();
```

## 🧪 测试

```bash
# 运行 Rig 集成测试
cargo test -p cortex-mem-rig

# 运行工具测试
cargo test -p cortex-mem-rig tools

# 运行端到端测试
cargo test -p cortex-mem-rig e2e
```

## 🚨 常见问题

### 1. 工具调用失败

确保：
- Cortex Memory 核心正确初始化
- 数据目录具有写权限
- 搜索索引已建立

### 2. 抽象内容为空

可能原因：
- 文件不存在
- 内容过短无法生成摘要
- LLM 服务不可用

### 3. 搜索结果不准确

优化方法：
- 使用更精确的查询
- 限定搜索范围
- 使用 find 工具而非 search

## 🛣️ 路线图

- [ ] 流式访问工具（适用于大文件）
- [ ] 缓存层优化
- [ ] 工具调用统计
- [ ] 自动工具选择
- [ ] 多模态记忆支持

## 📄 许可证

MIT 许可证 - 详见 [LICENSE](../../LICENSE) 文件

## 🔗 相关资源

- [Cortex Memory 核心](../cortex-mem-core/README.md)
- [Cortex Memory 工具](../cortex-mem-tools/README.md)
- [Rig 框架](https://github.com/coreylowman/rig)
- [Rig 文档](https://docs.rs/rig/)

---

**Built with ❤️ using Rust and Rig AI Framework**