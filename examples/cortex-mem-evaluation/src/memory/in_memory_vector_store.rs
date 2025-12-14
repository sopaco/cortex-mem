//! 内存向量存储实现
//! 
//! 用于评估的简单内存向量存储

use async_trait::async_trait;
use cortex_mem_core::{
    error::Result,
    types::{Filters, Memory, ScoredMemory},
    vector_store::VectorStore,
};
use std::collections::HashMap;
use std::sync::RwLock;

/// 简单的内存向量存储实现
#[derive(Clone)]
pub struct InMemoryVectorStore {
    memories: std::sync::Arc<RwLock<HashMap<String, Memory>>>,
}

impl InMemoryVectorStore {
    /// 创建新的内存向量存储
    pub fn new() -> Self {
        Self {
            memories: std::sync::Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 计算简单的余弦相似度（简化版）
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }
    
    /// 检查记忆是否匹配过滤器
    fn matches_filters(memory: &Memory, filters: &Filters) -> bool {
        // 检查用户ID
        if let Some(user_id) = &filters.user_id {
            if memory.metadata.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }
        
        // 检查记忆类型
        if let Some(memory_type) = &filters.memory_type {
            if &memory.metadata.memory_type != memory_type {
                return false;
            }
        }
        
        // 检查重要性评分
        if let Some(min_importance) = filters.min_importance {
            if memory.metadata.importance_score < min_importance {
                return false;
            }
        }
        
        if let Some(max_importance) = filters.max_importance {
            if memory.metadata.importance_score > max_importance {
                return false;
            }
        }
        
        // 检查创建时间
        if let Some(created_after) = &filters.created_after {
            if &memory.created_at < created_after {
                return false;
            }
        }
        
        if let Some(created_before) = &filters.created_before {
            if &memory.created_at > created_before {
                return false;
            }
        }
        
        // 检查实体
        if let Some(entities) = &filters.entities {
            for entity in entities {
                if !memory.metadata.entities.contains(entity) {
                    return false;
                }
            }
        }
        
        true
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn insert(&self, memory: &Memory) -> Result<()> {
        let mut memories = self.memories.write().unwrap();
        memories.insert(memory.id.clone(), memory.clone());
        Ok(())
    }
    
    async fn search(
        &self,
        query_vector: &[f32],
        filters: &Filters,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>> {
        self.search_with_threshold(query_vector, filters, limit, None).await
    }
    
    async fn search_with_threshold(
        &self,
        query_vector: &[f32],
        filters: &Filters,
        limit: usize,
        score_threshold: Option<f32>,
    ) -> Result<Vec<ScoredMemory>> {
        let memories = self.memories.read().unwrap();
        
        let mut scored_memories: Vec<ScoredMemory> = Vec::new();
        
        for memory in memories.values() {
            // 检查过滤器
            if !Self::matches_filters(memory, filters) {
                continue;
            }
            
            // 计算相似度
            let score = Self::cosine_similarity(query_vector, &memory.embedding);
            
            // 检查阈值
            if let Some(threshold) = score_threshold {
                if score < threshold {
                    continue;
                }
            }
            
            scored_memories.push(ScoredMemory {
                memory: memory.clone(),
                score,
            });
        }
        
        // 按分数排序（降序）
        scored_memories.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // 限制结果数量
        scored_memories.truncate(limit);
        
        Ok(scored_memories)
    }
    
    async fn update(&self, memory: &Memory) -> Result<()> {
        let mut memories = self.memories.write().unwrap();
        memories.insert(memory.id.clone(), memory.clone());
        Ok(())
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let mut memories = self.memories.write().unwrap();
        memories.remove(id);
        Ok(())
    }
    
    async fn get(&self, id: &str) -> Result<Option<Memory>> {
        let memories = self.memories.read().unwrap();
        Ok(memories.get(id).cloned())
    }
    
    async fn list(&self, filters: &Filters, limit: Option<usize>) -> Result<Vec<Memory>> {
        let memories = self.memories.read().unwrap();
        
        let mut filtered_memories: Vec<Memory> = Vec::new();
        
        for memory in memories.values() {
            if Self::matches_filters(memory, filters) {
                filtered_memories.push(memory.clone());
            }
        }
        
        // 按创建时间排序（最新的在前）
        filtered_memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // 限制结果数量
        if let Some(limit) = limit {
            filtered_memories.truncate(limit);
        }
        
        Ok(filtered_memories)
    }
    
    async fn health_check(&self) -> Result<bool> {
        Ok(true) // 内存存储总是健康的
    }
}