use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use std::sync::LazyLock;

// ── 全局速率限制器注册表 ─────────────────────────────────────────────────────

/// 全局速率限制器注册表
/// 
/// 使用 (api_base_url, api_key) 作为 key，确保同一个 embedding 服务
/// 的所有 EmbeddingClient 实例共享同一个 RateLimiter。
/// 这解决了多 tenant 场景下每个实例独立限流导致无法协调的问题。
static GLOBAL_RATE_LIMITERS: LazyLock<RwLock<HashMap<String, Arc<RateLimiter>>>> = 
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 生成全局 RateLimiter 的 key
fn make_global_key(api_base_url: &str, api_key: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    api_base_url.hash(&mut hasher);
    api_key.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// 获取或创建全局 RateLimiter
async fn get_or_create_global_rate_limiter(
    api_base_url: &str, 
    api_key: &str, 
    calls_per_minute: u32
) -> Arc<RateLimiter> {
    let key = make_global_key(api_base_url, api_key);
    
    // 先尝试读锁获取
    {
        let registry = GLOBAL_RATE_LIMITERS.read().await;
        if let Some(limiter) = registry.get(&key) {
            return limiter.clone();
        }
    }
    
    // 需要写锁创建
    let mut registry = GLOBAL_RATE_LIMITERS.write().await;
    // 双重检查，避免重复创建
    if let Some(limiter) = registry.get(&key) {
        return limiter.clone();
    }
    
    let limiter = Arc::new(RateLimiter::new(calls_per_minute));
    registry.insert(key, limiter.clone());
    limiter
}

// ── Embedding 速率限制器 ─────────────────────────────────────────────────────
///
/// 实现非阻塞的速率限制，使用 CAS 循环确保最小间隔。
/// 
/// ## 设计原则
/// - **不持有锁的同时 sleep**：避免阻塞其他请求
/// - **CAS 循环更新时间戳**：确保速率限制正确性
/// - **无阻塞等待**：多个请求可以并行等待，互不影响
///
/// 默认基准：30 次/分钟（即每次请求最小间隔 2000ms）。
pub struct RateLimiter {
    /// 下次允许请求的最早时间戳（UNIX 纳秒）
    next_allowed_nanos: AtomicU64,
    /// 每次请求之间的最小间隔（纳秒）
    min_interval_nanos: u64,
}

impl RateLimiter {
    /// 根据每分钟最大调用次数创建速率限制器
    pub fn new(calls_per_minute: u32) -> Self {
        let calls = calls_per_minute.max(1) as u64;
        let min_interval_nanos = 60_000_000_000u64 / calls; // 纳秒
        Self {
            next_allowed_nanos: AtomicU64::new(0),
            min_interval_nanos,
        }
    }

    /// 等待直到可以安全发出下一次请求
    /// 
    /// 使用 CAS 循环确保：
    /// 1. 先用 CAS 保留一个时间槽
    /// 2. 然后等待到该时间槽才返回
    /// 3. 保证请求之间有最小间隔，避免请求爆发
    pub async fn acquire(&self) {
        loop {
            let now_nanos = Self::current_nanos();
            let next_allowed = self.next_allowed_nanos.load(Ordering::SeqCst);
            
            // 计算我们想要保留的时间槽
            let our_slot = std::cmp::max(now_nanos, next_allowed) + self.min_interval_nanos;
            
            // 尝试用 CAS 保留时间槽
            match self.next_allowed_nanos.compare_exchange(
                next_allowed,
                our_slot,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    // 成功保留时间槽，等待到该时间
                    let wait_nanos = our_slot.saturating_sub(now_nanos);
                    if wait_nanos > 1_000_000 { // > 1ms
                        let wait = Duration::from_nanos(wait_nanos);
                        debug!("Rate limiter: acquired slot, waiting {:?}", wait);
                        tokio::time::sleep(wait).await;
                    }
                    return;
                }
                Err(_) => {
                    // 其他请求先更新了，重试（会读取新的 next_allowed）
                    // 短暂退避避免忙等
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
        }
    }

    /// 获取当前时间的 UNIX 纳秒
    fn current_nanos() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// 遇到 429 时额外退避（5 秒）
    pub async fn backoff_on_rate_limit(&self) {
        warn!("Rate limit hit (429), backing off for 5 seconds");
        // 延后下次允许时间
        let now_nanos = Self::current_nanos();
        let new_next = now_nanos + 5_000_000_000u64; // 5 秒
        self.next_allowed_nanos.store(new_next, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// ── 内嵌 LRU 缓存 ────────────────────────────────────────────────────────────

/// 缓存条目
struct CacheItem {
    embedding: Vec<f32>,
    created_at: Instant,
}

/// 内嵌 LRU 内存缓存（直接嵌入 EmbeddingClient，避免泛型包装）
struct InnerCache {
    entries: HashMap<String, CacheItem>,
    access_order: Vec<String>,
    max_entries: usize,
    ttl: Duration,
}

impl InnerCache {
    fn new(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// 查询缓存（只读，不更新 LRU 顺序）
    /// 
    /// 用于高并发场景下的缓存查询，牺牲 LRU 精度换取并发性能。
    /// 过期条目会在下次写入时被清理。
    fn peek(&self, key: &str) -> Option<Vec<f32>> {
        if let Some(item) = self.entries.get(key) {
            if item.created_at.elapsed() < self.ttl {
                return Some(item.embedding.clone());
            }
        }
        None
    }

    /// 写入缓存（满时淘汰最旧条目）
    fn put(&mut self, key: String, embedding: Vec<f32>) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            if let Some(oldest) = self.access_order.first().cloned() {
                self.entries.remove(&oldest);
                self.access_order.remove(0);
            }
        }
        self.entries.insert(
            key.clone(),
            CacheItem {
                embedding,
                created_at: Instant::now(),
            },
        );
        // 将新条目放到 access_order 末尾（如已存在则先移除）
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key);
    }

    /// 计算缓存键（含模型名，防止切换模型后复用旧向量）
    fn compute_key(model: &str, text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{}:{:x}", model, hasher.finish())
    }
}

// ── EmbeddingConfig ───────────────────────────────────────────────────────────

/// Embedding 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub api_base_url: String,
    pub api_key: String,
    pub model_name: String,
    /// 单批次最大文本数（默认 10）
    pub batch_size: usize,
    /// HTTP 超时（秒，默认 30）
    pub timeout_secs: u64,
    /// 每分钟最大 API 调用次数（默认 30，即单并发 2s 间隔）
    pub calls_per_minute: u32,
    /// 内存缓存最大条目数（默认 10000）
    pub cache_max_entries: usize,
    /// 内存缓存 TTL（秒，默认 3600）
    pub cache_ttl_secs: u64,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_base_url: std::env::var("EMBEDDING_API_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: std::env::var("EMBEDDING_API_KEY")
                .or_else(|_| std::env::var("LLM_API_KEY"))
                .unwrap_or_else(|_| "".to_string()),
            model_name: std::env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
            batch_size: 10,
            timeout_secs: 30,
            calls_per_minute: 30,
            cache_max_entries: 10_000,
            cache_ttl_secs: 3_600,
        }
    }
}

// ── EmbeddingClient ───────────────────────────────────────────────────────────

/// Embedding 客户端
///
/// 内置速率限制器（30 次/分钟单并发）和 LRU 内存缓存，
/// 对外 API 与原版保持一致。
pub struct EmbeddingClient {
    config: EmbeddingConfig,
    client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
    cache: Arc<RwLock<InnerCache>>,
}

impl EmbeddingClient {
    /// 创建新的 EmbeddingClient
    /// 
    /// 注意：所有使用相同 API endpoint 和 API key 的 EmbeddingClient 实例
    /// 会共享同一个全局 RateLimiter，确保跨实例的速率控制正确性。
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let calls_per_minute = config.calls_per_minute;
        let cache = Arc::new(RwLock::new(InnerCache::new(
            config.cache_max_entries,
            config.cache_ttl_secs,
        )));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| crate::Error::Embedding(format!("Failed to create HTTP client: {}", e)))?;

        info!(
            "EmbeddingClient initialized: model={}, rate_limit={}/min, cache={}entries/{}s",
            config.model_name,
            config.calls_per_minute,
            config.cache_max_entries,
            config.cache_ttl_secs,
        );

        Ok(Self {
            config,
            client,
            rate_limiter: Arc::new(RateLimiter::new(calls_per_minute)),
            cache,
        })
    }

    /// 创建新的 EmbeddingClient（异步版本，使用全局 RateLimiter）
    /// 
    /// 推荐使用此方法创建 EmbeddingClient，确保跨实例的速率控制正确性。
    pub async fn new_with_global_limiter(config: EmbeddingConfig) -> Result<Self> {
        let rate_limiter = get_or_create_global_rate_limiter(
            &config.api_base_url,
            &config.api_key,
            config.calls_per_minute,
        ).await;
        let cache = Arc::new(RwLock::new(InnerCache::new(
            config.cache_max_entries,
            config.cache_ttl_secs,
        )));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| crate::Error::Embedding(format!("Failed to create HTTP client: {}", e)))?;

        info!(
            "EmbeddingClient initialized (global limiter): model={}, rate_limit={}/min, cache={}entries/{}s",
            config.model_name,
            config.calls_per_minute,
            config.cache_max_entries,
            config.cache_ttl_secs,
        );

        Ok(Self {
            config,
            client,
            rate_limiter,
            cache,
        })
    }

    /// 嵌入单个文本（带缓存）
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let cache_key = InnerCache::compute_key(&self.config.model_name, text);
        
        // 先用 read lock 查询缓存（使用 peek，只读不更新 LRU）
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.peek(&cache_key) {
                debug!("Cache hit for text (len={})", text.chars().count());
                return Ok(cached);
            }
        }

        // 缓存未命中，调用 API
        // 注意：embed_batch_raw 内部已处理速率控制，不再阻塞其他请求
        let results = self.embed_batch_raw(&[text.to_string()]).await?;
        let embedding = results
            .into_iter()
            .next()
            .ok_or_else(|| crate::Error::Embedding("No embedding returned".to_string()))?;

        // 写入缓存（需要 write lock）
        {
            let mut cache = self.cache.write().await;
            cache.put(cache_key, embedding.clone());
        }

        Ok(embedding)
    }

    /// 批量嵌入（自动分批 + 缓存命中跳过 API 调用）
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut miss_texts: Vec<String> = Vec::new();
        let mut miss_indices: Vec<usize> = Vec::new();

        // 1. 批量查缓存（使用 read lock + peek，只读不更新 LRU）
        {
            let cache = self.cache.read().await;
            for (idx, text) in texts.iter().enumerate() {
                let key = InnerCache::compute_key(&self.config.model_name, text);
                if let Some(cached) = cache.peek(&key) {
                    results[idx] = Some(cached);
                } else {
                    miss_texts.push(text.clone());
                    miss_indices.push(idx);
                }
            }
        }

        if miss_texts.is_empty() {
            debug!("All {} embeddings served from cache", texts.len());
            return Ok(results.into_iter().map(|opt| opt.unwrap()).collect());
        }

        debug!(
            "{}/{} cache misses, calling API",
            miss_texts.len(),
            texts.len()
        );

        // 2. 顺序分批调用 API
        // 注意：embed_batch_raw 内部已处理速率控制，不再阻塞其他请求
        let mut api_results: Vec<Vec<f32>> = Vec::with_capacity(miss_texts.len());
        for chunk in miss_texts.chunks(self.config.batch_size) {
            let embeddings = self.embed_batch_raw(chunk).await?;
            api_results.extend(embeddings);
        }

        // 3. 写入缓存并填充结果（需要 write lock）
        {
            let mut cache = self.cache.write().await;
            for (api_idx, (text, embedding)) in
                miss_texts.iter().zip(api_results.iter()).enumerate()
            {
                let key = InnerCache::compute_key(&self.config.model_name, text);
                cache.put(key, embedding.clone());
                let result_idx = miss_indices[api_idx];
                results[result_idx] = Some(embedding.clone());
            }
        }

        Ok(results.into_iter().map(|opt| opt.unwrap()).collect())
    }

    /// 分块批量嵌入（向后兼容旧接口，内部复用 embed_batch）
    pub async fn embed_batch_chunked(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.embed_batch(texts).await
    }

    /// 获取 embedding 维度（发一次 API 请求探测）
    pub async fn dimension(&self) -> Result<usize> {
        let embedding = self.embed("test").await?;
        Ok(embedding.len())
    }

    // ── 私有方法 ──────────────────────────────────────────────────────────────

    /// 实际调用 Embedding API 的原始方法（含速率控制，不经过缓存）
    async fn embed_batch_raw(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct EmbeddingRequest {
            input: Vec<String>,
            model: String,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        #[derive(Deserialize)]
        struct EmbeddingResponse {
            data: Vec<EmbeddingData>,
        }

        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.config.model_name.clone(),
        };

        let url = format!("{}/embeddings", self.config.api_base_url);

        // 速率控制：等待令牌（保证单并发 + 最小间隔）
        self.rate_limiter.acquire().await;

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key),
            )
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::Error::Embedding(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // 遇到 429 额外退避
            if status.as_u16() == 429 {
                self.rate_limiter.backoff_on_rate_limit().await;
            }

            return Err(crate::Error::Embedding(format!(
                "Embedding API error ({}): {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| {
                crate::Error::Embedding(format!("Failed to parse response: {}", e))
            })?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }
}
