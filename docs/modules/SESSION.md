# 会话管理模块 (Session)

**模块路径**: `cortex-mem-core/src/session/`  
**职责**: 会话生命周期管理、消息存储、Timeline组织

---

## 核心组件

### SessionManager

```rust
pub struct SessionManager {
    filesystem: Arc<CortexFilesystem>,
    message_storage: MessageStorage,
    participant_manager: ParticipantManager,
}
```

**主要方法**:
- `create_session()` - 创建新会话
- `load_session()` - 加载会话元数据
- `close_session()` - 关闭会话
- `update_session()` - 更新会话信息

### MessageStorage

```rust
pub struct MessageStorage {
    filesystem: Arc<CortexFilesystem>,
}
```

**主要方法**:
- `save_message()` - 保存消息到Timeline
- `load_message()` - 加载单条消息
- `list_messages()` - 列出会话所有消息

### SessionMetadata

```rust
pub struct SessionMetadata {
    pub thread_id: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub participants: Vec<String>,
}
```

## 会话生命周期

```
Created → Active → Closed → Archived
```

### 状态转换

| 当前状态 | 操作 | 新状态 |
|---------|------|--------|
| - | create | Active |
| Active | add_message | Active |
| Active | close | Closed |
| Closed | archive | Archived |

## Timeline组织

### 目录结构

```
threads/{thread_id}/
├── .session.json          # 元数据
└── timeline/              # 时间线
    └── 2026-02/          # 年-月
        ├── 04/           # 日
        │   ├── 16_30_00_abc12345.md
        │   └── 16_31_00_def67890.md
        └── 05/
            └── 10_00_00_ghi24680.md
```

### 消息命名规则

```
{HH}_{MM}_{SS}_{message_id_short}.md
```

- HH: 小时（24小时制）
- MM: 分钟
- SS: 秒
- message_id_short: UUID前8位

## 消息格式

```markdown
# Message

**ID**: abc12345-1234-5678-90ab-cdef12345678  
**Role**: user  
**Timestamp**: 2026-02-04T16:30:00Z

## Content

User message content here.

## Metadata

```json
{
  "source": "cli",
  "version": "2.0.0"
}
```
```

## 自动化触发

会话关闭时可触发：
1. 记忆提取（LLM分析）
2. 索引更新
3. 归档操作

---

详见源码: [cortex-mem-core/src/session/](../../cortex-mem-core/src/session/)
