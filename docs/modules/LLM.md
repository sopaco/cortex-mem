# LLM集成模块 (LLM)

**模块路径**: `cortex-mem-core/src/llm/`  
**职责**: LLM客户端管理、提示工程、结构化输出

---

## 核心组件

### LLMClient

```rust
pub struct LLMClient {
    config: LLMConfig,
    agent: Agent,  // rig-core Agent
}
```

**主要方法**:
- `new()` - 创建客户端
- `generate()` - 生成文本
- `extract_structured()` - 提取结构化数据
- `prompt()` - 执行自定义提示

### LLMConfig

```rust
pub struct LLMConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model_efficient: String,
    pub temperature: f32,
    pub max_tokens: usize,
}
```

## 基于rig-core 0.23

### 为什么选择rig-core？

1. ✅ **OpenAI兼容**: 支持任何OpenAI API
2. ✅ **结构化输出**: 内置Serde集成
3. ✅ **提示管理**: 优雅的提示模板
4. ✅ **流式支持**: 支持SSE流式响应
5. ✅ **类型安全**: 强类型Rust接口

### 架构

```
Cortex-Mem
    ↓
LLMClient (cortex-mem-core)
    ↓
rig-core Agent
    ↓
OpenAI-compatible API
```

## 使用示例

### 1. 基础文本生成

```rust
let client = LLMClient::new(config)?;

let response = client.generate(
    "解释什么是OAuth 2.0授权码流程"
).await?;

println!("{}", response);
```

### 2. 结构化提取

```rust
#[derive(Deserialize)]
struct ExtractedFact {
    content: String,
    confidence: f64,
}

let facts: Vec<ExtractedFact> = client.extract_structured(
    "从对话中提取事实",
    conversation
).await?;
```

### 3. 自定义提示

```rust
let prompt = r#"
你是一个AI记忆提取助手。
从以下对话中提取所有重要的事实性信息。

对话：
{conversation}

要求：
1. 只提取明确陈述的事实
2. 包含上下文信息
3. 格式化为JSON数组

输出格式：
[{{"content": "...", "confidence": 0.9}}]
"#;

let response = client.prompt(prompt, context).await?;
```

## 提示工程最佳实践

### 1. 清晰的角色定义

```rust
let system_prompt = r#"
你是Cortex-Mem的记忆提取助手。
专长：从对话中识别和提取结构化知识。
原则：准确、客观、可验证。
"#;
```

### 2. 明确的输出格式

```rust
let format_instruction = r#"
输出格式（严格遵守）：
{
  "facts": [
    {"content": "...", "confidence": 0.9}
  ],
  "decisions": [
    {"decision": "...", "reasoning": "..."}
  ]
}
"#;
```

### 3. Few-shot示例

```rust
let examples = r#"
示例1：
输入：用户说他喜欢深色主题
输出：{"content": "用户偏好深色主题", "confidence": 0.95}

示例2：
输入：项目使用Rust语言开发
输出：{"content": "项目技术栈包含Rust", "confidence": 1.0}
"#;
```

## 提示模板管理

### PromptTemplate结构

```rust
pub struct PromptTemplate {
    pub system: String,
    pub user: String,
    pub format: String,
    pub examples: Option<String>,
}

impl PromptTemplate {
    pub fn render(&self, context: &Context) -> String {
        // 渲染提示，替换变量
    }
}
```

### 常用模板

**事实提取模板**:
```rust
const FACT_EXTRACTION_TEMPLATE: &str = r#"
从以下对话中提取事实性信息：

{conversation}

输出JSON数组，每个事实包含：
- content: 事实内容
- confidence: 置信度(0-1)
- source: 来源消息ID
"#;
```

**决策提取模板**:
```rust
const DECISION_EXTRACTION_TEMPLATE: &str = r#"
识别对话中的决策点：

{conversation}

输出JSON数组，每个决策包含：
- decision: 决策内容
- reasoning: 决策理由
- alternatives: 考虑的备选方案
- outcome: 决策结果（如有）
"#;
```

## 错误处理

```rust
pub enum LLMError {
    ApiError(String),
    InvalidResponse(String),
    RateLimitExceeded,
    Timeout,
    ParseError(serde_json::Error),
}

impl From<rig_core::Error> for LLMError {
    fn from(err: rig_core::Error) -> Self {
        // 错误转换
    }
}
```

## 性能优化

### 1. 批量处理

```rust
async fn process_batch(&self, items: Vec<String>) -> Result<Vec<Response>> {
    // 批量发送请求
    let tasks: Vec<_> = items
        .into_iter()
        .map(|item| self.generate(item))
        .collect();
    
    // 并发等待
    futures::future::try_join_all(tasks).await
}
```

### 2. 超时控制

```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30),
    client.generate(prompt)
).await??;
```

### 3. 重试机制

```rust
async fn generate_with_retry(&self, prompt: &str, max_retries: u32) -> Result<String> {
    for attempt in 0..max_retries {
        match self.generate(prompt).await {
            Ok(response) => return Ok(response),
            Err(e) if attempt < max_retries - 1 => {
                tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## 配置示例

### OpenAI官方

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model_efficient = "gpt-4"
temperature = 0.1
max_tokens = 4096
```

### Azure OpenAI

```toml
[llm]
api_base_url = "https://your-resource.openai.azure.com/openai/deployments/gpt-4"
api_key = "your-azure-key"
model_efficient = "gpt-4"
temperature = 0.1
max_tokens = 4096
```

### 自部署（Ollama）

```toml
[llm]
api_base_url = "http://localhost:11434/v1"
api_key = "ollama"  # 任意值
model_efficient = "llama2"
temperature = 0.1
max_tokens = 2048
```

## 成本优化

### 1. Token使用追踪

```rust
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

impl LLMClient {
    pub fn get_usage(&self) -> TokenUsage {
        // 获取累计使用量
    }
}
```

### 2. 提示压缩

```rust
fn compress_context(context: &str) -> String {
    // 移除冗余空白
    // 缩短重复内容
    // 保留关键信息
}
```

### 3. 缓存响应

```rust
struct ResponseCache {
    cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
}

struct CachedResponse {
    response: String,
    expires_at: DateTime<Utc>,
}
```

## 安全性

### 1. API密钥管理

```rust
// ✅ 从环境变量读取
let api_key = std::env::var("LLM_API_KEY")?;

// ✅ 从配置文件读取
let config = Config::load("config.toml")?;

// ❌ 不要硬编码
let api_key = "sk-hardcoded";  // 永远不要这样做
```

### 2. 内容过滤

```rust
fn sanitize_input(input: &str) -> String {
    // 移除敏感信息
    // 过滤有害内容
    // 限制长度
}
```

## 监控和日志

```rust
use tracing::{info, warn, error};

impl LLMClient {
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        info!("Generating response for prompt length: {}", prompt.len());
        
        let start = Instant::now();
        let response = self.agent.prompt(prompt).await?;
        let duration = start.elapsed();
        
        info!(
            "Generation completed in {:?}, tokens: {}",
            duration,
            response.len()
        );
        
        Ok(response)
    }
}
```

---

详见源码: [cortex-mem-core/src/llm/](../../cortex-mem-core/src/llm/)
