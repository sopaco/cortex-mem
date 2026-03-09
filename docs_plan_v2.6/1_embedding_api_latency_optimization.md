# 嵌入 API 延迟优化技术方案

## 1. 问题分析

### 1.1 当前实现缺陷

**位置**: `cortex-mem-core/src/embedding/client.rs:103-105`

```rust
// 当前实现：强制等待1秒以避免限流
tokio::time::sleep(std::time::Duration::from_secs(1)).await;
```

这个固定延迟存在以下问题：

1. **过度等待**：API 可能允许更高的请求频率，固定 1 秒延迟过于保守
2. **无法应对突发流量**：批量索引 100 个文件至少需要 100 秒，用户体验极差
3. **浪费 API 配额**：在低频使用时，等待时间完全可以被利用
4. **不可配置**：硬编码延迟时间，无法根据不同 API 提供商调整

### 1.2 现有缓存层的问题

**位置**: `cortex-mem-core/src/embedding/cache.rs`

虽然已有 EmbeddingCache 实现，但存在以下问题：

1. **缓存层未集成**：缓存代码存在但未被实际使用
2. **缓存键过于简单**：仅使用文本哈希，未考虑模型版本变化
3. **缺少持久化**：重启后缓存丢失，需要重新请求 API
4. **缺少统计信息**：无法监控命中率和效果

### 1.3 批处理效率问题

当前 `embed_batch_chunked` 方法是顺序执行的：

```rust
pub async fn embed_batch_chunked(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    for chunk in texts.chunks(self.config.batch_size) {
        let embeddings = self.embed_batch(chunk).await?;  // 顺序执行
        all_embeddings.extend(embeddings);
    }
}
```

## 2. 解决方案

### 2.1 架构设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        优化后的嵌入生成架构                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   用户请求   │───►│  持久化缓存  │───►│ 自适应限流器 │───►│  批量并发   │  │
│  │             │    │  (磁盘+内存) │    │             │    │   调度器    │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│                            │                   │                  │         │
│                            ▼                   ▼                  ▼         │
│                     ┌──────────────────────────────────────────────────┐   │
│                     │                  嵌入 API                         │   │
│                     └──────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 自适应速率限制器

#### 2.2.1 令牌桶算法实现

```rust
// 文件: cortex-mem-core/src/embedding/rate_limiter.rs (新建)

use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

/// 速率限制配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 令牌桶容量（最大突发请求数）
    pub max_burst: u32,
    /// 令牌补充速率（每秒补充的令牌数）
    pub tokens_per_second: f32,
    /// 最小等待时间（毫秒），避免过于频繁的请求
    pub min_wait_ms: u64,
    /// 最大等待时间（毫秒），超过此时间报错
    pub max_wait_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_burst: 10,              // 允许突发 10 个请求
            tokens_per_second: 2.0,     // 每秒 2 个请求（根据 API 调整）
            min_wait_ms: 50,            // 最小间隔 50ms
            max_wait_ms: 30000,         // 最多等待 30 秒
        }
    }
}

/// OpenAI API 配置
impl RateLimitConfig {
    pub fn openai() -> Self {
        Self {
            max_burst: 20,
            tokens_per_second: 5.0,     // OpenAI 允许较高频率
            min_wait_ms: 20,
            max_wait_ms: 60000,
        }
    }
    
    /// Azure OpenAI 配置
    pub fn azure_openai() -> Self {
        Self {
            max_burst: 10,
            tokens_per_second: 3.0,
            min_wait_ms: 50,
            max_wait_ms: 60000,
        }
    }
    
    /// 本地模型配置（无限制）
    pub fn local() -> Self {
        Self {
            max_burst: 1000,
            tokens_per_second: 1000.0,
            min_wait_ms: 0,
            max_wait_ms: 0,
        }
    }
}

/// 自适应速率限制器
pub struct AdaptiveRateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

struct RateLimiterState {
    tokens: f32,
    last_refill: Instant,
    /// 自适应调整：记录最近的错误
    recent_errors: u32,
    /// 自适应调整：当前退避倍数
    backoff_multiplier: f32,
}

impl AdaptiveRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(RateLimiterState {
                tokens: config.max_burst as f32,
                last_refill: Instant::now(),
                recent_errors: 0,
                backoff_multiplier: 1.0,
            })),
        }
    }
    
    /// 获取一个令牌（阻塞直到可用）
    pub async fn acquire(&self) -> Result<(), crate::Error> {
        let wait_duration = self.try_acquire().await?;
        
        if let Some(duration) = wait_duration {
            if duration.as_millis() as u64 > self.config.max_wait_ms {
                return Err(crate::Error::Embedding(
                    "Rate limit exceeded: max wait time reached".to_string()
                ));
            }
            tokio::time::sleep(duration).await;
        }
        
        // 确保最小间隔
        if self.config.min_wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.config.min_wait_ms)).await;
        }
        
        Ok(())
    }
    
    async fn try_acquire(&self) -> Result<Option<Duration>, crate::Error> {
        let mut state = self.state.lock().await;
        
        // 补充令牌
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f32();
        let tokens_to_add = elapsed * self.config.tokens_per_second / state.backoff_multiplier;
        state.tokens = (state.tokens + tokens_to_add).min(self.config.max_burst as f32);
        state.last_refill = now;
        
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            Ok(None)
        } else {
            // 计算需要等待的时间
            let tokens_needed = 1.0 - state.tokens;
            let wait_secs = tokens_needed * state.backoff_multiplier / self.config.tokens_per_second;
            Ok(Some(Duration::from_secs_f32(wait_secs)))
        }
    }
    
    /// 报告错误（用于自适应调整）
    pub async fn report_error(&self, is_rate_limit_error: bool) {
        let mut state = self.state.lock().await;
        
        if is_rate_limit_error {
            state.recent_errors += 1;
            // 指数退避
            state.backoff_multiplier = (state.backoff_multiplier * 1.5).min(10.0);
            tracing::warn!(
                "Rate limit error detected, increasing backoff to {:.2}x",
                state.backoff_multiplier
            );
        }
    }
    
    /// 报告成功（恢复正常速率）
    pub async fn report_success(&self) {
        let mut state = self.state.lock().await;
        state.recent_errors = 0;
        // 逐渐恢复
        state.backoff_multiplier = (state.backoff_multiplier * 0.9).max(1.0);
    }
    
    /// 获取当前状态
    pub async fn status(&self) -> RateLimiterStatus {
        let state = self.state.lock().await;
        RateLimiterStatus {
            available_tokens: state.tokens,
            backoff_multiplier: state.backoff_multiplier,
            recent_errors: state.recent_errors,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterStatus {
    pub available_tokens: f32,
    pub backoff_multiplier: f32,
    pub recent_errors: u32,
}
```

### 2.3 持久化缓存层

#### 2.3.1 缓存存储设计

```rust
// 文件: cortex-mem-core/src/embedding/persistent_cache.rs (新建)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// 嵌入向量
    embedding: Vec<f32>,
    /// 模型名称（用于模型升级时失效）
    model_name: String,
    /// 模型版本（如果 API 提供）
    model_version: Option<String>,
    /// 创建时间戳
    created_at: i64,
    /// 最后访问时间戳
    last_accessed: i64,
    /// 访问次数
    access_count: u32,
    /// 文本哈希
    text_hash: String,
}

/// 持久化缓存配置
#[derive(Debug, Clone)]
pub struct PersistentCacheConfig {
    /// 缓存目录
    pub cache_dir: PathBuf,
    /// 内存中保留的热点条目数
    pub hot_cache_size: usize,
    /// TTL（秒）
    pub ttl_secs: u64,
    /// 是否启用磁盘持久化
    pub persist_to_disk: bool,
}

impl Default for PersistentCacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("./.embedding_cache"),
            hot_cache_size: 10000,
            ttl_secs: 86400 * 7,  // 7 天
            persist_to_disk: true,
        }
    }
}

/// 两级缓存：内存（热点）+ 磁盘（持久化）
pub struct PersistentEmbeddingCache {
    config: PersistentCacheConfig,
    model_name: String,
    /// 内存中的热点缓存（LRU）
    hot_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// 访问顺序（LRU 实现）
    access_order: Arc<RwLock<Vec<String>>>,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_requests: u64,
    pub hot_hits: u64,
    pub disk_hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl PersistentEmbeddingCache {
    pub async fn new(config: PersistentCacheConfig, model_name: String) -> Result<Self, crate::Error> {
        let cache = Self {
            config,
            model_name,
            hot_cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        };
        
        // 从磁盘加载缓存
        if cache.config.persist_to_disk {
            cache.load_from_disk().await?;
        }
        
        Ok(cache)
    }
    
    /// 生成缓存键（包含模型信息）
    fn make_key(&self, text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        self.model_name.hash(&mut hasher);
        
        format!("{}:{:016x}", self.model_name, hasher.finish())
    }
    
    /// 获取嵌入（优先从缓存）
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let key = self.make_key(text);
        
        // 统计请求
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += 1;
        }
        
        // 1. 先查内存缓存
        {
            let cache = self.hot_cache.read().await;
            if let Some(entry) = cache.get(&key) {
                if self.is_valid(entry) {
                    // 更新访问信息
                    self.record_access(&key).await;
                    
                    let mut stats = self.stats.write().await;
                    stats.hot_hits += 1;
                    
                    return Some(entry.embedding.clone());
                }
            }
        }
        
        // 2. 查磁盘缓存
        if self.config.persist_to_disk {
            if let Ok(Some(entry)) = self.load_from_disk_entry(&key).await {
                if self.is_valid(&entry) {
                    // 提升到内存缓存
                    self.put_hot_cache(key.clone(), entry.clone()).await;
                    
                    let mut stats = self.stats.write().await;
                    stats.disk_hits += 1;
                    
                    return Some(entry.embedding);
                }
            }
        }
        
        // 3. 缓存未命中
        {
            let mut stats = self.stats.write().await;
            stats.misses += 1;
        }
        
        None
    }
    
    /// 存入缓存
    pub async fn put(&self, text: &str, embedding: Vec<f32>) {
        let key = self.make_key(text);
        let text_hash = self.hash_text(text);
        
        let entry = CacheEntry {
            embedding,
            model_name: self.model_name.clone(),
            model_version: None,
            created_at: chrono::Utc::now().timestamp(),
            last_accessed: chrono::Utc::now().timestamp(),
            access_count: 1,
            text_hash,
        };
        
        // 存入内存缓存
        self.put_hot_cache(key.clone(), entry.clone()).await;
        
        // 持久化到磁盘
        if self.config.persist_to_disk {
            let _ = self.save_to_disk_entry(&key, &entry).await;
        }
    }
    
    /// 批量获取
    pub async fn get_batch(&self, texts: &[String]) -> Vec<Option<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.get(text).await);
        }
        results
    }
    
    /// 批量存储
    pub async fn put_batch(&self, texts: &[String], embeddings: Vec<Vec<f32>>) {
        for (text, embedding) in texts.iter().zip(embeddings.into_iter()) {
            self.put(text, embedding).await;
        }
    }
    
    /// 获取命中率
    pub async fn hit_rate(&self) -> f32 {
        let stats = self.stats.read().await;
        if stats.total_requests == 0 {
            return 0.0;
        }
        (stats.hot_hits + stats.disk_hits) as f32 / stats.total_requests as f32
    }
    
    /// 清理过期缓存
    pub async fn cleanup_expired(&self) -> usize {
        let mut cleaned = 0;
        
        // 清理内存缓存
        {
            let mut cache = self.hot_cache.write().await;
            let now = chrono::Utc::now().timestamp();
            let ttl = self.config.ttl_secs as i64;
            
            cache.retain(|_, entry| {
                let valid = now - entry.created_at < ttl;
                if !valid {
                    cleaned += 1;
                }
                valid
            });
        }
        
        // 清理磁盘缓存
        if self.config.persist_to_disk {
            cleaned += self.cleanup_disk_cache().await.unwrap_or(0);
        }
        
        cleaned
    }
    
    // ... 其他私有方法实现
    async fn is_valid(&self, entry: &CacheEntry) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - entry.created_at < self.config.ttl_secs as i64
            && entry.model_name == self.model_name
    }
    
    async fn put_hot_cache(&self, key: String, entry: CacheEntry) {
        let mut cache = self.hot_cache.write().await;
        let mut order = self.access_order.write().await;
        
        // LRU 淘汰
        while cache.len() >= self.config.hot_cache_size {
            if let Some(oldest) = order.first().cloned() {
                cache.remove(&oldest);
                order.remove(0);
                
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }
        
        cache.insert(key.clone(), entry);
        order.push(key);
    }
    
    async fn record_access(&self, key: &str) {
        let mut order = self.access_order.write().await;
        if let Some(pos) = order.iter().position(|k| k == key) {
            order.remove(pos);
            order.push(key.to_string());
        }
    }
    
    fn hash_text(&self, text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
    
    async fn load_from_disk(&self) -> Result<(), crate::Error> {
        // 实现：从磁盘加载缓存索引
        Ok(())
    }
    
    async fn load_from_disk_entry(&self, key: &str) -> Result<Option<CacheEntry>, crate::Error> {
        // 实现：从磁盘加载单个条目
        Ok(None)
    }
    
    async fn save_to_disk_entry(&self, key: &str, entry: &CacheEntry) -> Result<(), crate::Error> {
        // 实现：保存单个条目到磁盘
        Ok(())
    }
    
    async fn cleanup_disk_cache(&self) -> Result<usize, crate::Error> {
        // 实现：清理磁盘上的过期缓存
        Ok(0)
    }
}
```

### 2.4 集成改进后的 EmbeddingClient

```rust
// 文件: cortex-mem-core/src/embedding/client.rs (修改)

use crate::embedding::{AdaptiveRateLimiter, PersistentEmbeddingCache, RateLimitConfig};
use std::sync::Arc;

/// 增强的嵌入客户端
pub struct EmbeddingClientV2 {
    config: EmbeddingConfig,
    http_client: reqwest::Client,
    rate_limiter: AdaptiveRateLimiter,
    cache: Option<Arc<PersistentEmbeddingCache>>,
}

impl EmbeddingClientV2 {
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| crate::Error::Embedding(format!("Failed to create HTTP client: {}", e)))?;
        
        // 根据配置选择速率限制策略
        let rate_limit_config = match config.rate_limit_preset.as_str() {
            "openai" => RateLimitConfig::openai(),
            "azure" => RateLimitConfig::azure_openai(),
            "local" => RateLimitConfig::local(),
            _ => RateLimitConfig::default(),
        };
        
        Ok(Self {
            config,
            http_client,
            rate_limiter: AdaptiveRateLimiter::new(rate_limit_config),
            cache: None,  // 延迟初始化
        })
    }
    
    /// 启用持久化缓存
    pub async fn with_cache(mut self, cache_dir: impl Into<std::path::PathBuf>) -> Result<Self> {
        let cache_config = PersistentCacheConfig {
            cache_dir: cache_dir.into(),
            ..Default::default()
        };
        
        self.cache = Some(Arc::new(
            PersistentEmbeddingCache::new(cache_config, self.config.model_name.clone()).await?
        ));
        
        Ok(self)
    }
    
    /// 嵌入单个文本
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // 1. 尝试从缓存获取
        if let Some(cache) = &self.cache {
            if let Some(embedding) = cache.get(text).await {
                tracing::debug!("Embedding cache hit");
                return Ok(embedding);
            }
        }
        
        // 2. 获取速率限制许可
        self.rate_limiter.acquire().await?;
        
        // 3. 调用 API
        let embedding = self.call_api(&[text.to_string()]).await?
            .into_iter()
            .next()
            .ok_or_else(|| crate::Error::Embedding("No embedding returned".to_string()))?;
        
        // 4. 存入缓存
        if let Some(cache) = &self.cache {
            cache.put(text, embedding.clone()).await;
        }
        
        // 5. 报告成功（用于自适应调整）
        self.rate_limiter.report_success().await;
        
        Ok(embedding)
    }
    
    /// 批量嵌入（智能调度）
    pub async fn embed_batch_smart(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        
        // 1. 批量检查缓存
        let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut cache_miss_indices = Vec::new();
        let mut cache_miss_texts = Vec::new();
        
        if let Some(cache) = &self.cache {
            let cached = cache.get_batch(texts).await;
            for (i, result) in cached.into_iter().enumerate() {
                if let Some(embedding) = result {
                    results[i] = Some(embedding);
                } else {
                    cache_miss_indices.push(i);
                    cache_miss_texts.push(texts[i].clone());
                }
            }
        } else {
            cache_miss_indices = (0..texts.len()).collect();
            cache_miss_texts = texts.to_vec();
        }
        
        // 2. 批量处理未命中的请求
        if !cache_miss_texts.is_empty() {
            let mut all_embeddings = Vec::new();
            
            // 并发批处理（根据速率限制调整并发度）
            let concurrent_batches = self.calculate_concurrent_batches().await;
            for chunk in cache_miss_texts.chunks(self.config.batch_size * concurrent_batches) {
                // 获取批量许可
                for _ in 0..(chunk.len().div_ceil(self.config.batch_size)) {
                    self.rate_limiter.acquire().await?;
                }
                
                // 并发调用 API
                let embeddings = self.call_api(chunk).await?;
                all_embeddings.extend(embeddings);
            }
            
            // 3. 存入缓存并填充结果
            if let Some(cache) = &self.cache {
                cache.put_batch(&cache_miss_texts, all_embeddings.clone()).await;
            }
            
            for (idx, embedding) in cache_miss_indices.into_iter().zip(all_embeddings.into_iter()) {
                results[idx] = Some(embedding);
            }
        }
        
        Ok(results.into_iter().map(|opt| opt.unwrap()).collect())
    }
    
    /// 计算可以并发执行的批次数
    async fn calculate_concurrent_batches(&self) -> usize {
        let status = self.rate_limiter.status().await;
        // 根据可用令牌和退避倍数计算并发度
        let base_concurrent = (status.available_tokens as usize).max(1);
        let adjusted = (base_concurrent as f32 / status.backoff_multiplier) as usize;
        adjusted.max(1).min(4)  // 限制最大并发为 4
    }
    
    async fn call_api(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // ... 原有的 API 调用逻辑，移除固定延迟
        // ... 错误处理时调用 self.rate_limiter.report_error(true).await
    }
}
```

### 2.5 配置扩展

```toml
# config.toml 新增配置项

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model_name = "text-embedding-3-small"
batch_size = 32
timeout_secs = 30

# 新增：速率限制预设
rate_limit_preset = "openai"  # openai, azure, local, custom

# 新增：缓存配置
[embedding.cache]
enabled = true
cache_dir = "./.embedding_cache"
hot_cache_size = 10000
ttl_days = 7
persist_to_disk = true

# 新增：自定义速率限制（当 preset = "custom" 时使用）
[embedding.rate_limit]
max_burst = 10
tokens_per_second = 2.0
min_wait_ms = 50
max_wait_ms = 30000
```

## 3. 实现计划

### 3.1 文件结构

```
cortex-mem-core/src/embedding/
├── mod.rs              # 模块导出
├── client.rs           # 主客户端（修改）
├── cache.rs            # 内存缓存（保留，简化）
├── persistent_cache.rs # 持久化缓存（新建）
└── rate_limiter.rs     # 自适应速率限制（新建）
```

### 3.2 实现步骤

| 步骤 | 任务 | 预期效果 |
|------|------|----------|
| 1 | 实现自适应速率限制器 | 移除固定延迟，提升 30-50% 吞吐量 |
| 2 | 实现持久化缓存 | 减少 60-80% 重复 API 调用 |
| 3 | 重构 EmbeddingClient | 集成缓存和速率限制 |
| 4 | 添加配置支持 | 允许用户自定义策略 |
| 5 | 添加监控指标 | 支持性能调优 |

### 3.3 测试要点

1. **单元测试**：速率限制器的令牌补充和消耗逻辑
2. **集成测试**：缓存命中率、API 调用减少率
3. **压力测试**：批量索引 1000 个文件的耗时对比
4. **恢复测试**：模拟 429 错误时的自动退避

## 4. 预期收益

| 指标 | 当前 | 优化后 | 提升 |
|------|------|--------|------|
| 批量索引 100 文件 | ~100s | ~20-30s | 70%+ |
| 重复查询延迟 | 1-2s | <10ms | 99%+ |
| API 调用次数（有缓存） | 100% | 20-40% | 60-80% |
| 429 错误恢复时间 | 手动 | 自动 | N/A |

## 5. 兼容性说明

1. **向后兼容**：新配置项有默认值，旧配置文件无需修改
2. **渐进式启用**：缓存默认启用，可通过配置关闭
3. **平滑迁移**：保留原有 `embed_batch` 方法签名，新增 `embed_batch_smart`
