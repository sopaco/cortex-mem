# Cortex-Mem V2.1 与 OpenViking 对齐分析及补充说明

## 一、整体对齐评估

基于对 OpenViking 项目的深入分析，Cortex-Mem V2.1 的设计方案在核心理念上与 OpenViking 高度对齐，实现了文件系统范式、分层上下文模型和记忆管理机制。以下是详细的对齐分析和补充建议。

## 二、URI 结构设计对齐分析

### 2.1 对齐程度：✅ 高度对齐

Cortex-Mem V2.1 的 URI 结构设计与 OpenViking 基本一致，仅存在协议差异（`cortex://` vs `viking://`）。两者都采用了相同的目录层次结构：

```
cortex://                    (viking://)
├── resources/              (resources/)
├── user/                   (user/)
├── agent/                  (agent/)
└── session/{session_id}/   (session/{session_id}/)
```

### 2.2 差异点与建议

1. **差异**：Cortex-Mem 中 User/Agent 目录直接位于根目录下，而 OpenViking 使用了 `user/memories/` 和 `agent/memories/` 子目录。
   
2. **建议**：考虑采用以下结构以更好地对齐 OpenViking：
   ```
   cortex://
   ├── resources/
   ├── user/memories/
   │   ├── profile.md
   │   ├── preferences/
   │   ├── entities/
   │   └── events/
   ├── agent/memories/
   │   ├── cases/
   │   └── patterns/  (Note: OpenViking 使用 patterns 而非 instructions)
   └── session/
   ```

3. **说明**：OpenViking 中的 `patterns` 与 Cortex-Mem 的 `instructions` 类似，都是存储 Agent 的行为模式和指令。

## 三、记忆分类体系对齐分析

### 3.1 User 记忆分类：✅ 基本对齐

| Cortex-Mem | OpenViking | 对齐状态 |
|------------|------------|----------|
| profile.md | profile.md | ✅ 完全对齐 |
| preferences/ | preferences/ | ✅ 完全对齐 |
| entities/ | entities/ | ✅ 完全对齐 |
| events/ | events/ | ✅ 完全对齐 |

### 3.2 Agent 记忆分类：⚠️ 部分对齐

| Cortex-Mem | OpenViking | 对齐状态 |
|------------|------------|----------|
| cases/ | cases/ | ✅ 完全对齐 |
| skills/ | - | ❌ OpenViking 无此分类 |
| instructions/ | patterns/ | ⚠️ 功能相似，命名不同 |

**建议**：
1. 将 `instructions/` 重命名为 `patterns/` 以保持一致性
2. 考虑将 `skills/` 合并到 `patterns/` 中，因为 OpenViking 将所有 Agent 行为模式统一归为 `patterns`

## 四、租户模型对齐分析

### 4.1 对齐程度：⚠️ 简化版本

OpenViking 采用复杂的三层 Key 结构（Root/Account/User）和 RBAC 模型，而 Cortex-Mem V2.1 采用了简化的租户模型：

- **OpenViking**: account_id + user_id + agent_id 三元组
- **Cortex-Mem**: 仅 tenant_id（1 个租户 = 1 个 Bot 实例）

### 4.2 设计差异分析

Cortex-Mem 的简化设计适用于单租户场景，与 OpenViking 的嵌入式模式（Embedded Mode）更为接近。这种设计有以下优点：

1. **简单性**：减少复杂的多租户逻辑，降低实现难度
2. **性能**：避免复杂的权限检查和过滤开销
3. **适用性**：适合单个 Bot 实例或独立部署场景

### 4.3 补充建议

如果未来需要支持多租户，可以参考 OpenViking 的以下设计：

1. **API Key 管理**：实现三层 Key 结构（Root/Account/User）
2. **RBAC 模型**：添加角色和权限检查机制
3. **存储隔离**：在物理路径中添加 account_id 前缀

## 五、目录递归检索策略对齐分析

### 5.1 对齐程度：✅ 概念对齐

Cortex-Mem 的"改进五：实现目录递归检索"与 OpenViking 的核心检索策略概念一致，但需要补充以下关键细节：

### 5.2 OpenViking 目录递归检索的关键实现细节

1. **意图分析**：
   - 使用 LLM 分析查询意图，生成 0-5 个 TypedQuery 对象
   - 每个 TypedQuery 包含重写后的查询、上下文类型（MEMORY/RESOURCE/SKILL）、意图和优先级
   - 复杂查询时应考虑实现类似机制

2. **分数传播机制**：
   - 最终分数 = α × 当前节点分数 + (1-α) × 父目录分数
   - 默认 α=0.5，体现"上下文语境"的重要性
   - 建议在实现中引入此机制以提高相关性排序准确性

3. **收敛检测**：
   - 当连续多轮（默认3轮）前 K 个结果没有变化时，提前终止检索
   - 平衡检索效率和准确性
   - 建议实现类似机制以避免不必要的计算

4. **分层上下文模型应用**：
   - L0（摘要，约100 Token）用于向量搜索召回
   - L1（概览，约2000 Token）用于 Rerank 精排
   - L2（详情，无限制）按需加载
   - Cortex-Mem 已实现此模型，需确保在递归检索中正确应用

## 六、记忆提取机制对齐分析

### 6.1 对齐程度：✅ 概念对齐

Cortex-Mem 的"改进七：实现 Session 记忆提取机制"与 OpenViking 的 MemoryExtractor 概念一致，需要补充以下细节：

### 6.2 OpenViking MemoryExtractor 的关键实现细节

1. **语言检测**：
   - 首先检测用户消息的主要语言
   - 确保提取的记忆以用户使用的语言存储
   - 避免系统消息的偏置

2. **记忆分类与存储**：
   - OpenViking 将记忆分为 6 类：PROFILE, PREFERENCES, ENTITIES, EVENTS, CASES, PATTERNS
   - PROFILE 总是合并到单一文件，其他类别作为独立文件存储
   - 记忆存储路径：`viking://user/memories/{category}/{uuid}.md`

3. **去重与合并机制**：
   - 支持三种操作：CREATE（创建）, MERGE（合并）, SKIP（跳过）
   - 使用向量预过滤 + LLM 决策的去重策略
   - 合并时使用专门的提示模板确保内容连贯性

### 6.3 实施建议

1. 实现详细的记忆去重和合并机制
2. 确保记忆 UUID 生成的唯一性
3. 考虑增加语言检测功能
4. 支持记忆的批量处理和后台向量化

## 七、向量存储策略对齐分析

### 7.1 对齐程度：✅ 概念对齐

Cortex-Mem 的"改进六：向量存储 ID 规范化"与 OpenViking 的设计理念一致，需要参考以下细节：

### 7.2 OpenViking 向量存储设计

1. **双层架构**：
   - AGFS 负责内容存储（L0/L1/L2 完整内容）
   - VectorDB 负责索引存储（URI、向量、元数据）
   - 内容和索引分离，提高灵活性

2. **多租户隔离**：
   - 在 VectorDB schema 中添加 `account_id` 和 `owner_space` 字段
   - 查询时根据角色添加过滤条件
   - 简化版可考虑仅添加 `tenant_id` 字段

3. **向量化队列**：
   - 新创建或合并的记忆加入队列
   - 后台处理，不影响主流程

## 八、总结与补充建议

### 8.1 对齐程度总结

| 方面 | 对齐程度 | 说明 |
|------|----------|------|
| URI 结构 | 85% | 基本一致，需微调目录层次 |
| 记忆分类 | 75% | User 维度高度对齐，Agent 维度需调整 |
| 租户模型 | 60% | 简化版本，适合单租户场景 |
| 目录递归检索 | 70% | 概念对齐，需补充实现细节 |
| 记忆提取 | 80% | 概念对齐，需增加去重合并机制 |
| 向量存储 | 85% | 基本一致，需参考双层架构 |

### 8.2 优先补充建议

1. **目录结构调整**：
   - User/Agent 下添加 `memories/` 子目录
   - 将 `instructions/` 重命名为 `patterns/`
   - 考虑合并 `skills/` 到 `patterns/`

2. **目录递归检索增强**：
   - 实现意图分析（可选）
   - 采用分数传播机制
   - 添加收敛检测逻辑

3. **记忆提取完善**：
   - 实现语言检测
   - 添加去重和合并机制
   - 规范化记忆 UUID 生成

4. **向量存储优化**：
   - 实现内容和索引分离存储
   - 添加后台向量化队列
   - 优化索引结构以支持递归检索

### 8.3 实施路线图建议

| 阶段 | 重点改进项 | 预期效果 |
|------|------------|----------|
| 第一阶段 | 目录结构调整、记忆分类对齐 | 基础架构与 OpenViking 完全对齐 |
| 第二阶段 | 目录递归检索实现、向量存储优化 | 核心检索能力追平 OpenViking |
| 第三阶段 | 记忆提取完善、高级功能实现 | 全面功能对齐，具备生产可用性 |

通过以上补充和优化，Cortex-Mem V2.1 将能够在核心能力上追齐 OpenViking，同时在单租户场景下保持简洁高效的特性。

---

## 九、OpenViking 技术方案深度问答与咨询记录

本部分收集了通过 deepwiki MCP 查询到的高置信度且深入的关于 OpenViking 方案细节的常见 Q&A 和咨询记录，为 Cortex-Mem 的实现提供更具体的参考。

### 9.1 OpenViking 文件系统范式的实现细节

**Q：OpenViking 中的文件系统范式与传统文件系统有哪些主要区别？它是如何实现虚拟文件系统与实际存储的映射的？**

**A：** OpenViking 的文件系统范式与传统文件系统存在显著差异，主要体现在存储模型、检索方法、输出形式、记忆能力、可观测性以及上下文类型上。

**主要区别：**
1. **存储模型**：传统文件系统通常是扁平化的存储模型，而 OpenViking 采用层次化的虚拟文件系统 AGFS (Agent Global File System) 来组织上下文。所有上下文，包括记忆、资源和技能，都被抽象并组织在一个虚拟文件系统中，并映射到 `viking://` 协议下的虚拟目录。

2. **检索方法**：传统文件系统主要通过路径或文件名进行直接访问。OpenViking 则采用创新的"目录递归检索"策略，结合了意图分析、向量检索、精细探索和递归下探等多种检索方式。

3. **输出形式**：传统文件系统返回原始文件内容。OpenViking 提供结构化的上下文输出，分为 L0 (摘要)、L1 (概览) 和 L2 (详情) 三个层级，实现按需加载，从而降低 Token 消耗。

4. **记忆能力**：传统文件系统不具备记忆能力。OpenViking 内置了记忆自迭代闭环，支持自动提取和迭代六种记忆分类，使 Agent 能够"越用越聪明"。

**虚拟文件系统与实际存储的映射：**

OpenViking 通过 `VikingFS` 抽象层实现虚拟文件系统与实际存储的映射：

1. **双层存储架构**：
   - AGFS (Agent Global File System)：负责内容存储，包括 L0/L1/L2 完整内容、多媒体文件和关联关系
   - 向量库索引：负责索引存储，包含 URI、向量和元数据，但不存储文件内容，主要用于语义搜索

2. **URI 映射**：`VikingFS` 将 `viking://` 格式的虚拟 URI 映射到 AGFS 的本地路径，例如 `viking://resources/docs/auth` 映射到 `/local/resources/docs/auth`。

3. **向量同步**：当文件或目录被删除或移动时，`VikingFS` 会同步更新向量索引。例如，`rm()` 方法在删除 AGFS 中的文件后，会调用 `_delete_from_vector_store()` 方法从向量存储中删除对应的 URI。

### 9.2 向量化实现与多模态处理

**Q：OpenViking 中的向量化实现有什么特点？它是如何处理多模态内容（如图片、视频）的向量化？向量检索和标量过滤是如何结合使用的？**

**A：** OpenViking 的向量化实现具有多模态内容处理能力，支持向量检索与标量过滤的结合使用。

**向量化实现特点：**
1. **多模态支持**：能够处理文本、图片、视频等多种模态的数据，并将其转化为向量表示。通过配置 `embedding` 模块中的 `input` 参数为 `"multimodal"` 来实现。例如，`doubao-embedding-vision-250615` 模型支持多模态输入。

2. **多种 Embedding 模式**：支持密集 (Dense)、稀疏 (Sparse) 和混合 (Hybrid) 三种 Embedding 模式。

3. **可插拔的 Embedding Provider**：支持多种 Embedding 服务提供商，包括 `volcengine`、`openai` 和 `vikingdb`。

4. **异步处理**：向量化过程通过异步队列进行处理，将资源摄取与向量化解耦，提高效率。

**多模态内容向量化：**

OpenViking 通过 `VectorizerAdapter` 类处理多模态内容的向量化。当调用 `search_by_multimodal` 方法时，它会利用 `vectorizer_adapter` 将输入的文本、图片或视频数据转化为密集向量和稀疏向量。

例如，在 `LocalCollection` 中，`search_by_multimodal` 方法会检查 `vectorizer_adapter` 是否已初始化，然后调用其 `vectorize_one` 方法来生成向量。`vectorize_one` 方法接收 `text`、`image` 和 `video` 作为输入，并将其传递给底层的 `vectorizer.vectorize_document` 方法进行向量化。

对于 Volcengine 后端，`VolcengineCollection` 直接调用 `/api/vikingdb/data/search/multi_modal` 接口进行多模态搜索。

**向量检索和标量过滤的结合：**

OpenViking 通过 `search` 方法结合向量相似度搜索和标量过滤。当调用 `search` 方法时，如果提供了 `query_vector` 或 `sparse_query_vector`，系统会执行向量搜索。此时，`filter` 参数中定义的标量过滤条件会与向量搜索结合，以进一步缩小结果范围。如果未提供向量，则只执行纯粹的标量过滤。

标量过滤使用嵌套字典结构定义操作符，例如 `must` (AND)、`should` (OR)、`must_not` (NOT)、`range` (范围) 和 `prefix` (前缀) 等。

### 9.3 记忆去重与合并机制

**Q：OpenViking 中是如何实现记忆的去重和合并机制的？特别是在处理重复和冲突信息时采用了什么策略？记忆更新的触发时机和频率是如何控制的？**

**A：** OpenViking 通过 `SessionCompressor` 类实现记忆的去重和合并机制，该机制在会话提交后异步触发。

**记忆去重和合并机制：**

OpenViking 的记忆去重和合并机制主要通过 `SessionCompressor`、`MemoryExtractor` 和 `MemoryDeduplicator` 协同工作来完成：

1. **记忆提取**：当会话提交时，`SessionCompressor` 会调用 `MemoryExtractor` 从会话消息中提取候选记忆。这些记忆分为六个类别：`PROFILE`、`PREFERENCES`、`ENTITIES`、`EVENTS`、`CASES` 和 `PATTERNS`。每个记忆都包含 L0（摘要）、L1（概述）和 L2（完整内容）三个层次的抽象。

2. **去重决策**：对于提取出的候选记忆，`SessionCompressor` 会根据记忆类别进行不同的处理：
   - `PROFILE` 类别：`PROFILE` 记忆是特殊情况，它总是直接合并，跳过去重步骤，因为它是一个单一文件，持续累积用户身份信息。`_append_to_profile` 方法负责将新内容合并到现有的 `profile.md` 文件中。
   - 其他类别：对于其他记忆类别，`SessionCompressor` 会调用 `MemoryDeduplicator` 进行去重决策。

3. **MemoryDeduplicator 采用两阶段方法进行去重**：
   - **向量预过滤**：首先，它会为候选记忆生成嵌入向量，并在向量数据库中进行相似性搜索。只有当相似度超过 `SIMILARITY_THRESHOLD` (默认为 0.7) 且类别匹配时，才会进入下一阶段。
   - **LLM 决策**：如果找到相似记忆，`MemoryDeduplicator` 会调用 VLM (Vision Language Model) 并使用 `dedup_decision` 提示词来决定是 `CREATE` (创建新记忆)、`MERGE` (合并到现有记忆) 还是 `SKIP` (跳过，因为是重复的)。

**冲突处理和合并策略：**
- **`CREATE` 决策**：如果决定 `CREATE`，则会创建一个新的记忆文件，并将其加入向量化队列。
- **`SKIP` 决策**：如果决定 `SKIP`，则会增加跳过计数器，不执行任何操作。
- **`MERGE` 决策**：
  - 对于 `PREFERENCES`、`ENTITIES` 和 `PATTERNS` 类别，如果决定 `MERGE`，系统会读取现有的记忆内容，然后调用 `MemoryExtractor._merge_memory()` 方法，使用 VLM 和 `memory_merge` 提示词将现有内容与新内容合并。合并后的内容会写回原文件，并重新进行向量化。
  - `memory_merge.yaml` 提示词要求 VLM 移除重复信息，保留最新细节，保持叙述连贯，并且只输出合并后的内容，不带解释。VLM 的 `temperature` 设置为 `0.0`，以确保合并行为的确定性。
  - 对于 `EVENTS` 和 `CASES` 类别，即使 `MemoryDeduplicator` 决定 `MERGE`，`SessionCompressor` 也会将其视为 `SKIP`，因为这些类别不支持合并。

**记忆更新的触发时机和频率：**

记忆更新的触发时机和频率由会话的 `commit()` 操作控制：
- **触发时机**：当用户调用 `session.commit()` 方法时，会话消息会被持久化，并触发记忆提取任务。
- **频率控制**：记忆提取是一个异步过程，`session.commit()` 会立即返回，而提取任务会被加入后台队列进行处理。这意味着记忆更新的频率取决于 `commit()` 操作的调用频率，以及后台工作队列的处理能力。用户可以通过 `client.wait_processed()` 方法等待后台处理完成。

### 9.4 L0/L1/L2 三层架构的实现细节

**Q：OpenViking 中 L0/L1/L2 三层架构的具体实现细节是什么？每层的内容是如何生成和维护的？在检索过程中这三层是如何协同工作的？**

**A：** OpenViking 的 L0/L1/L2 三层架构是一种渐进式内容加载机制，旨在平衡检索效率和内容完整性，同时降低 Token 消耗。

**L0/L1/L2 三层架构实现细节：**

OpenViking 将所有上下文内容处理为三个层级：
- **L0 (摘要)**：最精简的内容表示，通常为一句话概括，限制在约 100 个 Token。它用于快速检索和识别，例如向量搜索召回和快速过滤。
- **L1 (概览)**：包含核心信息和使用场景的综合摘要，限制在约 2K 个 Token。它用于 Agent 在规划阶段进行决策，例如 Rerank 精排和内容导航。
- **L2 (详情)**：完整的原始数据，没有 Token 限制。它仅在 Agent 确认需要时才进行深度读取。

在文件系统中，L0 和 L1 层级的内容以 `.abstract.md` 和 `.overview.md` 的形式存储在每个目录中，而 L2 层级则是原始文件或子目录。

**内容生成和维护：**

L0/L1 层级的内容生成主要发生在两个阶段：
1. **添加资源时**：在 Parser 解析文件后，SemanticQueue 会异步生成 L0/L1。`SemanticProcessor` 负责自底向上遍历目录，为每个目录生成 L0/L1。子目录的 L0 会被聚合到父目录的 L1 中，形成层级导航。
2. **会话归档时**：`SessionCompressor` 会在压缩旧消息时为历史片段生成 L0/L1。

L2 层级的内容是原始文件，因此其维护与原始文件的生命周期一致。

**检索过程中的协同工作：**

在检索过程中，L0/L1/L2 层级协同工作，以实现高效和精确的上下文获取：
1. **意图分析**：首先通过意图分析生成多个检索条件。
2. **初始定位 (L0)**：利用向量检索快速定位初始切片所在的高分目录。L0 摘要在此阶段用于快速判断相关性。
3. **精细探索 (L1)**：在该目录下进行二次检索，并将高分结果更新至候选集合。L1 概览在此阶段提供内容导航和决策参考，帮助 Agent 了解内容结构和要点。
4. **递归下探**：若目录下仍存在子目录，则逐层递归重复上述二次检索步骤。
5. **结果汇总 (L2)**：最终拿到最相关上下文返回。只有在确定需要详细信息时，才会加载 L2 详情。

这种"先锁定高分目录、再精细探索内容"的策略，不仅能找到语义最匹配的片段，更能理解信息所在的完整语境，从而提升检索的全局性与准确性。

### 9.5 AGFS (Agent Global File System) 的设计与实现

**Q：OpenViking 中的 AGFS (Agent Global File System) 是如何设计和实现的？它与传统分布式文件系统有哪些区别？在多租户环境下是如何实现数据隔离的？**

**A：** OpenViking 中的 AGFS (Agent Global File System) 是一个底层存储实现，它为 VikingFS 提供 POSIX 风格的文件操作，并支持多种后端存储。AGFS 的设计旨在为 AI Agent 提供统一的上下文管理，通过文件系统范式来组织记忆、资源和技能。

**AGFS 的设计与实现：**

AGFS 是 OpenViking 双层存储架构中的内容存储层，负责存储 L0/L1/L2 的完整内容、多媒体文件和关联关系。

**架构**：
AGFS 作为 VikingFS 的底层实现，通过 Go 语言编写，提供高性能的文件操作。它包含以下关键组件：
- **AGFS Server**: 核心文件系统实现
- **LevelDB**: 用于元数据和索引的键值存储
- **SPDLog**: 高性能日志记录
- **SDK Bindings**: 通过 CGo 实现 Python VikingFS 与 Go AGFS 之间的接口

**后端类型**：
AGFS 支持多种后端存储，通过配置文件 `ov.conf` 中的 `storage.agfs.backend` 字段进行配置：
- **本地文件系统 (`localfs`)**: 将数据存储在本地文件系统中。在 `openviking/agfs_manager.py` 中，当 `backend` 为 `"local"` 时，AGFS 配置会启用 `localfs` 插件，并指定 `local_dir` 为 `vikingfs_path`。
- **S3 兼容存储 (`s3fs`)**: 支持 S3 兼容的对象存储服务。当 `backend` 为 `"s3"` 时，AGFS 配置会启用 `s3fs` 插件，并包含 `bucket`, `region`, `access_key_id`, `secret_access_key`, `endpoint`, `prefix`, `disable_ssl` 等 S3 配置。
- **内存存储 (`memory`)**: 主要用于测试目的，数据存储在内存中。当 `backend` 为 `"memory"` 时，AGFS 配置会启用 `memfs` 插件。

AGFS 的配置是通过 `AGFSManager` 动态生成的 YAML 文件。

**与传统分布式文件系统的区别：**

OpenViking 的 AGFS 与传统分布式文件系统在设计理念和功能上存在显著差异：
- **存储模型**: 传统文件系统通常是扁平的存储模型，而 AGFS 采用分层文件系统（Hierarchical filesystem）来统一管理 Agent 的记忆、资源和技能。
- **URI 抽象**: AGFS 通过 VikingFS 提供统一的 URI 抽象层 (`viking://`)，将逻辑路径映射到物理存储位置，屏蔽了底层存储细节。
- **AI 增强操作**: 除了传统的 `read`, `write`, `ls` 等文件操作，VikingFS 还提供 `abstract`, `overview`, `find` 等 AI 增强操作，用于语义搜索和分层内容加载。
- **分层上下文**: AGFS 支持 L0/L1/L2 三层上下文模型，按需加载，以优化 token 消耗和检索效率。

**多租户环境下的数据隔离：**

OpenViking 在多租户环境下通过三维隔离模型实现数据隔离：`account`、`user` 和 `agent`。

**隔离维度**：
- **Account 级隔离**: 顶层隔离，不同租户之间的数据完全不可见。
- **User 级隔离**: 同一 `account` 内，不同用户的私有数据互不可见，例如用户记忆和资源。
- **Agent 级隔离**: 同一 `account` 内，不同 Agent 的数据互不可见，例如 Agent 的学习记忆、技能和指令。

**实现方式**：
1. **AGFS 文件系统隔离**:
   - `VikingFS` 保持单例，不持有任何租户状态。
   - 通过 `RequestContext` 传递 `account_id`。
   - `_uri_to_path` 方法在多租户模式下会将 `account_id` 作为前缀添加到 AGFS 物理路径中，实现物理隔离。
   - `_path_to_uri` 方法在将 AGFS 路径转换回 Viking URI 时会移除 `account_id` 前缀，对用户透明。
   - `VikingFS` 内部的 `_is_accessible()` 方法根据用户的角色和 URI 中的 `space` 信息进行权限过滤，确保用户只能访问其有权限的数据。

2. **VectorDB 租户隔离**:
   - 向量库的 `context` 集合中新增 `account_id` 和 `owner_space` 字段。
   - 查询时，根据用户的角色 (`ROOT`, `ACCOUNT_ADMIN`, `USER`) 注入相应的过滤条件，确保只返回用户有权限的数据。
   - 写入时，`Context` 对象会携带 `account_id` 和 `owner_space`，并透传到 VectorDB。

3. **目录初始化**:
   - 在创建新 `account` 时，会初始化 `account` 级的公共根目录。
   - 用户首次访问时，会懒初始化 `user space` 子目录。
   - Agent 首次使用时，会懒初始化 `agent space` 子目录。

---

以上问答和咨询记录详细介绍了 OpenViking 的核心技术实现，这些信息对 Cortex-Mem V2.1 的实现提供了宝贵的参考，特别是在文件系统范式设计、多层架构实现、向量化处理和记忆管理机制方面。