# 🔧 工具调用失败问题分析与修复

## 🐛 问题现象

```
[2026-02-09 17:40:37.743 ERROR] 流式处理错误: Completion(RequestError("Failed to get tool definitions"))
```

**错误类型**：`RequestError("Failed to get tool definitions")`  
**发生时机**：流式对话过程中  
**影响范围**：所有工具调用

---

## 🔍 可能的原因分析

### 1. Rig 框架版本兼容性问题

**发现**：
- 项目使用 `rig-core = "0.23"`
- 实际依赖 `rig-core v0.23.1`

**可能的问题**：
- Rig 0.23.x 的工具定义 API 可能有变化
- 异步 trait 的签名可能不兼容
- 工具定义序列化问题

### 2. 工具定义异步问题

**当前代码**（`cortex-mem-rig/src/tools/mod.rs`）：

```rust
fn definition(
    &self,
    _prompt: String,
) -> impl std::future::Future<Output = ToolDefinition> + Send + Sync {
    async {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "...".to_string(),
            parameters: json!({...}),
        }
    }
}
```

**潜在问题**：
- 异步 future 的返回类型可能不稳定
- Rig 框架可能期望不同的签名
- 序列化时可能出现问题

### 3. LLM API 调用问题

**可能场景**：
- API 密钥无效
- API 端点不可达
- 请求格式不兼容
- 网络超时

### 4. 工具数量过多

**当前注册的工具**：
```rust
.tool(memory_tools.search_tool())
.tool(memory_tools.find_tool())
.tool(memory_tools.abstract_tool())
.tool(memory_tools.overview_tool())
.tool(memory_tools.read_tool())
.tool(memory_tools.ls_tool())
.tool(memory_tools.store_tool())
// 7 个工具
```

**可能问题**：
- 某些 LLM API 对工具数量有限制
- 工具定义总大小超过 API 限制
- 序列化后的 JSON 过大

---

## 🛠️ 修复方案

### 方案1：简化工具定义（立即尝试）

**思路**：减少工具数量，保留核心功能

**修改**：`examples/cortex-mem-tars/src/agent.rs`

```rust
// 构建带有精简工具集的 agent
let completion_model = llm_client
    .completion_model(model)
    .completions_api()
    .into_agent_builder()
    .preamble(&system_prompt)
    // ========== 核心工具（最小集）==========
    .tool(memory_tools.search_tool())  // 搜索
    .tool(memory_tools.store_tool())   // 存储
    // 其他工具可以在需要时逐步添加
    .build();
```

**优点**：
- 减少 API 负载
- 降低出错概率
- 便于调试

### 方案2：检查 Rig 版本兼容性

**步骤**：
1. 查看 Rig 0.23 的 changelog
2. 检查 Tool trait 的定义变化
3. 更新工具实现以匹配新 API

### 方案3：添加错误处理和日志

**修改**：添加更详细的日志输出

```rust
pub async fn create_memory_agent(...) -> Result<...> {
    // 创建租户工具
    let memory_tools = create_memory_tools_with_tenant(data_dir, agent_id).await?;
    
    tracing::info!("Created memory tools for agent: {}", agent_id);
    
    // 构建 agent
    let completion_model = llm_client
        .completion_model(model)
        .completions_api()
        .into_agent_builder()
        .preamble(&system_prompt)
        .tool(memory_tools.search_tool())
        .tool(memory_tools.store_tool())
        .build();
    
    tracing::info!("Agent built successfully");
    
    Ok(completion_model)
}
```

### 方案4：检查 API 配置

**验证清单**：
- ✅ API Key 是否正确
- ✅ API Base URL 是否可达
- ✅ 模型名称是否存在
- ✅ 网络连接是否正常

---

## 🎯 立即行动步骤

### Step 1: 最小化工具集测试

**目的**：排除工具数量过多的问题

**操作**：
1. 注释掉大部分工具
2. 只保留 `search` 和 `store`
3. 重新编译运行
4. 观察是否还报错

### Step 2: 添加详细日志

**目的**：定位具体失败位置

**操作**：
1. 在 agent 创建前后添加日志
2. 在工具调用时添加日志
3. 观察日志输出

### Step 3: 验证 API 连接

**目的**：排除 LLM API 问题

**操作**：
```bash
# 测试 API 连接
curl -X POST "https://api.openai.com/v1/chat/completions" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

---

## 📊 其他已完成的修复

### ✅ 修复 L0/L1 文件生成

**问题**：Store 时不生成 `.abstract.md` 和 `.overview.md`

**原因**：`generate_all_layers` 依赖 LLM，没有 LLM 时跳过生成

**修复**：添加 fallback 方法（基于规则）

**代码**：`cortex-mem-core/src/layers/manager.rs`

```rust
pub async fn generate_all_layers(&self, uri: &str, content: &str) -> Result<()> {
    // 1. Write L2 (detail)
    self.filesystem.write(uri, content).await?;
    
    // 2. Generate L0/L1 (with or without LLM)
    if let Some(llm) = &self.llm_client {
        // ✅ 有 LLM：使用 LLM 生成高质量摘要
        let abstract_text = self.abstract_gen.generate_with_llm(content, llm).await?;
        // ...
    } else {
        // ✅ 没有 LLM：使用 fallback 方法（基于规则）
        let abstract_text = self.abstract_gen.generate(content).await?;
        let abstract_uri = Self::get_layer_uri(uri, ContextLayer::L0Abstract);
        self.filesystem.write(&abstract_uri, &abstract_text).await?;
        
        let overview = self.overview_gen.generate(content).await?;
        let overview_uri = Self::get_layer_uri(uri, ContextLayer::L1Overview);
        self.filesystem.write(&overview_uri, &overview).await?;
    }
    
    Ok(())
}
```

**效果**：
- ✅ 现在 Store 时会自动生成 L0/L1 文件
- ✅ 即使没有 LLM 也能工作
- ✅ 使用基于规则的 fallback 方法

---

## 🧪 测试建议

### 测试1：验证 L0/L1 生成

```bash
# 1. 清理旧数据
rm -rf ~/Library/Application\ Support/com.cortex-mem.tars/cortex

# 2. 重新运行 TARS
cargo run -p cortex-mem-tars

# 3. 存储一段记忆
# （与 Bot 对话："请记住这段对话"）

# 4. 检查生成的文件
tree ~/Library/Application\ Support/com.cortex-mem.tars/cortex/tenants/.../timeline/

# 预期结果：
# ├── 10_00_00_xxx.md       # L2
# ├── .abstract.md           # L0（新生成！）
# └── .overview.md           # L1（新生成！）
```

### 测试2：最小工具集测试

```bash
# 修改 agent.rs 只保留 2 个工具
# 重新编译运行
cargo run -p cortex-mem-tars

# 观察是否还报 "Failed to get tool definitions"
```

---

## 📝 下一步行动

### 优先级1：工具调用失败

1. **简化工具集**
   - 只保留 `search` 和 `store`
   - 测试是否解决问题

2. **添加日志**
   - 定位具体失败位置
   - 获取更多错误信息

3. **检查 API**
   - 验证 API 连接
   - 测试基础 LLM 调用

### 优先级2：验证 L0/L1 生成

1. **清理数据重新测试**
2. **确认文件生成**
3. **检查内容质量**

---

## 🎯 总结

### 当前状态

| 问题 | 状态 | 说明 |
|------|------|------|
| **工具调用失败** | 🔴 待修复 | "Failed to get tool definitions" |
| **L0/L1 不生成** | ✅ 已修复 | 添加 fallback 方法 |
| **Dimension 错误** | ✅ 已修复 | 支持 session 维度 |
| **数据目录重复** | ✅ 已修复 | 统一路径来源 |

### 推荐修复顺序

1. ✅ **L0/L1 生成**（已完成）
2. 🔴 **工具调用失败**（进行中）
   - 先尝试最小工具集
   - 添加详细日志
   - 检查 API 配置

---

**文档创建时间**：2026-02-09 17:50  
**问题优先级**：高  
**预估修复时间**：20-30 分钟
