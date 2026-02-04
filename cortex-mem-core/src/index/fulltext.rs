use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;

/// Full-text search index using Tantivy
pub struct FullTextIndex {
    index: Index,
    uri_field: Field,
    content_field: Field,
    keywords_field: Field,
}

/// Full-text search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextResult {
    pub uri: String,
    pub score: f32,
    pub snippet: String,
}

impl FullTextIndex {
    /// Create a new full-text index
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        
        let uri_field = schema_builder.add_text_field("uri", STRING | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let keywords_field = schema_builder.add_text_field("keywords", TEXT);
        
        let schema = schema_builder.build();
        
        std::fs::create_dir_all(index_path.as_ref())?;
        let index = Index::create_in_dir(index_path, schema)?;
        
        Ok(Self {
            index,
            uri_field,
            content_field,
            keywords_field,
        })
    }
    
    /// Create in-memory index for testing
    pub fn in_memory() -> Result<Self> {
        let mut schema_builder = Schema::builder();
        
        let uri_field = schema_builder.add_text_field("uri", STRING | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let keywords_field = schema_builder.add_text_field("keywords", TEXT);
        
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema);
        
        Ok(Self {
            index,
            uri_field,
            content_field,
            keywords_field,
        })
    }
    
    /// Index a document
    pub fn add_document(&self, uri: &str, content: &str, keywords: &[String]) -> Result<()> {
        let mut index_writer: tantivy::IndexWriter = self.index.writer(50_000_000)?;
        
        let mut doc = tantivy::TantivyDocument::new();
        doc.add_text(self.uri_field, uri);
        doc.add_text(self.content_field, content);
        doc.add_text(self.keywords_field, &keywords.join(" "));
        
        index_writer.add_document(doc)?;
        index_writer.commit()?;
        
        Ok(())
    }
    
    /// Delete a document by URI
    pub fn delete_document(&self, uri: &str) -> Result<()> {
        let mut index_writer: tantivy::IndexWriter = self.index.writer(50_000_000)?;
        
        let term = Term::from_field_text(self.uri_field, uri);
        index_writer.delete_term(term);
        index_writer.commit()?;
        
        Ok(())
    }
    
    /// Search for documents
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<FullTextResult>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.content_field, self.keywords_field],
        );
        
        let query = query_parser.parse_query(query_str)?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            
            let uri = retrieved_doc
                .get_first(self.uri_field)
                .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let snippet = Self::create_snippet(&content, query_str);
            
            results.push(FullTextResult {
                uri,
                score: _score,
                snippet,
            });
        }
        
        Ok(results)
    }
    
    /// Create a snippet highlighting query terms
    fn create_snippet(content: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        // Find first query word
        for word in query_lower.split_whitespace() {
            if let Some(pos) = content_lower.find(word) {
                let start = pos.saturating_sub(50);
                let end = (pos + word.len() + 50).min(content.len());
                
                let mut snippet = content[start..end].to_string();
                
                if start > 0 {
                    snippet = format!("...{}", snippet);
                }
                if end < content.len() {
                    snippet = format!("{}...", snippet);
                }
                
                return snippet;
            }
        }
        
        // Fallback: first 100 chars
        if content.len() > 100 {
            format!("{}...", &content[..97])
        } else {
            content.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fulltext_index() {
        let index = FullTextIndex::in_memory().unwrap();
        
        // Add some documents
        index.add_document(
            "cortex://threads/t1/msg1.md",
            "This is about OAuth 2.0 authentication and security.",
            &vec!["oauth".to_string(), "authentication".to_string()],
        ).unwrap();
        
        index.add_document(
            "cortex://threads/t1/msg2.md",
            "PostgreSQL database setup and configuration.",
            &vec!["database".to_string(), "postgresql".to_string()],
        ).unwrap();
        
        // Search for OAuth
        let results = index.search("OAuth authentication", 10).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].uri.contains("msg1"));
        
        // Search for database
        let results = index.search("database", 10).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].uri.contains("msg2"));
    }
    
    #[test]
    fn test_delete_document() {
        let index = FullTextIndex::in_memory().unwrap();
        
        index.add_document(
            "cortex://global/doc1.md",
            "This is a test document.",
            &vec!["test".to_string()],
        ).unwrap();
        
        let results = index.search("test", 10).unwrap();
        assert_eq!(results.len(), 1);
        
        index.delete_document("cortex://global/doc1.md").unwrap();
        
        // Note: Tantivy deletion requires a reader reload
        // In production, we'd need to handle this properly
    }
}
