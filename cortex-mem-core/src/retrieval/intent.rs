use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query intent extracted from user query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub time_range: Option<TimeRange>,
    pub query_type: QueryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Factual,      // Asking for facts
    Procedural,   // How-to questions
    Conceptual,   // Understanding concepts
    General,      // General queries
}

/// Intent analyzer that extracts meaning from queries
pub struct IntentAnalyzer;

impl IntentAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    /// Analyze query and extract intent
    pub async fn analyze(&self, query: &str) -> Result<Intent> {
        let keywords = self.extract_keywords(query);
        let entities = self.extract_entities(query);
        let time_range = self.extract_time_range(query);
        let query_type = self.classify_query_type(query);
        
        Ok(Intent {
            keywords,
            entities,
            time_range,
            query_type,
        })
    }
    
    /// Extract keywords from query
    fn extract_keywords(&self, query: &str) -> Vec<String> {
        // Simple implementation: filter stopwords and short words
        let stopwords = vec![
            "a", "an", "the", "is", "are", "was", "were", "be", "been",
            "have", "has", "had", "do", "does", "did", "will", "would",
            "can", "could", "should", "may", "might", "must",
            "in", "on", "at", "to", "for", "of", "with", "by", "from",
            "what", "when", "where", "why", "how", "which", "who",
        ];
        
        query
            .to_lowercase()
            .split_whitespace()
            .filter(|word| {
                word.len() > 2 && !stopwords.contains(&word.as_ref())
            })
            .map(|word| {
                // Remove punctuation
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|word| !word.is_empty())
            .collect()
    }
    
    /// Extract named entities (simplified)
    fn extract_entities(&self, query: &str) -> Vec<String> {
        // Simple heuristic: capitalized words that aren't at sentence start
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut entities = Vec::new();
        
        for (i, word) in words.iter().enumerate() {
            if i > 0 && word.chars().next().map_or(false, |c| c.is_uppercase()) {
                entities.push(word.to_string());
            }
        }
        
        entities
    }
    
    /// Extract time range from query (simplified)
    fn extract_time_range(&self, query: &str) -> Option<TimeRange> {
        let query_lower = query.to_lowercase();
        
        // Check for common time phrases
        if query_lower.contains("today") {
            let now = Utc::now();
            let start = now.date_naive().and_hms_opt(0, 0, 0)
                .and_then(|dt| dt.and_local_timezone(Utc).single())?;
            return Some(TimeRange {
                start: Some(start),
                end: Some(now),
            });
        }
        
        if query_lower.contains("this week") {
            let now = Utc::now();
            // Simplified: last 7 days
            let start = now - chrono::Duration::days(7);
            return Some(TimeRange {
                start: Some(start),
                end: Some(now),
            });
        }
        
        if query_lower.contains("this month") {
            let now = Utc::now();
            let start = now - chrono::Duration::days(30);
            return Some(TimeRange {
                start: Some(start),
                end: Some(now),
            });
        }
        
        None
    }
    
    /// Classify query type
    fn classify_query_type(&self, query: &str) -> QueryType {
        let query_lower = query.to_lowercase();
        
        // Check for question words
        if query_lower.starts_with("how") || query_lower.contains("how to") {
            return QueryType::Procedural;
        }
        
        if query_lower.starts_with("what is") || query_lower.starts_with("what are") {
            return QueryType::Conceptual;
        }
        
        if query_lower.starts_with("when") || query_lower.starts_with("where") {
            return QueryType::Factual;
        }
        
        QueryType::General
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_keyword_extraction() {
        let analyzer = IntentAnalyzer::new();
        let query = "How to implement OAuth 2.0 authentication?";
        
        let intent = analyzer.analyze(query).await.unwrap();
        
        assert!(intent.keywords.contains(&"implement".to_string()));
        assert!(intent.keywords.contains(&"oauth".to_string()));
        assert!(intent.keywords.contains(&"authentication".to_string()));
        assert!(!intent.keywords.contains(&"how".to_string())); // stopword
        assert!(!intent.keywords.contains(&"to".to_string()));  // stopword
    }
    
    #[tokio::test]
    async fn test_entity_extraction() {
        let analyzer = IntentAnalyzer::new();
        let query = "What does OpenAI use for GPT models?";
        
        let intent = analyzer.analyze(query).await.unwrap();
        
        assert!(intent.entities.contains(&"OpenAI".to_string()));
        assert!(intent.entities.contains(&"GPT".to_string()));
    }
    
    #[tokio::test]
    async fn test_query_type_classification() {
        let analyzer = IntentAnalyzer::new();
        
        let how_query = analyzer.analyze("How to do something?").await.unwrap();
        assert_eq!(how_query.query_type, QueryType::Procedural);
        
        let what_query = analyzer.analyze("What is Rust?").await.unwrap();
        assert_eq!(what_query.query_type, QueryType::Conceptual);
        
        let when_query = analyzer.analyze("When was it created?").await.unwrap();
        assert_eq!(when_query.query_type, QueryType::Factual);
    }
    
    #[tokio::test]
    async fn test_time_range_extraction() {
        let analyzer = IntentAnalyzer::new();
        
        let today_query = analyzer.analyze("Show me messages from today").await.unwrap();
        assert!(today_query.time_range.is_some());
        
        let week_query = analyzer.analyze("What happened this week?").await.unwrap();
        assert!(week_query.time_range.is_some());
    }
}
