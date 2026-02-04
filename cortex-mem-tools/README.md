# Cortex-Mem-Tools

**高级工具库 for Cortex-Mem V2**

提供高级API封装，简化Cortex-Mem核心功能的集成和使用。

---

## 🎯 功能

- ✅ **统一操作接口** - `MemoryOperations`结构体封装所有核心功能
- ✅ **会话管理** - 创建、管理、关闭会话
- ✅ **消息存储** - 添加消息到会话时间轴
- ✅ **智能搜索** - 全文和语义搜索
- ✅ **文件操作** - 读取、列表文件
- ✅ **类型安全** - 完整的类型定义和错误处理

---

## 📦 安装

在你的`Cargo.toml`中添加：

```toml
[dependencies]
cortex-mem-tools = { path = "../cortex-mem-tools" }

# 如果需要向量搜索
[features]
vector-search = ["cortex-mem-tools/vector-search"]
```

---

## 🚀 快速开始

### 基本用法

```rust
use cortex_mem_tools::MemoryOperations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从数据目录创建
    let ops = MemoryOperations::from_data_dir("./cortex-data").await?;
    
    // 添加消息
    let msg_id = ops.add_message(
        "my-session",
        "user",
        "Hello, how are you?"
    ).await?;
    
    println!("Message added: {}", msg_id);
    
    // 搜索消息
    let results = ops.search("Hello", Some("my-session"), 10).await?;
    for memory in results {
        println!("Found: {} (score: {:.2})", 
            memory.uri, 
            memory.score.unwrap_or(0.0)
        );
    }
    
    // 列出会话
    let sessions = ops.list_sessions().await?;
    for session in sessions {
        println!("Session: {} ({})", session.thread_id, session.status);
    }
    
    Ok(())
}
```

### 使用Arc共享

```rust
use std::sync::Arc;
use cortex_mem_tools::MemoryOperations;
use cortex_mem_core::{CortexFilesystem, SessionManager, SessionConfig};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建共享组件
    let filesystem = Arc::new(CortexFilesystem::new("./cortex-data"));
    filesystem.initialize().await?;
    
    let config = SessionConfig::default();
    let session_manager = SessionManager::new(filesystem.clone(), config);
    let session_manager = Arc::new(RwLock::new(session_manager));
    
    // 创建操作接口
    let ops = MemoryOperations::new(filesystem, session_manager);
    
    // 使用操作接口...
    
    Ok(())
}
```

---

## 📖 API文档

### MemoryOperations

核心操作结构体。

#### 创建

```rust
// 从数据目录创建（自动初始化文件系统和会话管理器）
let ops = MemoryOperations::from_data_dir("./cortex-data").await?;

// 从已有组件创建
let ops = MemoryOperations::new(filesystem, session_manager);
```

#### 会话操作

```rust
// 添加消息
let message_id = ops.add_message(thread_id, role, content).await?;

// 列出所有会话
let sessions = ops.list_sessions().await?;

// 获取特定会话
let session = ops.get_session(thread_id).await?;

// 关闭会话
ops.close_session(thread_id).await?;
```

#### 搜索操作

```rust
// 搜索记忆
let results = ops.search(
    "query string",      // 查询
    Some("thread-id"),   // 可选：限定线程
    10                   // 结果数量
).await?;
```

#### 文件操作

```rust
// 读取文件
let content = ops.read_file("cortex://threads/my-session/.session.json").await?;

// 列出文件
let files = ops.list_files("cortex://threads").await?;
```

---

## 🔧 类型定义

### OperationResult<T>

操作结果包装器：

```rust
pub struct OperationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### SessionInfo

会话信息：

```rust
pub struct SessionInfo {
    pub thread_id: String,
    pub status: String,  // "active" | "closed" | "archived"
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### MemoryInfo

记忆信息：

```rust
pub struct MemoryInfo {
    pub uri: String,
    pub content: String,
    pub score: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## ⚠️ 错误处理

```rust
use cortex_mem_tools::{MemoryOperations, ToolsError, Result};

async fn example() -> Result<()> {
    let ops = MemoryOperations::from_data_dir("./data").await?;
    
    match ops.add_message("test", "user", "Hello").await {
        Ok(id) => println!("Success: {}", id),
        Err(ToolsError::NotFound(msg)) => eprintln!("Not found: {}", msg),
        Err(ToolsError::InvalidInput(msg)) => eprintln!("Invalid: {}", msg),
        Err(e) => eprintln!("Error: {}", e),
    }
    
    Ok(())
}
```

---

## 📝 示例

更多示例请查看：
- `examples/` 目录
- `cortex-mem-service` - REST API实现
- `cortex-mem-mcp` - MCP服务器实现
- `cortex-mem-rig` - Rig框架集成

---

## 🤝 贡献

欢迎提交Issue和Pull Request！

---

## 📄 许可证

MIT License - 查看 [LICENSE](../LICENSE) 文件了解详情
