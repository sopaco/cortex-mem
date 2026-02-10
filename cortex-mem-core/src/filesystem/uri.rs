use crate::{Dimension, Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Cortex URI representing a memory resource
/// 
/// Format: `cortex://{dimension}/{id}/{category}/{subcategory}/{resource}?{params}`
/// 
/// Examples:
/// - `cortex://session/thread_abc123/timeline/2026-02/03/10_00.md`
/// - `cortex://agents/bot_001/memories/facts/oauth_knowledge.md`
/// - `cortex://users/user_001/preferences/communication_style.md`
#[derive(Debug, Clone, PartialEq)]
pub struct CortexUri {
    pub dimension: Dimension,
    pub id: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub resource: Option<String>,
    pub params: HashMap<String, String>,
}

impl CortexUri {
    /// Create a new CortexUri
    pub fn new(dimension: Dimension, id: String) -> Self {
        Self {
            dimension,
            id,
            category: String::new(),
            subcategory: None,
            resource: None,
            params: HashMap::new(),
        }
    }
    
    /// Convert URI to file system path
    pub fn to_file_path(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();
        
        // Add dimension
        path.push(self.dimension.as_str());
        
        // Add id (only if not empty)
        if !self.id.is_empty() {
            path.push(&self.id);
        }
        
        // Add category
        if !self.category.is_empty() {
            path.push(&self.category);
        }
        
        // Add subcategory
        if let Some(ref sub) = self.subcategory {
            path.push(sub);
        }
        
        // Add resource
        if let Some(ref res) = self.resource {
            path.push(res);
        }
        
        path
    }
    
    /// Get directory URI (without resource)
    pub fn directory_uri(&self) -> String {
        let mut uri = format!(
            "cortex://{}/{}/{}",
            self.dimension.as_str(),
            self.id,
            self.category
        );
        
        if let Some(ref sub) = self.subcategory {
            uri.push('/');
            uri.push_str(sub);
        }
        
        uri
    }
    
    /// Convert to full URI string
    pub fn to_string(&self) -> String {
        let mut uri = format!(
            "cortex://{}/{}",
            self.dimension.as_str(),
            self.id
        );
        
        if !self.category.is_empty() {
            uri.push('/');
            uri.push_str(&self.category);
        }
        
        if let Some(ref sub) = self.subcategory {
            uri.push('/');
            uri.push_str(sub);
        }
        
        if let Some(ref res) = self.resource {
            uri.push('/');
            uri.push_str(res);
        }
        
        if !self.params.is_empty() {
            uri.push('?');
            let params: Vec<String> = self.params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            uri.push_str(&params.join("&"));
        }
        
        uri
    }
}

/// URI Parser for cortex:// protocol
pub struct UriParser;

impl UriParser {
    /// Parse a cortex:// URI string
    /// 
    /// # Examples
    /// 
    /// ```
    /// use cortex_mem_core::filesystem::UriParser;
    /// 
    /// let uri = UriParser::parse("cortex://session/abc123/timeline").unwrap();
    /// assert_eq!(uri.dimension, cortex_mem_core::Dimension::Session);
    /// assert_eq!(uri.id, "abc123");
    /// assert_eq!(uri.category, "timeline");
    /// ```
    pub fn parse(uri: &str) -> Result<CortexUri> {
        // 1. Validate scheme
        if !uri.starts_with("cortex://") {
            return Err(Error::InvalidScheme);
        }
        
        // 2. Split path and query
        let uri_without_scheme = &uri[9..]; // Skip "cortex://"
        let (path_part, query_part) = uri_without_scheme
            .split_once('?')
            .map(|(p, q)| (p, Some(q)))
            .unwrap_or((uri_without_scheme, None));
        
        // 3. Parse path
        let parts: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
        
        // Allow dimension-only URIs (e.g., "cortex://session")
        if parts.is_empty() {
            return Err(Error::InvalidPath);
        }
        
        let dimension = Dimension::from_str(parts[0])
            .ok_or_else(|| Error::InvalidDimension(parts[0].to_string()))?;
        
        // If only dimension is provided, use empty string for id
        let id = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
        let category = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
        
        // Determine subcategory and resource
        // If parts.len() == 3: just category, no subcategory/resource
        // If parts.len() == 4: could be category/subcategory OR category/resource
        // If parts.len() > 4: category/subcategory/resource...
        let (subcategory, resource) = if parts.len() <= 3 {
            (None, None)
        } else if parts.len() == 4 {
            // If last part has extension, it's a file (resource)
            if parts[3].contains('.') {
                (None, Some(parts[3].to_string()))
            } else {
                // Otherwise it's a subcategory
                (Some(parts[3].to_string()), None)
            }
        } else {
            // parts.len() > 4: category/subcategory/resource/...
            (
                Some(parts[3].to_string()),
                Some(parts[4..].join("/"))
            )
        };
        
        // 4. Parse query params
        let params = Self::parse_query_params(query_part);
        
        Ok(CortexUri {
            dimension,
            id,
            category,
            subcategory,
            resource,
            params,
        })
    }
    
    fn parse_query_params(query: Option<&str>) -> HashMap<String, String> {
        query
            .map(|q| {
                q.split('&')
                    .filter_map(|pair| {
                        let mut parts = pair.split('=');
                        Some((
                            parts.next()?.to_string(),
                            parts.next()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_uri() {
        let uri = UriParser::parse("cortex://session/abc123/timeline").unwrap();
        assert_eq!(uri.dimension, Dimension::Session);
        assert_eq!(uri.id, "abc123");
        assert_eq!(uri.category, "timeline");
        assert!(uri.subcategory.is_none());
        assert!(uri.resource.is_none());
    }
    
    #[test]
    fn test_parse_full_uri() {
        let uri = UriParser::parse("cortex://session/abc123/timeline/2026-02/03/10_00.md").unwrap();
        assert_eq!(uri.dimension, Dimension::Session);
        assert_eq!(uri.id, "abc123");
        assert_eq!(uri.category, "timeline");
        assert_eq!(uri.subcategory, Some("2026-02".to_string()));
        assert_eq!(uri.resource, Some("03/10_00.md".to_string()));
    }
    
    #[test]
    fn test_parse_with_params() {
        let uri = UriParser::parse("cortex://session/abc123/timeline?layer=L1").unwrap();
        assert_eq!(uri.params.get("layer"), Some(&"L1".to_string()));
    }
    
    #[test]
    fn test_invalid_scheme() {
        let result = UriParser::parse("http://threads/abc123");
        assert!(matches!(result, Err(Error::InvalidScheme)));
    }
    
    #[test]
    fn test_to_file_path() {
        let uri = CortexUri {
            dimension: Dimension::Session,
            id: "abc123".to_string(),
            category: "timeline".to_string(),
            subcategory: Some("2026-02".to_string()),
            resource: Some("03.md".to_string()),
            params: HashMap::new(),
        };
        
        let path = uri.to_file_path(Path::new("/data"));
        assert_eq!(path, PathBuf::from("/data/session/abc123/timeline/2026-02/03.md"));
    }
}
