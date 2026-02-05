use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Language information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub language_code: String,
    pub language_name: String,
    pub confidence: f32,
}

/// Extract and remove code blocks from text (similar to mem0's remove_code_blocks)
pub fn remove_code_blocks(content: &str) -> String {
    use regex::Regex;
    let pattern = Regex::new(r"^```[a-zA-Z0-9]*\n([\s\S]*?)\n```$").unwrap();
    
    if let Some(match_result) = pattern.find(content.trim()) {
        let inner_content = &content[match_result.start() + 3..match_result.end() - 3];
        let cleaned = inner_content.trim();
        
        // Remove thinking blocks like <think>...</think> or【thinking】...【/thinking】
        let cleaned = regex::Regex::new(r"(<think>.*?</think>|【thinking】.*?【/thinking】)")
            .unwrap_or_else(|_| {
                // If the primary pattern fails, create a simple one
                regex::Regex::new(r"【thinking】.*?【/thinking】").unwrap()
            })
            .replace_all(cleaned, "")
            .replace("\n\n\n", "\n\n")
            .trim()
            .to_string();
            
        cleaned
    } else {
        // If no code blocks found, remove thinking blocks from the whole text
        let cleaned = regex::Regex::new(r"(<think>.*?</think>|【thinking】.*?【/thinking】)")
            .unwrap_or_else(|_| {
                regex::Regex::new(r"【thinking】.*?【/thinking】").unwrap()
            })
            .replace_all(content, "")
            .replace("\n\n\n", "\n\n")
            .trim()
            .to_string();
            
        cleaned
    }
}

/// Extract JSON content from text, removing enclosing triple backticks and optional 'json' tag
pub fn extract_json(text: &str) -> String {
    let text = text.trim();
    
    // First try to find code blocks
    if let Some(pattern) = regex::Regex::new(r"```(?:json)?\s*(.*?)\s*```").unwrap().find(text) {
        let json_str = &text[pattern.start() + 3 + 3..pattern.end() - 3]; // Skip ``` and optional 'json\n'
        json_str.trim().to_string()
    } else {
        // Assume it's raw JSON
        text.to_string()
    }
}

/// Detect language of the input text
pub fn detect_language(text: &str) -> LanguageInfo {
    // Simple language detection based on common patterns
    // For production use, consider using a proper NLP library like whatlang or cld3
    
    let clean_text = text.trim().to_lowercase();
    
    // Chinese detection
    if clean_text.chars().any(|c| (c as u32) > 0x4E00 && (c as u32) < 0x9FFF) {
        return LanguageInfo {
            language_code: "zh".to_string(),
            language_name: "Chinese".to_string(),
            confidence: 0.9,
        };
    }
    
    // Japanese detection (Hiragana, Katakana, Kanji)
    if clean_text.chars().any(|c| 
        (c as u32 >= 0x3040 && c as u32 <= 0x30FF) || // Hiragana, Katakana
        ((c as u32) >= 0x4E00 && (c as u32) < 0x9FFF)     // Kanji
    ) {
        return LanguageInfo {
            language_code: "ja".to_string(),
            language_name: "Japanese".to_string(),
            confidence: 0.8,
        };
    }
    
    // Korean detection
    if clean_text.chars().any(|c| c as u32 >= 0xAC00 && c as u32 <= 0xD7AF) {
        return LanguageInfo {
            language_code: "ko".to_string(),
            language_name: "Korean".to_string(),
            confidence: 0.8,
        };
    }
    
    // Russian/Cyrillic detection
    if clean_text.chars().any(|c| c as u32 >= 0x0400 && c as u32 <= 0x04FF) {
        return LanguageInfo {
            language_code: "ru".to_string(),
            language_name: "Russian".to_string(),
            confidence: 0.9,
        };
    }
    
    // Arabic detection
    if clean_text.chars().any(|c| c as u32 >= 0x0600 && c as u32 <= 0x06FF) {
        return LanguageInfo {
            language_code: "ar".to_string(),
            language_name: "Arabic".to_string(),
            confidence: 0.9,
        };
    }
    
    // Default to English
    LanguageInfo {
        language_code: "en".to_string(),
        language_name: "English".to_string(),
        confidence: 0.7,
    }
}

/// Parse messages from conversation (similar to mem0's parse_messages)
pub fn parse_messages(messages: &[crate::session::Message]) -> String {
    use crate::session::MessageRole;
    
    let mut response = String::new();
    
    for msg in messages {
        match msg.role {
            MessageRole::System => response.push_str(&format!("system: {}\n", msg.content)),
            MessageRole::User => response.push_str(&format!("user: {}\n", msg.content)),
            MessageRole::Assistant => response.push_str(&format!("assistant: {}\n", msg.content)),
        }
    }
    
    response
}

/// Sanitize text for Cypher queries (similar to mem0's sanitize_relationship_for_cypher)
pub fn sanitize_for_cypher(text: &str) -> String {
    let char_map = HashMap::from([
        ("...", "_ellipsis_"),
        ("…", "_ellipsis_"),
        ("。", "_period_"),
        ("，", "_comma_"),
        ("；", "_semicolon_"),
        ("：", "_colon_"),
        ("！", "_exclamation_"),
        ("？", "_question_"),
        ("（", "_lparen_"),
        ("）", "_rparen_"),
        ("【", "_lbracket_"),
        ("】", "_rbracket_"),
        ("《", "_langle_"),
        ("》", "_rangle_"),
        ("'", "_apostrophe_"),
        ("\"", "_quote_"),
        ("\\", "_backslash_"),
        ("/", "_slash_"),
        ("|", "_pipe_"),
        ("&", "_ampersand_"),
        ("=", "_equals_"),
        ("+", "_plus_"),
        ("*", "_asterisk_"),
        ("^", "_caret_"),
        ("%", "_percent_"),
        ("$", "_dollar_"),
        ("#", "_hash_"),
        ("@", "_at_"),
        ("!", "_bang_"),
        ("?", "_question_"),
        ("(", "_lparen_"),
        (")", "_rparen_"),
        ("[", "_lbracket_"),
        ("]", "_rbracket_"),
        ("{", "_lbrace_"),
        ("}", "_rbrace_"),
        ("<", "_langle_"),
        (">", "_rangle_"),
    ]);
    
    let mut sanitized = text.to_string();
    
    for (old, new) in &char_map {
        sanitized = sanitized.replace(old, new);
    }
    
    // Clean up multiple underscores
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    
    sanitized.trim_start_matches('_').trim_end_matches('_').to_string()
}

/// Filter message history by roles (for user-only or assistant-only extraction)
pub fn filter_messages_by_role(messages: &[crate::session::Message], role: crate::session::MessageRole) -> Vec<crate::session::Message> {
    messages
        .iter()
        .filter(|msg| msg.role == role)
        .cloned()
        .collect()
}

/// Filter messages by multiple roles
pub fn filter_messages_by_roles(messages: &[crate::session::Message], roles: &[crate::session::MessageRole]) -> Vec<crate::session::Message> {
    messages
        .iter()
        .filter(|msg| roles.contains(&msg.role))
        .cloned()
        .collect()
}

