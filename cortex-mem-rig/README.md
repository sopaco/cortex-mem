# Cortex-Mem-Rig

**Simplified Memory Tools for External Integrations**

Cortex-Mem V2的简化集成工具，提供基本的记忆操作功能，无需完整的Rig框架依赖。

> **注意**: 这是V2的简化版本，移除了对`rig-core`的硬依赖。如果需要完整的Rig框架集成，请参考V1版本或自行适配。

---

## 🎯 功能

- ✅ **存储消息** - 将消息保存到会话
- ✅ **查询记忆** - 搜索相关记忆
- ✅ **简化API** - 易于集成到任何Rust项目
- ✅ **无框架依赖** - 不依赖rig-core或其他重型框架

---

## 📦 安装

在你的`Cargo.toml`中添加：

```toml
[dependencies]
cortex-mem-rig = { path = "../cortex-mem-rig" }
cortex-mem-tools = { path = "../cortex-mem-tools" }
```

---

## 🚀 快速开始

### 基本用法

```rust
use cortex_mem_rig::{MemoryTools, StoreMemoryArgs, QueryMemoryArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从数据目录创建工具
    let tools = MemoryTools::from_data_dir("./cortex-data").await?;
    
    // 存储消息
    let store_args = StoreMemoryArgs {
        thread_id: "my-session".to_string(),
        role: "user".to_string(),
        content: "Hello, how can I help you?".to_string(),
    };
    let result = tools.store_memory(store_args).await?;
    println!("{}", result);
    
    // 查询记忆
    let query_args = QueryMemoryArgs {
        query: "help".to_string(),
        thread_id: Some("my-session".to_string()),
        limit: Some(10),
    };
    let result = tools.query_memory(query_args).await?;
    println!("{}", result);
    
    Ok(())
}
```

### 使用共享MemoryOperations

```rust
use cortex_mem_tools::MemoryOperations;
use cortex_mem_rig::{MemoryTools, create_memory_tools};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建共享操作
    let ops = Arc::new(MemoryOperations::from_data_dir("./cortex-data").await?);
    
    // 创建工具
    let tools = create_memory_tools(ops.clone());
    
    // 也可以直接使用ops
    let msg_id = ops.add_message("session", "user", "Hello").await?;
    
    Ok(())
}
```

---

## 📖 API文档

### MemoryTools

主要结构体。

#### 创建

```rust
// 从数据目录创建
let tools = MemoryTools::from_data_dir("./cortex-data").await?;

// 从MemoryOperations创建
let tools = MemoryTools::new(operations);

// 使用create函数
let tools = create_memory_tools(operations);
```

#### 操作

```rust
// 存储消息
let result = tools.store_memory(StoreMemoryArgs {
    thread_id: "session-id".to_string(),
    role: "user".to_string(),
    content: "message content".to_string(),
}).await?;

// 查询记忆
let result = tools.query_memory(QueryMemoryArgs {
    query: "search query".to_string(),
    thread_id: Some("session-id".to_string()),
    limit: Some(10),
}).await?;

// 获取底层操作接口
let ops = tools.operations();
```

### 类型

#### StoreMemoryArgs

```rust
pub struct StoreMemoryArgs {
    pub thread_id: String,    // 会话ID
    pub role: String,          // 角色: "user" | "assistant" | "system"
    pub content: String,       // 消息内容
}
```

#### QueryMemoryArgs

```rust
pub struct QueryMemoryArgs {
    pub query: String,             // 搜索查询
    pub thread_id: Option<String>, // 可选：限定会话
    pub limit: Option<usize>,      // 可选：结果数量（默认10）
}
```

---

## 🔧 与Rig框架集成

虽然这个版本不直接依赖`rig-core`，但你可以轻松集成到Rig框架中：

```rust
use rig::tool::Tool;
use cortex_mem_rig::{MemoryTools, StoreMemoryArgs};
use std::sync::Arc;

// 创建你自己的Tool包装器
struct MyMemoryTool {
    tools: Arc<MemoryTools>,
}

// 实现Rig的Tool trait
impl Tool for MyMemoryTool {
    // ... 实现细节
}
```

---

## 📝 测试

运行测试：

```bash
cargo test -p cortex-mem-rig
```

测试包含：
- 存储消息测试
- 查询记忆测试
- 集成测试

---

## 🆚 与V1的区别

| 特性 | V1 | V2 (简化版) |
|------|----|----|
| Rig框架集成 | ✅ 完整集成 | ❌ 移除依赖 |
| 核心功能 | ✅ | ✅ |
| 独立使用 | ❌ 需要rig-core | ✅ 可独立使用 |
| API复杂度 | 高 | 低 |

**为什么简化？**
- 移除对外部框架的硬依赖
- 提供更灵活的集成方式
- 降低编译时间和二进制大小
- 允许用户自行选择集成方式

---

## 🔄 迁移指南

如果你之前使用V1版本：

```rust
// V1
let tools = MemoryTools::new(memory_manager, config);
let result = tools.store_memory(payload).await?;

// V2
let tools = MemoryTools::from_data_dir("./data").await?;
let result = tools.store_memory(StoreMemoryArgs {
    thread_id: "session".to_string(),
    role: "user".to_string(),
    content: "message".to_string(),
}).await?;
```

主要变化：
1. 不再依赖`MemoryManager`，改用`MemoryOperations`
2. 参数从`payload`改为类型化的`Args`结构体
3. 移除了MCP工具定义（已移至`cortex-mem-mcp`）

---

## 📚 相关项目

- **cortex-mem-tools** - 底层操作库
- **cortex-mem-core** - 核心功能
- **cortex-mem-service** - HTTP REST API
- **cortex-mem-mcp** - Claude Desktop集成

---

## 🤝 贡献

欢迎提交Issue和Pull Request！

如果需要完整的Rig框架支持，请提交Feature Request。

---

## 📄 许可证

MIT License - 查看 [LICENSE](../LICENSE) 文件了解详情
