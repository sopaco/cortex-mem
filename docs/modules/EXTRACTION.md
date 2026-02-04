# 记忆提取模块 (Extraction)

**模块路径**: `cortex-mem-core/src/extraction/`  
**职责**: LLM驱动的记忆提取、结构化知识抽取

---

## 核心组件

### MemoryExtractor

```rust
pub struct MemoryExtractor {
    filesystem: Arc<CortexFilesystem>,
    llm_client: Arc<LLMClient>,
    config: ExtractionConfig,
}
```

**主要方法**:
- `extract_from_messages()` - 从消息列表提取记忆
- `extract_facts()` - 提取事实
- `extract_decisions()` - 提取决策
- `extract_entities()` - 提取实体

### ExtractionConfig

```rust
pub struct ExtractionConfig {
    pub extract_facts: bool,
    pub extract_decisions: bool,
    pub extract_entities: bool,
    pub min_confidence: f64,
    pub max_messages_per_batch: usize,
}
```

## 提取类型

### 1. Facts（事实）

**定义**: 陈述性知识，客观信息

**示例**:
- "用户喜欢深色主题"
- "项目使用Rust语言"
- "会议定于周五下午3点"

**结构**:
```rust
pub struct ExtractedFact {
    pub content: String,
    pub confidence: f64,
    pub source: String,
    pub timestamp: DateTime<Utc>,
}
```

### 2. Decisions（决策）

**定义**: 决策点、推理过程

**示例**:
- "选择使用Axum框架，因为性能优异"
- "决定不实现鉴权功能，简化部署"
- "推迟Web UI开发，优先完成API"

**结构**:
```rust
pub struct ExtractedDecision {
    pub decision: String,
    pub reasoning: String,
    pub alternatives: Vec<String>,
    pub outcome: Option<String>,
}
```

### 3. Entities（实体）

**定义**: 人物、概念、事件

**示例**:
- 人物: "Alice", "Bob"
- 概念: "OAuth 2.0", "REST API"
- 事件: "系统升级", "Bug修复"

**结构**:
```rust
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: EntityType,
    pub description: String,
    pub mentions: Vec<String>,
}

pub enum EntityType {
    Person,
    Concept,
    Event,
    Tool,
}
```

## 提取流程

```
输入: 会话消息列表
    ↓
1. 构建对话上下文
    ↓
2. 分批处理（max_messages_per_batch）
    ↓
3. 并行提取
    ┌──────────┬──────────┬──────────┐
    │  Facts   │Decisions │ Entities │
    └────┬─────┴─────┬────┴─────┬────┘
         │           │          │
         └───────────┴──────────┘
                 ↓
4. LLM调用（rig-core）
    ↓
5. 结果解析和验证
    ↓
6. 置信度过滤（min_confidence）
    ↓
输出: ExtractedMemories
```

## LLM提示策略

### Facts提取提示

```
从以下对话中提取所有重要的事实性信息：

{conversation}

要求：
1. 只提取明确陈述的事实
2. 避免主观判断
3. 包含时间、地点等上下文
4. 格式化为JSON数组

输出格式：
[
  {
    "content": "...",
    "confidence": 0.9
  }
]
```

### Decisions提取提示

```
从以下对话中识别所有决策点：

{conversation}

要求：
1. 提取明确的决策
2. 包含决策理由
3. 列出考虑的备选方案
4. 记录决策结果（如有）

输出格式：
[
  {
    "decision": "...",
    "reasoning": "...",
    "alternatives": ["..."],
    "outcome": "..."
  }
]
```

## 存储策略

### 用户记忆存储

```
cortex://users/{user_id}/memories/
├── facts/
│   ├── 2026-02-04_fact_001.md
│   └── 2026-02-04_fact_002.md
├── decisions/
│   └── 2026-02-04_decision_001.md
└── entities/
    └── person_alice.md
```

### Agent记忆存储

```
cortex://agents/{agent_id}/memories/
├── learned/              # 从交互中学习的知识
├── behaviors/            # 行为模式
└── preferences/          # 用户偏好总结
```

## 性能优化

1. **批量处理**: 避免单条消息分析
2. **并行提取**: Facts/Decisions/Entities并行
3. **缓存机制**: 缓存LLM响应（未来）
4. **增量更新**: 只处理新消息（未来）

## 配置示例

```rust
let config = ExtractionConfig {
    extract_facts: true,
    extract_decisions: true,
    extract_entities: true,
    min_confidence: 0.6,  // 过滤低置信度结果
    max_messages_per_batch: 50,  // 每批最多50条消息
};
```

---

详见源码: [cortex-mem-core/src/extraction/](../../cortex-mem-core/src/extraction/)
