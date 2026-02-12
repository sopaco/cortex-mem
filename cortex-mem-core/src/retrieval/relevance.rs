use super::intent::Intent;

/// Relevance calculator for scoring matches
pub struct RelevanceCalculator;

impl RelevanceCalculator {
    pub fn new() -> Self {
        Self
    }
    
    /// Calculate relevance score between text and intent
    /// 
    /// Returns a score between 0.0 and 1.0
    pub fn calculate(&self, text: &str, intent: &Intent) -> f32 {
        let mut score = 0.0;
        
        // 1. Keyword matching (0-0.6)
        let keyword_score = self.keyword_matching(text, &intent.keywords);
        score += keyword_score * 0.6;
        
        // 2. Entity matching (0-0.3)
        let entity_score = self.entity_matching(text, &intent.entities);
        score += entity_score * 0.3;
        
        // 3. Query type relevance (0-0.1)
        let type_score = self.query_type_matching(text, &intent.query_type);
        score += type_score * 0.1;
        
        score.min(1.0)
    }
    
    /// Calculate keyword matching score
    fn keyword_matching(&self, text: &str, keywords: &[String]) -> f32 {
        if keywords.is_empty() {
            return 0.5; // Default score if no keywords
        }
        
        let text_lower = text.to_lowercase();
        let mut matched = 0;
        let mut weighted_score = 0.0;
        
        for keyword in keywords {
            let keyword_lower = keyword.to_lowercase();
            
            // Count occurrences
            let count = text_lower.matches(&keyword_lower).count();
            
            if count > 0 {
                matched += 1;
                // TF (term frequency) component
                weighted_score += (1.0 + (count as f32).ln()) / (1.0 + text.len() as f32 / 1000.0);
            }
        }
        
        // Combine coverage and TF
        let coverage = matched as f32 / keywords.len() as f32;
        let tf_normalized = (weighted_score / keywords.len() as f32).min(1.0);
        
        (coverage * 0.7 + tf_normalized * 0.3).min(1.0)
    }
    
    /// Calculate entity matching score
    fn entity_matching(&self, text: &str, entities: &[String]) -> f32 {
        if entities.is_empty() {
            return 1.0; // No entities required, full score
        }
        
        let matched = entities
            .iter()
            .filter(|entity| text.contains(entity.as_str()))
            .count();
        
        matched as f32 / entities.len() as f32
    }
    
    /// Query type matching
    fn query_type_matching(&self, _text: &str, _query_type: &super::intent::QueryType) -> f32 {
        // Placeholder: could analyze text structure
        0.5
    }
    
    /// Calculate TF-IDF score (simplified)
    pub fn calculate_tfidf(&self, text: &str, keyword: &str, doc_count: usize, docs_with_keyword: usize) -> f32 {
        // TF (Term Frequency)
        let text_lower = text.to_lowercase();
        let keyword_lower = keyword.to_lowercase();
        let tf = text_lower.matches(&keyword_lower).count() as f32;
        
        // IDF (Inverse Document Frequency)
        let idf = if docs_with_keyword > 0 {
            ((doc_count as f32) / (docs_with_keyword as f32)).ln()
        } else {
            0.0
        };
        
        tf * idf
    }
    
    /// Calculate BM25 score (commonly used in search engines)
    pub fn calculate_bm25(
        &self,
        text: &str,
        keywords: &[String],
        avg_doc_length: f32,
        k1: f32,
        b: f32,
    ) -> f32 {
        let doc_length = text.split_whitespace().count() as f32;
        let mut score = 0.0;
        
        for keyword in keywords {
            let tf = text.to_lowercase()
                .matches(&keyword.to_lowercase())
                .count() as f32;
            
            // BM25 formula
            let numerator = tf * (k1 + 1.0);
            let denominator = tf + k1 * (1.0 - b + b * (doc_length / avg_doc_length));
            
            score += numerator / denominator;
        }
        
        score
    }
}

// 核心功能测试已迁移至 cortex-mem-tools/tests/core_functionality_tests.rs
