# 🏢 Cortex Memory 租户隔离架构（最终方案）

## 💡 核心理念

**关键洞察**：在 TARS 这种场景中，每个 bot 就是一个**独立的租户（Tenant）**，租户之间完全隔离，**不需要在 URI 中体现租户 ID**。

### 问题诊断

**之前的设计（过于复杂）**：
```
cortex://threads/{agent_id}/timeline/...
cortex://agents/{agent_id}/memory/...
cortex://users/{user_id}/profile/...
```

**问题**：
- ❌ agent_id 污染了 URI
- ❌ 每个 URI 都要携带 ID
- ❌ 语义混乱（维度 vs 实例）
- ❌ 与 OpenViking 不一致

### 正确的设计（租户模式）

**租户隔离在底层，URI 保持简洁**：

```
cortex://
├── resources/{project}/      # 知识库
├── user/                     # 用户数据
├── agent/                    # Agent 数据
└── session/{session_id}/     # 会话
```

**每个租户看到的都是同样的 URI 结构**，但底层映射到不同的物理路径：

```
# Tenant A 的物理路径
/data/tenants/agent-a/cortex/
├── resources/
├── user/
├── agent/
└── session/

# Tenant B 的物理路径
/data/tenants/agent-b/cortex/
├── resources/
├── user/
├── agent/
└── session/
```

---

## 🎯 完整架构设计（方案一 + 租户隔离）

### URI 结构（完全对齐 OpenViking）

```
cortex://
├── resources/                    # 资源 - 用户添加的知识
│   ├── {project}/
│   │   ├── .abstract.md          # L0 摘要
│   │   ├── .overview.md          # L1 概览
│   │   └── ...                   # L2 完整内容
│   └── ...
│
├── user/                         # 用户 - 单例
│   ├── profile.md                # 用户基本信息
│   └── memories/
│       ├── preferences/          # 用户偏好
│       │   └── {topic}.md
│       ├── entities/             # 实体记忆（人、项目）
│       │   └── {entity}.md
│       └── events/               # 事件记录
│           └── {event}.md
│
├── agent/                        # Agent - 单例
│   ├── skills/                   # 技能
│   │   └── {skill-name}/
│   │       ├── .abstract.md
│   │       ├── SKILL.md
│   │       └── scripts/
│   ├── memories/
│   │   ├── cases/               # 学习的案例
│   │   │   └── {case}.md
│   │   └── patterns/            # 学习的模式
│   │       └── {pattern}.md
│   └── instructions/
│       └── system-prompt.md
│
└── session/                      # 会话 - 多实例
    └── {session_id}/
        ├── .abstract.md          # L0: 会话摘要
        ├── .overview.md          # L1: 会话概览
        ├── .meta.json            # 会话元数据
        └── timeline/
            └── {YYYY-MM}/{DD}/
                └── {HH_MM_SS}_{msg_id}.md
```

---

## 🔧 租户隔离实现

### 1. 底层文件系统映射

```rust
// cortex-mem-core/src/filesystem.rs

pub struct CortexFilesystem {
    base_path: PathBuf,      // 原来的全局根目录
    tenant_id: Option<String>,  // 新增：租户 ID
}

impl CortexFilesystem {
    /// 创建全局实例（无租户隔离）
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            tenant_id: None,
        }
    }
    
    /// 创建租户实例（有租户隔离）
    pub fn with_tenant(base_path: impl Into<PathBuf>, tenant_id: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            tenant_id: Some(tenant_id.into()),
        }
    }
    
    /// 解析 URI 到实际文件路径
    fn resolve_path(&self, uri: &str) -> Result<PathBuf> {
        // 1. 解析 cortex:// URI
        let path = uri.strip_prefix("cortex://")
            .ok_or_else(|| Error::InvalidUri(uri.to_string()))?;
        
        // 2. 如果有租户 ID，添加租户前缀
        let full_path = if let Some(tenant_id) = &self.tenant_id {
            // /data/tenants/{tenant_id}/cortex/{path}
            self.base_path
                .join("tenants")
                .join(tenant_id)
                .join("cortex")
                .join(path)
        } else {
            // /data/cortex/{path}
            self.base_path.join(path)
        };
        
        Ok(full_path)
    }
}
```

**效果**：

```rust
// Tenant A
let fs_a = CortexFilesystem::with_tenant("/data", "agent-a");
fs_a.resolve_path("cortex://user/memories/entities/SkyronJ.md")
// → /data/tenants/agent-a/cortex/user/memories/entities/SkyronJ.md

// Tenant B
let fs_b = CortexFilesystem::with_tenant("/data", "agent-b");
fs_b.resolve_path("cortex://user/memories/entities/SkyronJ.md")
// → /data/tenants/agent-b/cortex/user/memories/entities/SkyronJ.md
```

### 2. MemoryOperations 租户支持

```rust
// cortex-mem-tools/src/lib.rs

pub struct MemoryOperations {
    filesystem: Arc<CortexFilesystem>,      // 已经包含租户信息
    session_manager: Arc<RwLock<SessionManager>>,
    layer_manager: Arc<LayerManager>,
}

impl MemoryOperations {
    /// 创建租户实例
    pub fn with_tenant(
        base_path: impl Into<PathBuf>,
        tenant_id: impl Into<String>,
        llm_client: Option<Arc<dyn LLMClient>>
    ) -> Self {
        let filesystem = Arc::new(CortexFilesystem::with_tenant(base_path, tenant_id));
        let session_manager = Arc::new(RwLock::new(SessionManager::new(filesystem.clone())));
        let layer_manager = if let Some(llm) = llm_client {
            Arc::new(LayerManager::with_llm(filesystem.clone(), llm))
        } else {
            Arc::new(LayerManager::new(filesystem.clone()))
        };
        
        Self {
            filesystem,
            session_manager,
            layer_manager,
        }
    }
}
```

### 3. Rig Tools 租户支持

```rust
// cortex-mem-rig/src/lib.rs

pub struct MemoryTools {
    operations: Arc<MemoryOperations>,  // 已经包含租户信息
}

impl MemoryTools {
    pub fn with_tenant(
        base_path: impl Into<PathBuf>,
        tenant_id: impl Into<String>,
        llm_client: Option<Arc<dyn LLMClient>>
    ) -> Self {
        let operations = Arc::new(MemoryOperations::with_tenant(
            base_path,
            tenant_id,
            llm_client
        ));
        
        Self { operations }
    }
}

/// 创建租户工具（推荐）
pub fn create_memory_tools_with_tenant(
    base_path: impl Into<PathBuf>,
    tenant_id: impl Into<String>,
    llm_client: Option<Arc<dyn LLMClient>>
) -> MemoryTools {
    MemoryTools::with_tenant(base_path, tenant_id, llm_client)
}
```

### 4. TARS 集成

```rust
// examples/cortex-mem-tars/src/agent.rs

pub async fn create_memory_agent(
    base_path: impl Into<PathBuf>,
    agent_id: &str,  // 作为 tenant_id
    user_id: &str,
    user_info: Option<String>,
    bot_system_prompt: Option<String>,
    llm_client: Arc<dyn LLMClient>,
) -> Result<RigAgent<CompletionModel>, Box<dyn std::error::Error>> {
    
    // 创建租户工具（agent_id 作为 tenant_id）
    let memory_tools = create_memory_tools_with_tenant(
        base_path,
        agent_id,  // ✅ 租户 ID
        Some(llm_client.clone())
    );
    
    // 工具中的所有 URI 都不需要包含 agent_id
    // 例如：cortex://user/memories/entities/SkyronJ.md
    // 底层自动映射到：/data/tenants/{agent_id}/cortex/user/memories/entities/SkyronJ.md
    
    // ...
}
```

---

## 📊 使用示例

### 示例1：存储用户实体记忆

```rust
// TARS Agent A（tenant_id = "agent-a"）
let tools = create_memory_tools_with_tenant("/data", "agent-a", Some(llm));

// 存储关于 SkyronJ 的记忆
let store_tool = tools.store_tool();
store_tool.call(StoreArgs {
    content: "SkyronJ 是我的前任领导...".to_string(),
    thread_id: "".to_string(),  // 空字符串，工具内部会处理
    metadata: Some(json!({
        "type": "entity",
        "entity_name": "SkyronJ"
    })),
    auto_generate_layers: Some(true),
}).await?;

// 实际存储路径（用户不可见）：
// /data/tenants/agent-a/cortex/user/memories/entities/SkyronJ.md

// URI（用户可见，简洁）：
// cortex://user/memories/entities/SkyronJ.md
```

### 示例2：搜索记忆

```rust
// TARS Agent A 搜索
let search_tool = tools.search_tool();
search_tool.call(SearchArgs {
    query: "SkyronJ".to_string(),
    scope: Some("cortex://user/memories/".to_string()),  // ✅ 简洁的 URI
    engine: Some("keyword".to_string()),
    return_layers: Some(vec!["L0".to_string()]),
    ..Default::default()
}).await?;

// 底层自动在租户空间搜索：
// /data/tenants/agent-a/cortex/user/memories/
```

### 示例3：多租户隔离

```rust
// Agent A 和 Agent B 同时运行

// Agent A（租户 A）
let tools_a = create_memory_tools_with_tenant("/data", "agent-a", Some(llm));
tools_a.store_tool().call(StoreArgs {
    content: "Agent A 的记忆".to_string(),
    // ...
}).await?;
// → /data/tenants/agent-a/cortex/user/memories/...

// Agent B（租户 B）
let tools_b = create_memory_tools_with_tenant("/data", "agent-b", Some(llm));
tools_b.store_tool().call(StoreArgs {
    content: "Agent B 的记忆".to_string(),
    // ...
}).await?;
// → /data/tenants/agent-b/cortex/user/memories/...

// ✅ 完全物理隔离，URI 简洁一致
```

---

## 🎊 方案优势

### 1. URI 简洁清晰

**之前**：
```
cortex://threads/{agent_id}/timeline/2026-02/09/msg.md
cortex://agents/{agent_id}/memories/cases/case1.md
```

**现在**：
```
cortex://session/{session_id}/timeline/2026-02/09/msg.md
cortex://agent/memories/cases/case1.md
```

### 2. 完全对齐 OpenViking

```
OpenViking:     viking://resources/
Cortex Memory:  cortex://resources/

OpenViking:     viking://user/memories/
Cortex Memory:  cortex://user/memories/

OpenViking:     viking://agent/skills/
Cortex Memory:  cortex://agent/skills/

OpenViking:     viking://session/{id}/
Cortex Memory:  cortex://session/{id}/
```

✅ **完美对齐！**

### 3. 租户隔离在底层

```
用户/Agent 视角（逻辑 URI）：
  cortex://user/memories/entities/SkyronJ.md

底层实现（物理路径）：
  /data/tenants/agent-a/cortex/user/memories/entities/SkyronJ.md
  /data/tenants/agent-b/cortex/user/memories/entities/SkyronJ.md
```

✅ **关注点分离！**

### 4. 代码简洁

**创建工具**：
```rust
// 一行代码指定租户
let tools = create_memory_tools_with_tenant("/data", agent_id, Some(llm));
```

**使用工具**：
```rust
// URI 中不需要 agent_id
search_tool.call(SearchArgs {
    scope: Some("cortex://user/memories/".to_string()),  // ✅ 简洁
    // ...
}).await?;
```

---

## 📂 文件系统布局

### 租户隔离的物理结构

```
/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/
└── tenants/
    ├── agent-a/                          # Tenant A
    │   └── cortex/
    │       ├── resources/
    │       │   └── rust-docs/
    │       ├── user/
    │       │   ├── profile.md
    │       │   └── memories/
    │       │       ├── preferences/
    │       │       ├── entities/
    │       │       │   └── SkyronJ.md
    │       │       └── events/
    │       ├── agent/
    │       │   ├── skills/
    │       │   └── memories/
    │       └── session/
    │           └── {session_id}/
    │               └── timeline/
    │
    └── agent-b/                          # Tenant B
        └── cortex/
            ├── resources/
            ├── user/
            ├── agent/
            └── session/
```

**特点**：
- ✅ 每个租户完全独立
- ✅ 物理隔离，安全性高
- ✅ 易于备份和迁移（整个租户目录）
- ✅ 易于清理（删除整个租户目录）

---

## 🔄 迁移路径

### 从当前架构迁移

**当前**：
```
/data/cortex/
└── threads/
    └── {agent_id}/
        └── timeline/
```

**目标**：
```
/data/tenants/
└── {agent_id}/
    └── cortex/
        ├── resources/
        ├── user/
        ├── agent/
        └── session/
            └── default/          # 将旧的 timeline 迁移到默认 session
                └── timeline/
```

**迁移脚本**：
```rust
pub async fn migrate_to_tenant_model(
    old_base: &Path,
    new_base: &Path,
    agent_id: &str
) -> Result<()> {
    // 1. 创建租户目录
    let tenant_dir = new_base.join("tenants").join(agent_id).join("cortex");
    tokio::fs::create_dir_all(&tenant_dir).await?;
    
    // 2. 迁移 threads/{agent_id}/* → session/default/*
    let old_thread = old_base.join("threads").join(agent_id);
    let new_session = tenant_dir.join("session").join("default");
    if old_thread.exists() {
        copy_dir_all(&old_thread, &new_session).await?;
    }
    
    // 3. 创建空的 resources、user、agent 目录
    tokio::fs::create_dir_all(tenant_dir.join("resources")).await?;
    tokio::fs::create_dir_all(tenant_dir.join("user").join("memories")).await?;
    tokio::fs::create_dir_all(tenant_dir.join("agent").join("skills")).await?;
    
    Ok(())
}
```

---

## 🎯 最终推荐

**对于 TARS**：

1. ✅ **采用方案一（OpenViking 风格）+ 租户隔离**
2. ✅ **URI 简洁**：不包含 tenant_id
3. ✅ **底层隔离**：通过 `CortexFilesystem::with_tenant()`
4. ✅ **语义清晰**：resources、user、agent、session

**核心改动**：

```rust
// 1. CortexFilesystem 支持租户
let fs = CortexFilesystem::with_tenant("/data", agent_id);

// 2. MemoryOperations 使用租户文件系统
let ops = MemoryOperations::with_tenant("/data", agent_id, Some(llm));

// 3. TARS 创建租户工具
let tools = create_memory_tools_with_tenant("/data", agent_id, Some(llm));

// 4. URI 保持简洁
// cortex://user/memories/entities/SkyronJ.md
// cortex://session/{session_id}/timeline/...
```

---

## 📊 对比总结

| 方面 | 之前的设计 | 租户隔离方案 |
|------|-----------|------------|
| **URI 复杂度** | `cortex://threads/{agent_id}/...` | `cortex://session/{session_id}/...` |
| **隔离方式** | URI 中包含 ID | 底层物理隔离 |
| **OpenViking 对齐** | ❌ 不一致 | ✅ 完全一致 |
| **代码简洁性** | 到处传 agent_id | 创建时指定一次 |
| **可维护性** | 复杂 | 简单 |
| **安全性** | 中等（逻辑隔离） | 高（物理隔离） |

---

**方案创建时间**: 2026-02-09 15:55  
**核心理念**: 租户隔离 + OpenViking 对齐  
**推荐度**: ⭐⭐⭐⭐⭐  
**适用场景**: TARS 及所有类似的多租户 Agent 系统

**这就是最终的、最简洁的、最合理的方案！** 🎉
