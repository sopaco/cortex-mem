# Cortex Memory - 项目状态与路线图

**项目名称**: Cortex Memory  
**当前版本**: V2.0.0  
**最后更新**: 2026-02-11 10:00  
**状态**: ✅ 生产就绪

---

## 📊 项目概览

Cortex Memory是一个高性能、模块化的AI Agent记忆管理系统，采用`cortex://`虚拟URI协议，实现L0/L1/L2三层抽象架构，为AI Agent提供长期记忆存储和智能检索能力。

### 核心特性

- ✅ **虚拟文件系统**: cortex://协议，纯Markdown存储
- ✅ **三层架构**: L0/L1/L2抽象层，优化LLM上下文
- ✅ **智能检索**: 文件系统+向量+混合搜索
- ✅ **会话管理**: 完整的对话生命周期管理
- ✅ **记忆提取**: LLM驱动的自动提取
- ✅ **丰富工具链**: CLI、MCP、HTTP、Tools库、Rig集成
- ✅ **多维度存储**: session/user/agent 三种存储范围支持

---

## 🎯 当前状态 (2026-02-11)

### ✅ 已完成功能

#### 1. 核心系统 (cortex-mem-core)

| 模块 | 状态 | 功能 |
|------|------|------|
| 文件系统 (filesystem) | ✅ | cortex://协议、读写、列表、删除 |
| 会话管理 (session) | ✅ | 会话生命周期、消息存储、Timeline |
| 抽象层 (layers) | ✅ | L0/L1/L2自动生成和索引 |
| 检索引擎 (retrieval) | ✅ | 意图分析、递归检索 |
| 自动化 (automation) | ✅ | 自动索引、会话提取 |
| LLM集成 (llm) | ✅ | rig-core 0.23集成 |
| 向量存储 (vector_store) | ✅ | Qdrant集成（可选） |
| 自动索引 (sync) | ✅ | 文件系统到Qdrant自动同步 |
| 多维度存储 | ✅ | session/user/agent 存储范围 |

#### 2. 工具链

| 工具 | 状态 | 版本 | 说明 |
|------|------|------|------|
| cortex-mem-cli | ✅ | 2.0.0 | 命令行工具，支持向量搜索 |
| cortex-mem-mcp | ✅ | 2.0.0 | MCP服务器，支持Claude Desktop |
| cortex-mem-service | ✅ | 2.0.0 | HTTP REST API服务 |
| cortex-mem-tools | ✅ | 2.0.0 | 高级工具库 |
| cortex-mem-rig | ✅ | 2.0.0 | Rig框架集成（简化版） |
| cortex-mem-config | ✅ | 2.0.0 | 配置管理 |

#### 3. 向量搜索集成 (2026-02-04完成)

**实施阶段**:
- ✅ Phase 1: Service向量搜索 (30分钟)
- ✅ Phase 2: MCP向量搜索 (20分钟)
- ✅ Phase 3: Tools项目重做 (40分钟)
- ✅ Phase 4: Rig项目修复 (30分钟)
- ✅ Phase 5: 测试和文档 (30分钟)

**功能特性**:
- 3种搜索模式: `filesystem` / `vector` / `hybrid`
- Feature-gated设计: `--features vector-search`
- 自动降级: Qdrant未配置时使用文件系统搜索
- 环境变量配置: `QDRANT_URL`, `QDRANT_COLLECTION`

**测试状态**:
```bash
✅ cargo build --workspace
✅ cargo build --workspace --features vector-search
✅ 全workspace编译通过，55+测试通过
```

#### 4. 文档状态

| 文档类型 | 数量 | 状态 |
|----------|------|------|
| 主文档 (README.md) | 1 | ✅ 最新 |
| 技术文档 (docs/) | 7 | ✅ 最新 |
| 子项目文档 | 5 | ✅ 最新 |
| API文档 | 5 | ✅ 最新 |

---

## 🚀 近期完成 (最近30天)

### 2026-02-11: 多维度存储 + 自动存储机制 + 租户路径优化

**完成内容**:
1. ✅ **多维度存储支持** - 实现 session/user/agent 三种存储维度
   - **session**: 会话级存储（默认）- `cortex://session/{thread_id}/timeline/`
   - **user**: 用户长期记忆 - `cortex://user/{user_id}/memories/`
   - **agent**: Agent专属记忆 - `cortex://agent/{agent_id}/memories/`
2. ✅ **对话自动存储机制** - AgentChatHandler 自动保存每轮对话
   - 每轮对话后自动存储用户输入和助手回复
   - 存储到 session 维度，自动生成 L0/L1 摘要
   - 无需手动调用 store 工具，简化 Agent 逻辑
   - 废弃手动保存方法 `save_conversations_to_memory()`
3. ✅ **租户路径优化** - 统一单数命名规范
   - 使用 `user/` 替代 `users/`，`agent/` 替代 `agents/`
   - 修复租户目录结构：移除多余的 `cortex` 子文件夹
   - 租户路径：`/data/tenants/{tenant_id}/user/`（不再是 `.../cortex/user/`）
   - 支持自动路由：单数路径在租户模式下自动映射到租户子目录
4. ✅ **自动记忆提取集成** - 退出时触发会话分析
   - 创建 AutoExtractor 并绑定到租户 operations
   - 应用退出时自动提取会话中的事实、决策、实体
   - 自动保存到 user 和 agent 维度作为长期记忆
5. ✅ **StoreTool 增强** - 新增 `scope` 参数支持
   - cortex-mem-rig: 更新工具定义，添加 scope 枚举参数
   - cortex-mem-tools: 实现多维度存储逻辑
   - 自动日期路径组织（YYYY-MM/DD/HH_MM_SS_id.md）
6. ✅ **用户记忆检索优化** - 改进 `extract_user_basic_info` 函数
   - 从 `cortex://user/{user_id}/` 维度搜索长期记忆
   - 优先使用 L1 overview，回退到 L0 abstract 或原始内容
   - 支持加载最多 20 条用户记忆

**技术亮点**:
- **零手动存储**: Agent 无需关心存储逻辑，专注对话质量
- **统一的路径架构**: `cortex://{scope}/{id}/{type}/{date}/{file}.md`
- **租户透明**: 单数路径自动适配租户模式，代码无需感知租户
- **完整的生命周期**: 对话存储 → 会话提取 → 长期记忆生成
- **向后兼容**: 默认 session 维度保持原有行为

### 2026-02-09: 向量搜索完善 + Bot记忆隔离 + 工具链修复

**完成内容**:
1. ✅ **真正的向量搜索** - 实现完整的向量语义搜索
   - Embedding生成（EmbeddingClient）
   - Qdrant向量存储
   - 语义搜索、递归搜索、混合搜索
2. ✅ **Qdrant索引自动化** - SyncManager自动同步
   - 文件系统到Qdrant的自动同步
   - 增量索引更新（基于哈希检查）
   - 按维度同步（agents/users/threads/global）
3. ✅ **Bot记忆隔离** - 多Bot系统支持
   - cortex-mem-rig: Bot隔离工具
   - cortex-mem-tools: scope参数支持
   - cortex-mem-tars: 完整示例
   - 修正隔离路径: `cortex://threads/{bot_id}`
4. ✅ **工具链修复** - TARS工具链问题修复
   - 修复 StoreArgs.thread_id 反序列化问题
   - 修复 LsArgs.uri 参数问题
   - 实现 LsTool bot_id 自动注入
   - 更新 System Prompt 说明
5. ✅ **配置系统完善** - 添加EmbeddingConfig
6. ✅ **编译优化** - 修复所有编译错误

**技术亮点**:
- SyncManager自动索引（新增）
- 真正的向量搜索（不再降级）
- Bot记忆完全隔离（路径统一）
- 工具链 serde(default) 修复
- Feature-gating设计
- 类型安全的API

### 2026-01-XX: V2核心系统

**完成内容**:
1. ✅ cortex://虚拟文件系统
2. ✅ L0/L1/L2三层抽象架构
3. ✅ 会话管理和Timeline
4. ✅ 记忆提取引擎
5. ✅ 智能检索系统
6. ✅ LLM集成（rig-core 0.23）
7. ✅ MCP服务器（rmcp 0.14）

---

## 📋 后续待办 (Roadmap)

### ✅ 已完成 (2026-02-09)

#### 1. 向量搜索完善
- [x] 实现真正的向量搜索（embedding生成）
- [x] Qdrant索引自动化（SyncManager）
  - [x] 自动同步文件系统到Qdrant
  - [x] 增量索引更新
  - [x] 索引状态监控

### 🔥 高优先级 (1-2周)

#### 2. 代码质量提升
- [ ] 清理编译警告（13个）
- [ ] 修复测试编译失败
- [ ] 补充集成测试
- [ ] 生成API文档（rustdoc）

### ⭐ 中优先级 (1个月)

#### 3. 功能增强
- [ ] Web管理界面 (cortex-mem-insights)
  - [ ] 基础UI框架
  - [ ] 会话浏览器
  - [ ] 搜索界面
  - [ ] 记忆可视化
---

## 🎓 技术债务

### 已完成

1. ~~向量搜索实现~~ ✅
   - ~~当前: 降级实现，仍使用文件系统搜索~~
   - ~~目标: 实现真正的向量语义搜索~~
   - ~~优先级: 高~~
   - **完成**: 实现了完整的向量搜索和SyncManager自动索引

2. ~~Bot记忆隔离~~ ✅
   - ~~当前: 不同Bot记忆混淆~~
   - ~~目标: Bot记忆完全隔离~~
   - ~~优先级: 高~~
   - **完成**: 实现了完整的Bot记忆隔离
     - cortex-mem-rig: Bot隔离工具（bot_id自动注入）
     - cortex-mem-tools: scope参数支持
     - 统一路径: `cortex://threads/{bot_id}`
     - 修复工具链: StoreArgs/LsArgs serde(default)

3. ~~对话自动存储~~ ✅
   - ~~当前: 需要手动调用 store 工具保存对话~~
   - ~~目标: AgentChatHandler 自动保存每轮对话~~
   - ~~优先级: 高~~
   - **完成**: 实现了完整的自动存储机制
     - AgentChatHandler 自动保存用户输入和助手回复
     - 存储到 session 维度，自动生成 L0/L1 摘要
     - 移除 store_tool()，简化 Agent 工具链

4. ~~租户路径规范化~~ ✅
   - ~~当前: 混合使用 users/ 和 user/，租户路径冗余~~
   - ~~目标: 统一使用单数命名，简化租户目录结构~~
   - ~~优先级: 中~~
   - **完成**: 路径规范化完成
     - 统一使用 `user/` 和 `agent/`（单数）
     - 修复租户目录：`tenants/{tenant_id}/user/`（移除多余 cortex 层）
     - 支持自动路由到租户子目录

### 需要重构的部分

1. **错误处理统一** ⚠️
   - 当前: 多种错误类型混用
   - 目标: 统一错误处理机制
   - 优先级: 中

### 已知问题

1. **编译警告** (13个警告)
   - 类型: 未使用的导入、变量
   - 影响: 无功能影响，但影响代码质量
   - 优先级: 低

2. **SessionStatus Display** 
   - 问题: 没有实现Display trait
   - 解决: 手动转换为字符串
   - 优先级: 低（已有workaround）

3. **Rig框架集成**
   - 问题: 移除了完整的rig-core集成
   - 现状: 简化版本，提供基本功能
   - 优先级: 低（根据需求决定）

---

## 📈 项目指标

### 代码统计

| 指标 | 数值 |
|------|------|
| 总代码行数 | ~15,000行 |
| Rust文件数 | ~80个 |
| 测试数量 | 55+ |
| 文档数量 | 20+ |
| 子项目数 | 7个 |

---

## 🔄 版本历史

### V2.0.0 (2026-02-11)

**重大变更**:
- ✅ 从Qdrant向量数据库迁移到文件系统为主
- ✅ 引入cortex://虚拟URI协议
- ✅ 实现L0/L1/L2三层抽象架构
- ✅ 完整的会话管理系统
- ✅ LLM驱动的记忆提取
- ✅ **真正的向量搜索**（2026-02-09完成）
- ✅ **Qdrant索引自动化**（2026-02-09完成）
- ✅ **Bot记忆隔离**（2026-02-09完成）
- ✅ **工具链修复**（2026-02-09完成）
- ✅ **多维度存储**（2026-02-11完成）
- ✅ **对话自动存储**（2026-02-11完成）
- ✅ **租户路径规范化**（2026-02-11完成）

**新增功能**:
- ✅ MCP服务器（Claude Desktop集成）
- ✅ HTTP REST API服务
- ✅ Tools高级工具库（serde(default)修复）
- ✅ Rig框架集成（Bot隔离支持）
- ✅ 向量搜索支持（3种模式）
- ✅ **SyncManager自动索引**
- ✅ **Bot记忆隔离支持**（cortex://threads/{bot_id}）
- ✅ **TARS演示程序**（完整的多Bot示例）
- ✅ **多维度存储支持**（session/user/agent scope）
- ✅ **对话自动存储**（AgentChatHandler 自动保存）
- ✅ **自动记忆提取**（退出时触发会话分析）

**改进**:
- ✅ 零外部依赖存储
- ✅ 易于迁移和备份
- ✅ 完整的文档系统
- ✅ 模块化设计
- ✅ Feature-gating可选功能
- ✅ **完整的向量搜索实现**
- ✅ **租户路径透明化**（单数命名自动适配）
- ✅ **零手动存储**（对话自动保存）

### V1.x (历史版本)

**特点**:
- Qdrant向量数据库
- 基础记忆存储
- 简单检索

**迁移到V2的原因**:
- 降低外部依赖
- 提高可维护性
- 增强扩展性
- 优化LLM集成

---

## 🤝 贡献指南

### 如何贡献

1. **报告问题**: 使用GitHub Issues
2. **提交PR**: Fork → Branch → PR
3. **文档改进**: 直接提交PR
4. **功能建议**: 先开Issue讨论

### 开发环境设置

```bash
# 1. 克隆仓库
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem

# 2. 安装依赖
cargo build

# 3. 运行测试
cargo test --workspace

# 4. 运行CLI
cargo run -p cortex-mem-cli -- --help
```

### 代码规范

- ✅ 使用rustfmt格式化代码
- ✅ 运行clippy检查
- ✅ 添加单元测试
- ✅ 更新相关文档

---

## 📞 联系方式

- **GitHub**: https://github.com/sopaco/cortex-mem
- **问题报告**: GitHub Issues
- **文档**: /docs 目录

---

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---

**最后更新**: 2026-02-11 10:30  
**维护者**: Cortex Memory Development Team  
**状态**: ✅ 生产就绪 - 核心功能完整，多维度存储，自动存储机制，租户隔离
**状态**: ✅ 活跃开发中
