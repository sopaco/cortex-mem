# Rig框架Extractor使用指南

## 概述

Rig是一个用于构建LLM驱动应用程序的Rust框架，提供了强大的extractor机制，可以将大模型的非结构化文本输出转换为类型安全的结构化数据。本文档记录了在memo-rs项目中集成和使用rig extractor的经验和最佳实践。

## 核心概念

### 1. Extractor机制

Rig的extractor机制允许开发者：
- 定义结构化的数据类型
- 自动生成JSON schema
- 将LLM的文本输出解析为强类型对象
- 提供类型安全的错误处理

### 2. 核心组件

- **Extractor**: 结构化数据提取器
- **JsonSchema**: 自动生成JSON schema的trait
- **CompletionClient**: LLM客户端接口
- **Agent**: LLM代理，支持extractor功能

## 使用方式

### 1. 定义结构化数据类型

首先定义用于extractor的数据结构，必须实现以下trait：
- `serde::Deserialize`
- `serde::Serialize` 
- `schemars::JsonSchema`

```rust
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredFactExtraction {
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetailedFactExtraction {
    pub facts: Vec<StructuredFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredFact {
    pub content: String,
    pub importance: f32,
    pub category: String,
    pub entities: Vec<String>,
    pub source_role: String,
}
```

### 2. 创建Extractor

使用rig客户端创建extractor：

```rust
let extractor = client
    .extractor_completions_api::<StructuredFactExtraction>("gpt-4")
    .preamble(system_prompt)
    .max_tokens(2000)
    .build();
```

### 3. 执行提取

调用extract方法进行结构化数据提取：

```rust
let result: StructuredFactExtraction = extractor.extract("").await?;
```

## 实际应用示例

### 1. LLM客户端集成

在LLM客户端trait中添加extractor方法：

```rust
#[async_trait]
pub trait LLMClient: Send + Sync + dyn_clone::DynClone {
    // 传统方法
    async fn complete(&self, prompt: &str) -> Result<String>;
    
    // 新的extractor方法
    async fn extract_structured_facts(&self, prompt: &str) -> Result<StructuredFactExtraction>;
    async fn extract_detailed_facts(&self, prompt: &str) -> Result<DetailedFactExtraction>;
    async fn classify_memory(&self, prompt: &str) -> Result<MemoryClassification>;
}
```

### 2. 实现extractor方法

在OpenAI客户端实现中：

```rust
async fn extract_structured_facts(&self, prompt: &str) -> Result<StructuredFactExtraction> {
    let extractor = self
        .client
        .extractor_completions_api::<StructuredFactExtraction>("gpt-4")
        .preamble(prompt)
        .max_tokens(2000)
        .build();

    extractor
        .extract("")
        .await
        .map_err(|e| MemoryError::LLM(e.to_string()))
}
```

### 3. 重构现有代码

将原来的字符串处理逻辑替换为extractor调用：

```rust
// 原来的方式
let response = self.llm_client.complete(&prompt).await?;
let facts = self.parse_facts_response(&response)?;

// 使用extractor的方式
match self.llm_client.extract_structured_facts(&prompt).await {
    Ok(structured_facts) => {
        let facts = self.parse_structured_facts(structured_facts);
        // 处理结构化数据
    }
    Err(e) => {
        // Fallback到传统方法
        let response = self.llm_client.complete(&prompt).await?;
        let facts = self.parse_facts_response_fallback(&response)?;
    }
}
```

## 最佳实践

### 1. Fallback机制

始终提供fallback机制，确保在extractor失败时系统仍能正常工作：

```rust
match self.llm_client.extract_structured_facts(&prompt).await {
    Ok(result) => Ok(result),
    Err(e) => {
        debug!("Rig extractor failed, falling back to traditional method: {}", e);
        // 使用传统方法作为fallback
        self.traditional_method(prompt).await
    }
}
```

### 2. 类型设计原则

- 保持数据结构简单明了
- 使用明确的字段名称
- 为可选字段提供合理的默认值
- 考虑LLM的理解能力，避免过于复杂的嵌套结构

### 3. Prompt设计

为extractor设计专门的prompt：

```rust
fn build_extraction_prompt(&self, content: &str) -> String {
    format!(
        r#"Extract structured information from the following text.
        
Return the result in the specified JSON format.

Text: {}

JSON Response:"#,
        content
    )
}
```

### 4. 错误处理

实现适当的错误处理和日志记录：

```rust
use tracing::debug;

match extractor.extract("").await {
    Ok(result) => {
        debug!("Successfully extracted {} items", result.items.len());
        Ok(result)
    }
    Err(e) => {
        debug!("Extractor failed: {}, using fallback", e);
        self.fallback_extraction(content).await
    }
}
```

## 性能考虑

### 1. Token使用

Extractor通常需要更多的token，因为包含了schema信息。合理设置max_tokens：

```rust
let extractor = client
    .extractor_completions_api::<MyType>("gpt-4")
    .max_tokens(2000)  // 根据数据结构复杂度调整
    .build();
```

### 2. 批处理

对于大量数据，考虑实现批处理逻辑：

```rust
async fn extract_batch(&self, prompts: &[String]) -> Result<Vec<ExtractionResult>> {
    let mut results = Vec::new();
    for prompt in prompts {
        let result = self.extract_single(prompt).await?;
        results.push(result);
    }
    Ok(results)
}
```

## 依赖配置

确保在Cargo.toml中包含必要的依赖：

```toml
[dependencies]
rig-core = "0.23"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "1.0", features = ["derive"] }
async-trait = "0.1"
```

## 常见问题和解决方案

### 1. Schema冲突

确保数据结构不会产生循环引用或过于复杂的嵌套。

### 2. 解析失败

当LLM返回的内容不符合预期格式时，extractor会失败。此时应该：
- 检查prompt是否清晰
- 简化数据结构
- 提供更好的示例

### 3. 性能优化

- 对于简单场景，继续使用传统方法
- 对于复杂结构化数据，使用extractor
- 考虑缓存频繁使用的提取结果

## 总结

Rig的extractor机制为处理LLM输出提供了类型安全、自动化的解决方案。通过合理的设计和实现，可以显著提高代码的可靠性和可维护性。关键是要：

1. 设计合适的数据结构
2. 实现可靠的fallback机制
3. 优化prompt设计
4. 处理好错误情况

这种方式特别适合需要从大量非结构化文本中提取结构化信息的应用场景，如记忆管理、数据提取、内容分析等。