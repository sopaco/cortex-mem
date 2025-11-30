# 领域模块划分与技术方案详细说明

## 模块架构总览

Memo-RS系统采用领域驱动设计（DDD）思想，将复杂的记忆管理系统分解为多个高内聚、低耦合的领域模块。每个模块负责特定的业务功能，通过清晰的接口进行交互。

### 模块层次结构

```
应用层 (Application Layer)
├── CLI接口模块 (memo-cli)
├── HTTP API模块 (memo-service)  
└── Rig集成模块 (memo-rig)

核心层 (Core Layer)
├── 记忆管理域 (Memory Domain)
├── 向量存储域 (Vector Store Domain)
├── LLM集成域 (LLM Integration Domain)
└── 配置管理域 (Configuration Domain)

基础设施层 (Infrastructure Layer)
├── Qdrant存储
├── OpenAI API
└── 文件系统
```

## 1. 记忆管理域 (Memory Domain)

### 1.1 领域概述
记忆管理域是系统的核心业务域，负责记忆的创建、存储、检索、更新和删除操作。

### 1.2 核心模块

#### MemoryManager - 记忆管理器
**职责**: 协调各子组件，提供统一的记忆操作接口

**关键类**:
```rust
pub struct MemoryManager {
    vector_store: Box<dyn VectorStore>,
    llm_client: Box<dyn LLMClient>,  
    config: MemoryConfig,
    fact_extractor: Box<dyn FactExtractor>,
    memory_updater: Box<dyn MemoryUpdater>,
    importance_evaluator: Box<dyn ImportanceEvaluator>,
    duplicate_detector: Box<dyn DuplicateDetector>,
    memory_classifier: Box<dyn MemoryClassifier>,
}
```

**设计模式**:
- **外观模式**: 为复杂的记忆操作提供简化接口
- **策略模式**: 支持不同的记忆处理策略
- **工厂模式**: 动态创建子组件实例

**核心技术实现**:

```rust
impl MemoryManager {
    /// 核心记忆存储流程
    pub async fn add_memory(
        &self,
        messages: &[Message],
        metadata: MemoryMetadata,
    ) -> Result<Vec<MemoryResult>> {
        // 1. 策略选择
        let extraction_strategy = self.analyze_extraction_strategy(messages, &metadata);
        
        // 2. 事实提取
        let extracted_facts = match extraction_strategy {
            ExtractionStrategy::DualChannel => {
                let user_facts = self.fact_extractor.extract_user_facts(messages).await?;
                let assistant_facts = self.fact_extractor.extract_meaningful_assistant_facts(messages).await?;
                [user_facts, assistant_facts].concat()
            }
            // ... 其他策略
        };
        
        // 3. 智能过滤
        let filtered_facts = self.intelligent_fact_filtering(extracted_facts).await?;
        
        // 4. 记忆更新
        self.update_memories_with_facts(filtered_facts, &metadata).await
    }
}
```

#### FactExtractor - 事实提取器
**职责**: 从对话中智能提取结构化的关键信息

**技术方案**:

1. **多策略提取**:
   ```rust
   pub enum ExtractionStrategy {
       DualChannel,        // 用户+助手双通道
       UserOnly,          // 仅用户信息
       AssistantOnly,     // 仅助手信息
       ProceduralMemory,  // 程序型记忆
   }
   ```

2. **智能提示工程**:
   ```rust
   fn build_user_memory_prompt(&self, messages: &[Message]) -> String {
       format!(
           r#"You are a Personal Information Organizer, specialized in accurately storing facts, 
           user memories, and preferences. Generate facts SOLELY based on USER messages.
           
           Types to Remember:
           1. Personal Preferences
           2. Important Personal Details
           3. Plans and Intentions
           4. Activity Preferences
           5. Professional Details
           
           Conversation: {}"#,
           parse_messages(messages)
       )
   }
   ```

3. **严格过滤机制**:
   ```rust
   async fn intelligent_fact_filtering(&self, facts: Vec<ExtractedFact>) -> Result<Vec<ExtractedFact>> {
       // 语义去重
       for fact in &facts {
           if self.are_facts_semantically_similar(fact.content, existing_content) {
               continue; // 跳过语义重复
           }
       }
       
       // 重要性阈值过滤
       if fact.importance >= 0.5 {
           filtered_facts.push(fact.clone());
       }
   }
   ```

#### MemoryUpdater - 记忆更新器
**职责**: 智能决策新信息与现有记忆的交互方式

**更新策略优先级**:
1. **IGNORE** - 忽略冗余信息
2. **MERGE** - 合并相关信息
3. **UPDATE** - 更新现有记忆
4. **CREATE** - 创建新记忆

**UUID映射解决**:
```rust
struct UuidMapping {
    temp_to_real: HashMap<String, String>, // 临时UUID -> 真实UUID
    real_to_temp: HashMap<String, String>, // 真实UUID -> 临时UUID
}

impl UuidMapping {
    fn create_from_existing_memories(&mut self, memories: &[ScoredMemory]) {
        for (idx, memory) in memories.iter().enumerate() {
            let temp_uuid = idx.to_string(); // 使用数字索引
            let real_uuid = memory.memory.id.clone();
            self.temp_to_real.insert(temp_uuid, real_uuid);
            self.real_to_temp.insert(real_uuid, temp_uuid);
        }
    }
}
```

#### ImportanceEvaluator - 重要性评估器
**职责**: 评估记忆内容的重要性，为记忆排序和过滤提供依据

**评估维度**:
- 信息新颖性
- 用户明确性
- 上下文相关性
- 时间敏感性

**实现方案**:
```rust
#[async_trait]
pub trait ImportanceEvaluator: Send + Sync {
    async fn evaluate_importance(&self, memory: &Memory) -> Result<f32>;
}

pub struct LLMImportanceEvaluator {
    llm_client: Box<dyn LLMClient>,
    hybrid_threshold: f32,
}

impl ImportanceEvaluator for LLMImportanceEvaluator {
    async fn evaluate_importance(&self, memory: &Memory) -> Result<f32> {
        if memory.content.len() < self.hybrid_threshold as usize {
            // 短文本使用启发式算法
            self.heuristic_importance(memory)
        } else {
            // 长文本使用LLM评估
            self.llm_importance(memory).await
        }
    }
}
```

#### DuplicateDetector - 去重检测器
**职责**: 检测和合并重复或高度相似的记忆

**检测策略**:
```rust
#[async_trait]
pub trait DuplicateDetector: Send + Sync {
    async fn detect_duplicates(&self, memory: &Memory) -> Result<Vec<Memory>>;
    async fn merge_memories(&self, memories: &[Memory]) -> Result<Memory>;
}

pub struct SemanticDuplicateDetector {
    similarity_threshold: f32,
    merge_threshold: f32,
    llm_client: Box<dyn LLMClient>,
}

impl DuplicateDetector for SemanticDuplicateDetector {
    async fn detect_duplicates(&self, memory: &Memory) -> Result<Vec<Memory>> {
        // 1. 语义相似度搜索
        let query_embedding = self.llm_client.embed(&memory.content).await?;
        let similar_memories = self.search_similar_memories(&query_embedding).await?;
        
        // 2. 过滤高相似度记忆
        similar_memories.into_iter()
            .filter(|m| self.calculate_similarity(&memory.content, &m.content) >= self.similarity_threshold)
            .collect()
    }
}
```

#### MemoryClassifier - 记忆分类器
**职责**: 自动分类记忆内容，提取实体和主题信息

**分类类型**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Conversational, // 对话型
    Procedural,     // 程序型  
    Factual,        // 事实型
    Semantic,       // 语义型
    Episodic,       // 情景型
    Personal,       // 个人型
}
```

### 1.3 数据模型

#### Memory数据结构
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub id: String,                    // 记忆唯一标识
    pub content: String,               // 记忆内容
    pub embedding: Vec<f32>,           // 向量表示
    pub metadata: MemoryMetadata,      // 元数据
    pub created_at: DateTime<Utc>,     // 创建时间
    pub updated_at: DateTime<Utc>,     // 更新时间
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryMetadata {
    pub user_id: Option<String>,       // 用户ID
    pub agent_id: Option<String>,      // 智能体ID  
    pub run_id: Option<String>,        // 运行ID
    pub actor_id: Option<String>,      // 执行者ID
    pub role: Option<String>,          // 角色
    pub memory_type: MemoryType,       // 记忆类型
    pub hash: String,                  // 内容哈希
    pub importance_score: f32,         // 重要性评分
    pub entities: Vec<String>,         // 实体列表
    pub topics: Vec<String>,           // 主题列表
    pub custom: HashMap<String, serde_json::Value>, // 自定义字段
}
```

## 2. 向量存储域 (Vector Store Domain)

### 2.1 领域概述
向量存储域负责向量的存储、索引和相似性搜索，提供高性能的语义检索能力。

### 2.2 核心模块

#### VectorStore - 向量存储接口
**职责**: 定义向量存储的统一接口，支持多种存储后端

**核心接口**:
```rust
#[async_trait]
pub trait VectorStore: Send + Sync + dyn_clone::DynClone {
    async fn insert(&self, memory: &Memory) -> Result<()>;
    async fn search(&self, query_vector: &[f32], filters: &Filters, limit: usize) -> Result<Vec<ScoredMemory>>;
    async fn search_with_threshold(&self, query_vector: &[f32], filters: &Filters, limit: usize, score_threshold: Option<f32>) -> Result<Vec<ScoredMemory>>;
    async fn update(&self, memory: &Memory) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Memory>>;
    async fn list(&self, filters: &Filters, limit: Option<usize>) -> Result<Vec<Memory>>;
    async fn health_check(&self) -> Result<bool>;
}
```

#### QdrantVectorStore - Qdrant实现
**职责**: 基于Qdrant的向量存储实现

**技术特性**:
- 高性能HNSW索引
- 分布式存储支持
- 实时向量更新
- 多维度过滤支持

**实现方案**:
```rust
pub struct QdrantVectorStore {
    client: QdrantClient,
    collection_name: String,
    embedding_dim: usize,
}

impl VectorStore for QdrantVectorStore {
    async fn search(&self, query_vector: &[f32], filters: &Filters, limit: usize) -> Result<Vec<ScoredMemory>> {
        let mut qdrant_filter = self.build_qdrant_filter(filters);
        
        let search_result = self.client.search_points(&SearchPoints {
            collection_name: &self.collection_name,
            vector: query_vector.to_vec(),
            limit: limit as u64,
            with_payload: Some(true.into()),
            with_vectors: Some(false.into()),
            filter: Some(qdrant_filter),
            ..Default::default()
        }).await?;
        
        self.parse_search_results(search_result)
    }
    
    fn build_qdrant_filter(&self, filters: &Filters) -> Filter {
        let mut conditions = Vec::new();
        
        if let Some(user_id) = &filters.user_id {
            conditions.push(Condition::matches("metadata.user_id", user_id));
        }
        
        if let Some(memory_type) = &filters.memory_type {
            conditions.push(Condition::matches("metadata.memory_type", memory_type.to_string()));
        }
        
        Filter::must(conditions)
    }
}
```

### 2.3 搜索优化策略

#### 多维度过滤
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filters {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub min_importance: Option<f32>,
    pub max_importance: Option<f32>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub entities: Option<Vec<String>>,
    pub topics: Option<Vec<String>>,
    pub custom: HashMap<String, serde_json::Value>,
}
```

#### 相似度阈值控制
```rust
pub async fn search_with_threshold(
    &self,
    query_vector: &[f32],
    filters: &Filters,
    limit: usize,
    score_threshold: Option<f32>,
) -> Result<Vec<ScoredMemory>> {
    let threshold = score_threshold.unwrap_or(self.default_threshold);
    let results = self.client.search_points(&SearchPoints {
        collection_name: &self.collection_name,
        vector: query_vector.to_vec(),
        limit: (limit * 3) as u64, // 获取更多结果用于后过滤
        with_payload: Some(true.into()),
        with_vectors: Some(true.into()),
        // ... 其他参数
    }).await?;
    
    // 应用相似度阈值过滤
    let filtered_results: Vec<_> = results
        .into_iter()
        .filter(|result| result.score >= threshold)
        .take(limit as u64)
        .collect();
        
    self.parse_search_results(filtered_results)
}
```

## 3. LLM集成域 (LLM Integration Domain)

### 3.1 领域概述
LLM集成域负责与各种大语言模型服务集成，提供文本生成、嵌入生成、关键词提取等能力。

### 3.2 核心模块

#### LLMClient - LLM客户端接口
**职责**: 统一不同LLM提供商的接口

**核心接口**:
```rust
#[async_trait]
pub trait LLMClient: Send + Sync + dyn_clone::DynClone {
    async fn complete(&self, prompt: &str) -> Result<String>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    async fn extract_keywords(&self, content: &str) -> Result<Vec<String>>;
    async fn summarize(&self, content: &str, max_length: Option<usize>) -> Result<String>;
    async fn health_check(&self) -> Result<bool>;
}
```

#### OpenAILLMClient - OpenAI实现
**职责**: 基于OpenAI API的LLM客户端实现

**技术方案**:
```rust
pub struct OpenAILLMClient {
    completion_model: Agent<CompletionModel>,
    embedding_model: OpenAIEmbeddingModel,
}

impl LLMClient for OpenAILLMClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let response = self.completion_model
            .prompt(prompt)
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))?;
            
        Ok(response.trim().to_string())
    }
    
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embedding = self.embedding_model
            .embeddings(vec![text.to_string()])
            .await
            .map_err(|e| MemoryError::LLM(e.to_string()))?;
            
        Ok(embedding.data[0].embedding.clone())
    }
    
    async fn extract_keywords(&self, content: &str) -> Result<Vec<String>> {
        let prompt = format!(
            r#"Extract key concepts and entities from the following text. 
            Return only the keywords as a JSON array of strings:
            
            Text: {}
            
            Keywords:"#,
            content
        );
        
        let response = self.complete(&prompt).await?;
        self.parse_keywords_response(&response)
    }
}
```

### 3.3 错误处理与重试机制

#### 智能重试策略
```rust
async fn complete_with_retry(&self, prompt: &str, max_retries: usize) -> Result<String> {
    let mut last_error = None;
    
    for attempt in 0..max_retries {
        match self.complete(prompt).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                last_error = Some(e);
                if self.is_retryable_error(&e) && attempt < max_retries - 1 {
                    let delay = Duration::from_secs(2u64.pow(attempt as u32));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    Err(MemoryError::LLM(format!("Failed after {} retries: {:?}", max_retries, last_error)))
}
```

## 4. 配置管理域 (Configuration Domain)

### 4.1 领域概述
配置管理域负责系统各组件的配置加载、验证和管理。

### 4.2 核心模块

#### Config - 主配置结构
**职责**: 集中管理所有配置信息

**配置结构**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub qdrant: QdrantConfig,
    pub llm: LLMConfig,
    pub server: ServerConfig,
    pub embedding: EmbeddingConfig,
    pub memory: MemoryConfig,
}
```

#### MemoryConfig - 记忆配置
**职责**: 记忆管理相关配置

**关键配置项**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_memories: usize,
    pub similarity_threshold: f32,
    pub max_search_results: usize,
    pub enable_deduplication: bool,
    pub memory_ttl_hours: Option<u64>,
    pub auto_summary_threshold: usize,
    pub auto_enhance: bool,
    pub deduplicate: bool,
    pub merge_threshold: f32,
    pub search_similarity_threshold: Option<f32>,
}

impl MemoryConfig {
    pub fn default() -> Self {
        MemoryConfig {
            max_memories: 10000,
            similarity_threshold: 0.65,    // 降低阈值提高召回率
            max_search_results: 50,
            enable_deduplication: true,
            memory_ttl_hours: None,
            auto_summary_threshold: 32768,
            auto_enhance: true,
            deduplicate: true,
            merge_threshold: 0.75,         // 平衡合并粒度
            search_similarity_threshold: None,
        }
    }
}
```

### 4.3 环境变量配置

#### 配置加载机制
```rust
impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            qdrant: QdrantConfig::from_env()?,
            llm: LLMConfig::from_env()?,
            server: ServerConfig::from_env()?,
            embedding: EmbeddingConfig::from_env()?,
            memory: MemoryConfig::from_env()?,
        })
    }
}

impl LLMConfig {
    pub fn from_env() -> Result<Self> {
        Ok(LLMConfig {
            api_base_url: env::var("MEMO_LLM_API_BASE_URL").map_err(|_| {
                MemoryError::config("MEMO_LLM_API_BASE_URL environment variable is required")
            })?,
            api_key: env::var("MEMO_LLM_API_KEY").map_err(|_| {
                MemoryError::config("MEMO_LLM_API_KEY environment variable is required")
            })?,
            model_efficient: env::var("MEMO_LLM_MODEL")
                .unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
            temperature: env::var("LLM_TEMPERATURE")
                .unwrap_or_else(|_| "0.1".to_string())
                .parse()
                .map_err(|e| MemoryError::config(format!("Invalid temperature: {}", e)))?,
            max_tokens: env::var("LLM_MAX_TOKENS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|e| MemoryError::config(format!("Invalid max tokens: {}", e)))?,
        })
    }
}
```

## 5. 应用接口层

### 5.1 CLI接口 (memo-cli)

#### 命令结构
```
memo-cli
├── add     # 添加记忆
├── search  # 搜索记忆
├── list    # 列出记忆
├── get     # 获取记忆
├── update  # 更新记忆
├── delete  # 删除记忆
└── config  # 配置管理
```

#### 实现示例
```rust
#[derive(Parser)]
#[command(name = "memo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        #[arg(short, long)]
        content: String,
        #[arg(short, long)]
        user_id: String,
        #[arg(short, long)]
        memory_type: Option<String>,
    },
    Search {
        #[arg(short, long)]
        query: String,
        #[arg(short, long)]
        user_id: Option<String>,
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

impl Cli {
    async fn run(self) -> Result<()> {
        match self.command {
            Commands::Add { content, user_id, memory_type } => {
                self.add_memory(content, user_id, memory_type).await
            }
            Commands::Search { query, user_id, limit } => {
                self.search_memory(query, user_id, limit).await
            }
        }
    }
}
```

### 5.2 HTTP API (memo-service)

#### API结构
```
/health                    # 健康检查
/memories                  # 记忆管理
├── POST /memories         # 创建记忆
├── GET /memories          # 列出记忆
├── POST /memories/search  # 搜索记忆
├── GET /memories/{id}     # 获取记忆
├── PUT /memories/{id}     # 更新记忆
└── DELETE /memories/{id}  # 删除记忆
```

#### Web服务实现
```rust
use axum::{Router, routing::{post, get}, extract::{State, Json}};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateMemoryRequest {
    content: String,
    user_id: String,
    memory_type: Option<String>,
}

#[derive(Serialize)]
struct CreateMemoryResponse {
    id: String,
    content: String,
    created_at: String,
}

pub async fn create_memory(
    State(state): State<AppState>,
    Json(request): Json<CreateMemoryRequest>,
) -> Result<Json<CreateMemoryResponse>, ApiError> {
    let memory_type = match request.memory_type.as_deref() {
        Some("conversational") => MemoryType::Conversational,
        Some("procedural") => MemoryType::Procedural,
        Some("factual") => MemoryType::Factual,
        _ => MemoryType::Conversational,
    };
    
    let metadata = MemoryMetadata::new(memory_type)
        .with_user_id(request.user_id);
    
    let messages = vec![Message::user(request.content)];
    let results = state.memory_manager.add_memory(&messages, metadata).await?;
    
    if let Some(result) = results.first() {
        Ok(Json(CreateMemoryResponse {
            id: result.id.clone(),
            content: result.memory.clone(),
            created_at: Utc::now().to_rfc3339(),
        }))
    } else {
        Err(ApiError::InternalServerError)
    }
}
```

### 5.3 Rig集成 (memo-rig)

#### 工具集成
```rust
pub struct MemoryTool {
    memory_manager: Arc<MemoryManager>,
    config: MemoryToolConfig,
}

#[derive(Tool)]
pub struct MemoryArgs {
    #[arg(help = "Action to perform (search, add, list, etc.)")]
    pub action: String,
    
    #[arg(help = "Search query or memory content")]
    pub query: Option<String>,
    
    #[arg(help = "User ID")]
    pub user_id: Option<String>,
    
    #[arg(help = "Maximum number of results")]
    pub limit: Option<usize>,
    
    #[arg(help = "Memory ID for get/update/delete operations")]
    pub memory_id: Option<String>,
}

#[async_trait]
impl Tool for MemoryTool {
    async fn call(&self, args: MemoryArgs) -> Result<String, Box<dyn std::error::Error>> {
        match args.action.as_str() {
            "search" => self.handle_search(args).await,
            "add" => self.handle_add(args).await,
            "list" => self.handle_list(args).await,
            _ => Err(format!("Unknown action: {}", args.action).into()),
        }
    }
    
    async fn handle_search(&self, args: MemoryArgs) -> Result<String, Box<dyn std::error::Error>> {
        let query = args.query.ok_or("Query is required for search")?;
        let filters = Filters::for_user(&args.user_id);
        
        let results = self.memory_manager.search(&query, &filters, args.limit.unwrap_or(10)).await?;
        
        let json_results: Vec<_> = results.into_iter().map(|result| {
            serde_json::json!({
                "id": result.memory.id,
                "content": result.memory.content,
                "score": result.score,
                "importance": result.memory.metadata.importance_score
            })
        }).collect();
        
        Ok(serde_json::to_string(&json_results)?)
    }
}
```

## 6. 跨模块协作

### 6.1 模块间依赖关系

```
MemoryManager (协调者)
    ↓ depends on
FactExtractor ←───────→ LLMClient (依赖)
MemoryUpdater ←───────→ VectorStore (依赖)
ImportanceEvaluator ←───────→ LLMClient (依赖)
DuplicateDetector ←───────→ VectorStore + LLMClient (依赖)
MemoryClassifier ←───────→ LLMClient (依赖)
```

### 6.2 事件驱动架构

#### 内存事件处理
```rust
pub enum MemoryEvent {
    Created(Memory),
    Updated { old: Memory, new: Memory },
    Deleted(Memory),
    Merged { sources: Vec<Memory>, target: Memory },
}

pub trait MemoryEventHandler: Send + Sync {
    async fn handle_event(&self, event: MemoryEvent) -> Result<()>;
}

pub struct MemoryEventBus {
    handlers: Vec<Box<dyn MemoryEventHandler>>,
}

impl MemoryEventBus {
    pub fn register_handler(&mut self, handler: Box<dyn MemoryEventHandler>) {
        self.handlers.push(handler);
    }
    
    pub async fn emit_event(&self, event: MemoryEvent) -> Result<()> {
        for handler in &self.handlers {
            handler.handle_event(event.clone()).await?;
        }
        Ok(())
    }
}
```

### 6.3 配置传递机制

#### 依赖注入
```rust
pub struct ServiceContainer {
    pub memory_manager: Arc<MemoryManager>,
    pub vector_store: Arc<dyn VectorStore>,
    pub llm_client: Arc<dyn LLMClient>,
}

impl ServiceContainer {
    pub fn new() -> Result<Self> {
        let config = Config::from_env()?;
        
        let llm_client = Arc::new(OpenAILLMClient::new(&config.llm, &config.embedding)?);
        let vector_store = Arc::new(QdrantVectorStore::new(&config.qdrant).await?);
        
        let memory_manager = Arc::new(MemoryManager::new(
            Box::new(dyn_clone::clone_box(vector_store.as_ref())),
            Box::new(dyn_clone::clone_box(llm_client.as_ref())),
            config.memory,
        ));
        
        Ok(Self {
            memory_manager,
            vector_store,
            llm_client,
        })
    }
}
```

---

**模块设计原则**: 高内聚、低耦合、单一职责、可测试、可扩展