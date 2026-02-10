# 📚 L0/L1/L2 分层加载机制详解

## 🎯 核心概念

**L0/L1/L2** 是 Cortex Memory 的三层分层加载（Tiered Loading）机制，灵感来自 OpenViking 设计。目标是通过渐进式加载优化 Token 效率。

---

## 📁 文件系统体现

### 完整的文件结构

对于每个记忆文件，系统会创建以下三个文件：

```
timeline/2026-02/09/
├── 09_25_56_513eb12b.md    # L2 - 完整内容（原始文件）
├── .abstract.md             # L0 - 简短摘要（~100 tokens）
└── .overview.md             # L1 - 详细概览（~500-2000 tokens）
```

### 具体说明

| Layer | 文件名 | Token 数 | 用途 | 生成方式 |
|-------|--------|---------|------|---------|
| **L2 Detail** | `09_25_56_513eb12b.md` | 完整 | 原始内容，完整信息 | 直接存储 |
| **L1 Overview** | `.overview.md` | ~500-2000 | 核心信息概览 | LLM 生成 |
| **L0 Abstract** | `.abstract.md` | ~100 | 快速相关性检查 | LLM 生成 |

---

## 🔄 生成机制

### 1. 存储时（Store 工具）

当调用 `store` 工具存储记忆时：

```rust
pub async fn store(&self, args: StoreArgs) -> Result<StoreResponse> {
    // 1. 写入 L2 原始文件
    self.filesystem.write(uri, content).await?;
    
    // 2. 如果有 LLM，自动生成 L0/L1
    if let Some(llm) = &self.llm_client {
        // 生成 L0 Abstract
        let abstract_text = generate_abstract(content, llm).await?;
        self.filesystem.write(".abstract.md", &abstract_text).await?;
        
        // 生成 L1 Overview
        let overview = generate_overview(content, llm).await?;
        self.filesystem.write(".overview.md", &overview).await?;
    }
}
```

**关键点**：
- ✅ L2 总是立即写入
- ⚪ L0/L1 仅在有 LLM 客户端时生成
- ⚪ 如果没有 LLM，L0/L1 会在首次访问时按需生成（基于规则）

### 2. 读取时（按需生成）

当调用 `abstract`/`overview` 工具时：

```rust
async fn load_abstract(&self, uri: &str) -> Result<String> {
    let abstract_uri = ".abstract.md";
    
    // 如果 .abstract.md 存在，直接读取
    if self.filesystem.exists(abstract_uri).await? {
        return self.filesystem.read(abstract_uri).await;
    }
    
    // 否则，从 L2 生成
    let detail = self.load_detail(uri).await?;
    let abstract_text = generate_abstract(&detail).await?;
    
    // 保存供未来使用
    self.filesystem.write(abstract_uri, &abstract_text).await?;
    
    Ok(abstract_text)
}
```

**Lazy Generation（懒加载）**：
- 首次访问时才生成
- 生成后缓存到文件
- 下次直接读取，不重复生成

---

## 🎭 当前数据状态分析

### 你的数据目录

```bash
session/skyronj_memory_onboarding_v1/timeline/2026-02/09/
└── 09_25_56_513eb12b.md    # ✅ L2 文件（存在）
```

**观察**：
- ✅ **L2** 文件存在（591 字节）
- ❌ **L0** (`.abstract.md`) 不存在
- ❌ **L1** (`.overview.md`) 不存在

### 原因分析

**为什么 L0/L1 没有生成？**

可能的原因：
1. **LLM 客户端未配置**
   - TARS 的 `create_memory_agent` 可能没有传递 LLM 客户端给 `LayerManager`
   - 代码中 `llm_client: Option<Arc<dyn LLMClient>>` 为 `None`

2. **Store 工具未启用自动生成**
   - 当前 `store` 只写入了 L2 文件
   - 没有调用 `generate_all_layers`

3. **按需生成未触发**
   - L0/L1 只在首次访问时生成
   - 你还没有调用 `abstract` 或 `overview` 工具

### 验证方法

让我检查一下 TARS 的存储逻辑：

```bash
# 1. 检查是否调用了 abstract 工具
# （在 TARS 中与 Bot 对话，说："给我这段记忆的摘要"）

# 2. 查看生成的文件
tree ~/Library/Application\ Support/com.cortex-mem.tars/cortex/tenants/.../timeline/2026-02/09/

# 预期结果：
# ├── 09_25_56_513eb12b.md
# ├── .abstract.md         # ← 应该出现
# └── .overview.md         # ← 应该出现
```

---

## 🛠️ 如何生成 L0/L1 文件

### 方法1：使用 abstract 工具

在 TARS 中与 Bot 对话：

```
用户: "给我这段记忆的摘要"
或
用户: "总结一下我们刚才的对话"
```

**效果**：
- Bot 会调用 `abstract` 工具
- 系统自动生成 `.abstract.md` 文件
- 返回 L0 摘要给用户

### 方法2：使用 overview 工具

```
用户: "详细概述一下这段记忆"
```

**效果**：
- Bot 会调用 `overview` 工具
- 系统自动生成 `.overview.md` 文件
- 返回 L1 概览给用户

### 方法3：使用 search 工具

```rust
// Search 工具可以指定返回的层级
SearchArgs {
    query: "用户信息",
    return_layers: Some(vec!["L0".to_string()]),  // 请求 L0
    ...
}
```

**效果**：
- 搜索时自动加载 L0 层
- 如果不存在，自动生成

---

## 📊 Token 效率对比

### 场景：搜索 20 个记忆

**老方案（无分层）**：
```
20 × 5000 tokens (完整内容) = 100,000 tokens
```

**新方案（分层加载）**：
```
Step 1: 扫描 L0
20 × 100 tokens = 2,000 tokens

Step 2: 筛选相关记忆（假设 3 个）
3 × 2000 tokens (L1) = 6,000 tokens

Step 3: 深入阅读（假设 1 个）
1 × 5000 tokens (L2) = 5,000 tokens

总计: 2,000 + 6,000 + 5,000 = 13,000 tokens
节省: 87%
```

---

## 🔍 完整示例

### 存储记忆

**操作**：
```bash
# Store 工具调用
store(
    uri: "cortex://session/abc/timeline/2026-02/09/10_00_00_xxx.md",
    content: "用户提到了 OAuth 2.0 的安全问题..."
)
```

**生成的文件**：
```
timeline/2026-02/09/
└── 10_00_00_xxx.md        # L2 - 591 字节
```

### 首次访问 Abstract

**操作**：
```bash
# Abstract 工具调用
abstract(uri: "cortex://session/abc/timeline/2026-02/09/10_00_00_xxx.md")
```

**系统行为**：
1. 检查 `.abstract.md` 是否存在 → ❌ 不存在
2. 读取 `10_00_00_xxx.md` 内容
3. 调用 LLM 或规则生成摘要
4. 写入 `.abstract.md`
5. 返回摘要内容

**生成的文件**：
```
timeline/2026-02/09/
├── 10_00_00_xxx.md        # L2
└── .abstract.md           # L0 - 新生成！
```

### 再次访问 Abstract

**操作**：
```bash
abstract(uri: "cortex://session/abc/timeline/2026-02/09/10_00_00_xxx.md")
```

**系统行为**：
1. 检查 `.abstract.md` 是否存在 → ✅ 存在
2. 直接读取并返回 → **超快！**

---

## 🎯 实际文件内容示例

假设你的记忆内容是：

### L2 Detail (`09_25_56_513eb12b.md`)

```markdown
用户SkyronJ，前任领导兼朋友，INTJ人格向ENTJ转型中。
重视效率与创意，关注团队整体业绩与项目影响力。
技术专长为Rust，职业目标是成为更高层级的技术领导者，
希望在团队中扮演教练、布道师、架构师等多重角色。
曾在我（杨雪）面临组织优化时作为中间人与HRBP沟通，
争取到协商解除协议并保留年终奖。我随后通过内部活水
进入工程效率部门，继续留在快手，但与SkyronJ不再同部门、
不同办公地。我们曾是饭友，关系良好，建立过优质友情。
```

### L1 Overview (`.overview.md`) - 如果生成

```markdown
## 用户概览

**身份**: SkyronJ，前领导，现朋友  
**人格**: INTJ → ENTJ 转型  
**技能**: Rust 技术专长  
**目标**: 高级技术领导者（教练/布道师/架构师）

## 关键事件

- 组织优化期间协助杨雪协商离职
- 争取到协商解除协议 + 年终奖
- 杨雪内部活水至工程效率部门

## 关系状态

- 曾是同事，现为朋友
- 不同部门、不同办公地
- 建立过优质友情
```

### L0 Abstract (`.abstract.md`) - 如果生成

```markdown
用户SkyronJ：前领导转朋友，INTJ→ENTJ，Rust专家，
目标高级技术领导。曾协助杨雪协商离职，保留年终奖。
现不同部门，关系良好。
```

---

## 🎓 设计哲学

### 为什么需要三层？

1. **L0 (Abstract)**：快速扫描
   - 在大量记忆中快速判断相关性
   - 类似于"标题"或"标签"
   - Token 开销极小

2. **L1 (Overview)**：理解核心
   - 获取足够信息做决策
   - 不需要完整细节
   - 平衡信息量和 Token

3. **L2 (Detail)**：深入阅读
   - 只在真正需要时加载
   - 完整信息，无损失
   - 最大 Token 开销

### 渐进式加载

```
用户查询 → 搜索
    ↓
L0 扫描（20 个记忆）→ 初步筛选
    ↓
L1 理解（3 个相关）→ 深入评估
    ↓
L2 详读（1 个最相关）→ 完整信息
```

---

## 📝 总结

### L0/L1/L2 在文件系统中的体现

| Layer | 文件 | 状态 | 何时生成 |
|-------|------|------|---------|
| **L2** | `{timestamp}_{hash}.md` | ✅ 总是存在 | Store 时立即创建 |
| **L1** | `.overview.md` | ⚪ 按需生成 | 首次访问 overview 时 |
| **L0** | `.abstract.md` | ⚪ 按需生成 | 首次访问 abstract 时 |

### 当前你的数据

```
timeline/2026-02/09/
└── 09_25_56_513eb12b.md    # ✅ L2 存在
    .abstract.md            # ❌ 未生成（未访问）
    .overview.md            # ❌ 未生成（未访问）
```

### 如何生成 L0/L1

1. 在 TARS 中与 Bot 对话："给我这段记忆的摘要"
2. 或："详细概述一下"
3. 或：使用 search 工具时指定 `return_layers: ["L0"]`

### Token 效率

- **无分层**：100,000 tokens
- **分层加载**：8,000-13,000 tokens
- **节省**：80-92%

---

**文档创建时间**：2026-02-09 17:35  
**关键概念**：L0/L1/L2 分层加载、按需生成、Token 优化  
**实现状态**：✅ 核心机制已实现，等待触发
