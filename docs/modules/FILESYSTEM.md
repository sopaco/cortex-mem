# 文件系统模块 (Filesystem)

**模块路径**: `cortex-mem-core/src/filesystem/`  
**职责**: cortex:// 虚拟URI协议实现和文件操作

---

## 核心接口

```rust
pub trait FilesystemOperations {
    async fn read(&self, uri: &str) -> Result<String>;
    async fn write(&self, uri: &str, content: &str) -> Result<()>;
    async fn list(&self, uri: &str) -> Result<Vec<FileEntry>>;
    async fn delete(&self, uri: &str) -> Result<()>;
    async fn exists(&self, uri: &str) -> Result<bool>;
}
```

## URI规范

### URI格式

```
cortex://{namespace}/{path}
```

### 命名空间

| 命名空间 | 用途 | 示例 |
|---------|------|------|
| `threads/` | 会话数据 | `cortex://threads/my-chat/timeline/` |
| `users/` | 用户记忆 | `cortex://users/user123/memories/` |
| `agents/` | Agent记忆 | `cortex://agents/agent456/memories/` |
| `index/` | 索引数据 | `cortex://index/fulltext/` |

### Timeline路径结构

```
cortex://threads/{thread_id}/timeline/{YYYY-MM}/{DD}/{HH_MM_SS}_{message_id}.md
```

**示例**:
```
cortex://threads/my-chat/timeline/2026-02/04/16_30_00_abc12345.md
```

## 实现细节

### CortexFilesystem

```rust
pub struct CortexFilesystem {
    base_dir: PathBuf,
}

impl CortexFilesystem {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self
    pub async fn initialize(&self) -> Result<()>
    fn resolve_uri(&self, uri: &str) -> Result<PathBuf>
}
```

### URI解析流程

```
cortex://threads/abc/timeline/2026-02/04/file.md
    ↓
去除cortex://前缀
    ↓
threads/abc/timeline/2026-02/04/file.md
    ↓
拼接base_dir
    ↓
/path/to/cortex-data/threads/abc/timeline/2026-02/04/file.md
```

## 文件格式

### 消息文件 (.md)

```markdown
# Message

**ID**: abc12345  
**Role**: user  
**Timestamp**: 2026-02-04T16:30:00Z

## Content

This is the message content.
```

### 会话元数据 (.session.json)

```json
{
  "thread_id": "my-chat",
  "status": "Active",
  "created_at": "2026-02-04T16:00:00Z",
  "updated_at": "2026-02-04T16:30:00Z",
  "message_count": 5
}
```

## 性能优化

1. **异步I/O**: 所有操作使用tokio async
2. **路径缓存**: 缓存URI解析结果（未来）
3. **批量操作**: 支持batch read/write（未来）

## 错误处理

```rust
pub enum Error {
    NotFound { uri: String },
    InvalidUri { uri: String },
    IoError(std::io::Error),
    Permission { uri: String },
}
```

---

详见源码: [cortex-mem-core/src/filesystem/](../../cortex-mem-core/src/filesystem/)
