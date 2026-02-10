# LLM-Based L0/L1 Generation and Progressive Loading Guide

**Created**: 2026-02-10  
**Status**: Production Ready  
**Author**: Cortex Memory Team

---

## 📋 Overview

Cortex Memory implements a **three-tier layered loading architecture** (L0/L1/L2) inspired by OpenViking, enabling **80-92% token efficiency improvement** through progressive disclosure of context.

This guide explains how to use LLM-powered automatic generation of abstracts and overviews.

---

## 🎯 Core Concepts

### Three-Layer Architecture

| Layer | Name | Size | Purpose | Access Method |
|-------|------|------|---------|---------------|
| **L0** | Abstract | ~100 tokens | Quick relevance checking | `layer_manager.load(uri, L0Abstract)` |
| **L1** | Overview | ~500-2000 tokens | Decision-making & planning | `layer_manager.load(uri, L1Overview)` |
| **L2** | Detail | Full content | Deep reading | `layer_manager.load(uri, L2Detail)` |

### Progressive Loading Flow

```
User Query → Search
    ↓
L0 Scan (20 memories) → Initial filtering
    ↓
L1 Exploration (3 relevant) → Deep evaluation  
    ↓
L2 Full Read (1 most relevant) → Complete information
```

**Token Comparison**:
- **Without layering**: 20 × 5000 = 100,000 tokens
- **With layering**: 20×100 (L0) + 3×2000 (L1) + 1×5000 (L2) = 13,000 tokens
- **Savings**: 87%

---

## 🚀 Quick Start

### 1. Enable LLM in LayerManager

```rust
use cortex_mem_core::{
    CortexFilesystem, 
    layers::LayerManager,
    llm::{LLMClientImpl, LLMConfig},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Create filesystem
    let fs = Arc::new(CortexFilesystem::new("./cortex-data"));
    fs.initialize().await?;
    
    // 2. Create LLM client
    let llm_config = LLMConfig {
        api_base_url: "https://api.openai.com/v1".to_string(),
        api_key: std::env::var("OPENAI_API_KEY")?,
        model_efficient: "gpt-3.5-turbo".to_string(),
        temperature: 0.1,
        max_tokens: 4096,
    };
    let llm_client = Arc::new(LLMClientImpl::new(llm_config)?);
    
    // 3. Create LayerManager with LLM
    let layer_manager = LayerManager::with_llm(fs.clone(), llm_client);
    
    Ok(())
}
```

### 2. Store Memory with Auto L0/L1 Generation

```rust
use cortex_mem_core::ContextLayer;

// Store new memory - automatically generates L0/L1
let uri = "cortex://session/abc/timeline/2026-02/10/10_00_00_example.md";
let content = r#"User SkyronJ discussed OAuth 2.0 security best practices.
Key points:
- Always use HTTPS for token transmission
- Implement PKCE for mobile apps
- Rotate refresh tokens regularly
- Use short-lived access tokens (15 min recommended)
"#;

// This will:
// 1. Write L2 (full content) to the URI
// 2. Generate L0 abstract using LLM
// 3. Generate L1 overview using LLM
// 4. Cache them as .abstract.md and .overview.md
layer_manager.generate_all_layers(uri, content).await?;
```

**Generated Files**:
```
timeline/2026-02/10/
├── 10_00_00_example.md    # L2 - Full content
├── .abstract.md           # L0 - LLM-generated abstract
└── .overview.md           # L1 - LLM-generated overview
```

### 3. Progressive Loading

```rust
// Scenario: Search and progressively load

// Step 1: Load L0 for quick scanning (100 tokens each)
let memory_uris = vec![
    "cortex://session/abc/timeline/2026-02/10/10_00_00_example.md",
    "cortex://session/abc/timeline/2026-02/09/15_30_00_another.md",
    // ... 18 more
];

for uri in &memory_uris {
    let abstract_text = layer_manager.load(uri, ContextLayer::L0Abstract).await?;
    println!("L0: {}", abstract_text);
    // Decision: Keep top 3 based on relevance
}

// Step 2: Load L1 for detailed evaluation (2000 tokens each)
let relevant_uris = vec![
    memory_uris[0],  // Most relevant
    memory_uris[5],  // Second most relevant
    memory_uris[12], // Third most relevant
];

for uri in &relevant_uris {
    let overview = layer_manager.load(uri, ContextLayer::L1Overview).await?;
    println!("L1: {}", overview);
    // Decision: Select 1 for full read
}

// Step 3: Load L2 for complete information (full tokens)
let selected_uri = relevant_uris[0];
let full_content = layer_manager.load(selected_uri, ContextLayer::L2Detail).await?;
println!("L2: {}", full_content);
```

---

## 🔧 Configuration

### LLM Configuration

```rust
use cortex_mem_core::llm::LLMConfig;

// Option 1: Use environment variables
let config = LLMConfig::default();
// Reads from:
// - LLM_API_BASE_URL (default: https://api.openai.com/v1)
// - LLM_API_KEY
// - LLM_MODEL (default: gpt-3.5-turbo)

// Option 2: Explicit configuration
let config = LLMConfig {
    api_base_url: "https://your-llm-provider.com/v1".to_string(),
    api_key: "your-api-key".to_string(),
    model_efficient: "gpt-4".to_string(),
    temperature: 0.1,
    max_tokens: 4096,
};
```

### Environment Variables

```bash
# For OpenAI
export LLM_API_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="sk-..."
export LLM_MODEL="gpt-3.5-turbo"

# For Azure OpenAI
export LLM_API_BASE_URL="https://your-resource.openai.azure.com"
export LLM_API_KEY="your-azure-key"
export LLM_MODEL="gpt-35-turbo"

# For local LLM (e.g., Ollama)
export LLM_API_BASE_URL="http://localhost:11434/v1"
export LLM_API_KEY="ollama"
export LLM_MODEL="llama2"
```

---

## 📊 L0/L1 Generation Details

### L0 Abstract Generation

**Prompt Strategy** (based on OpenViking):
```
Generate a concise abstract (~100 tokens maximum).

Requirements:
- Single sentence or 2-3 short sentences maximum
- Capture the CORE ESSENCE: who, what, when
- Focus on most important information for quick relevance checking
- Use clear, direct language
- Avoid filler words
```

**Example Input**:
```markdown
User SkyronJ, former leader and friend, INTJ transitioning to ENTJ.
Values efficiency and creativity, focuses on team performance and project impact.
Technical expertise in Rust, career goal is to become a higher-level technical leader.
Aims to serve as coach, evangelist, and architect roles within teams.
```

**Example L0 Output** (~80 tokens):
```
User SkyronJ: former leader turned friend, INTJ→ENTJ, Rust expert, 
aspiring senior technical leader (coach/evangelist/architect roles). 
Values efficiency, creativity, and team impact.
```

### L1 Overview Generation

**Prompt Strategy** (based on OpenViking):
```
Generate a structured overview (~500-2000 tokens).

Structure:
## Summary - 2-3 paragraph overview
## Core Topics - 3-5 main themes (bullets)
## Key Points - 5-10 important takeaways (numbered/bullets)
## Entities - Important people/orgs/technologies
## Context - Background, timeframe, situational info
```

**Example L1 Output** (~600 tokens):
```markdown
## Summary

SkyronJ is a former team leader who has transitioned into a friend 
relationship. He exhibits INTJ personality traits with a conscious 
shift towards ENTJ characteristics. His technical foundation is in 
Rust programming, and he aspires to advance into senior technical 
leadership positions encompassing coaching, evangelism, and 
architectural responsibilities.

His core values center on operational efficiency and creative 
problem-solving, with a strong focus on overall team performance 
metrics and the broader impact of projects.

## Core Topics

- Technical Leadership Development
- Personality Evolution (INTJ → ENTJ)
- Rust Technology Expertise
- Team Performance Optimization

## Key Points

1. Professional goal: Higher-level technical leadership
2. Desired roles: Coach, Evangelist, Architect
3. Core values: Efficiency and creativity
4. Technical strength: Rust programming language
5. Focus areas: Team metrics and project impact

## Entities

- **SkyronJ**: Subject individual
- **Rust**: Primary technical expertise
- **INTJ/ENTJ**: Personality frameworks

## Context

This represents a professional relationship that has evolved 
from a hierarchical structure (leader-subordinate) to a peer 
friendship while maintaining technical collaboration.
```

---

## 🎓 Best Practices

### 1. **When to Use LLM Generation**

✅ **Use LLM-based generation when:**
- Content is complex or unstructured
- High-quality summaries are critical
- You have LLM API access and budget
- Content is in natural language (not code/data)

❌ **Use rule-based fallback when:**
- Content is already well-structured (markdown with clear headers)
- LLM costs are prohibitive
- Offline operation is required
- Content is primarily code or structured data

### 2. **Token Optimization**

```rust
// ✅ GOOD: Progressive loading
let abstract = layer_manager.load(uri, ContextLayer::L0Abstract).await?;
if is_relevant(&abstract) {
    let overview = layer_manager.load(uri, ContextLayer::L1Overview).await?;
    if needs_details(&overview) {
        let detail = layer_manager.load(uri, ContextLayer::L2Detail).await?;
    }
}

// ❌ BAD: Always loading full content
let detail = layer_manager.load(uri, ContextLayer::L2Detail).await?;
```

### 3. **Lazy Generation**

The system generates L0/L1 **on-demand** if not already cached:

```rust
// First access - generates and caches
let abstract1 = layer_manager.load(uri, ContextLayer::L0Abstract).await?;
// ~2-3 seconds (LLM call)

// Second access - reads from cache
let abstract2 = layer_manager.load(uri, ContextLayer::L0Abstract).await?;
// ~10ms (file read)
```

**File structure after first access**:
```
timeline/2026-02/10/
├── 10_00_00_example.md    # L2 - Original
├── .abstract.md           # L0 - Generated and cached
└── .overview.md           # L1 - Generated and cached
```

### 4. **Batch Processing**

```rust
// Process multiple memories efficiently
for uri in memory_uris {
    // Store with auto-generation
    layer_manager.generate_all_layers(&uri, &content).await?;
}

// Later: Fast L0 scanning (cached)
for uri in memory_uris {
    let abstract = layer_manager.load(&uri, ContextLayer::L0Abstract).await?;
    // Instant read from cached .abstract.md
}
```

---

## 🧪 Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_llm_generation_with_mock() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
        fs.initialize().await.unwrap();
        
        // Mock LLM client
        let mock_llm = Arc::new(MockLLMClient::new());
        let layer_manager = LayerManager::with_llm(fs.clone(), mock_llm);
        
        let uri = "cortex://session/test/msg.md";
        let content = "Test content about OAuth 2.0 security.";
        
        // Generate all layers
        layer_manager.generate_all_layers(uri, content).await.unwrap();
        
        // Verify L0
        let abstract_text = layer_manager.load(uri, ContextLayer::L0Abstract).await.unwrap();
        assert!(!abstract_text.is_empty());
        assert!(abstract_text.len() <= 200); // ~100 tokens ≈ 200 chars
        
        // Verify L1
        let overview = layer_manager.load(uri, ContextLayer::L1Overview).await.unwrap();
        assert!(overview.contains("# Overview"));
        
        // Verify L2
        let detail = layer_manager.load(uri, ContextLayer::L2Detail).await.unwrap();
        assert_eq!(detail, content);
    }
}
```

---

## 📈 Performance Characteristics

### Generation Times

| Operation | Cold (LLM call) | Warm (cached) |
|-----------|-----------------|---------------|
| **L0 Generation** | 2-3 seconds | 10ms |
| **L1 Generation** | 3-5 seconds | 15ms |
| **L2 Read** | N/A | 5-10ms |

### Token Usage Per Memory

| Layer | Average Tokens | Cost (GPT-3.5) | Cost (GPT-4) |
|-------|----------------|----------------|--------------|
| **L0** | 80-100 | $0.0001 | $0.0003 |
| **L1** | 500-2000 | $0.002 | $0.006 |
| **L2** | 5000+ | N/A | N/A |

### Storage Overhead

```
Original memory (L2): 5 KB
+ L0 abstract: 0.5 KB
+ L1 overview: 2 KB
= Total: 7.5 KB (50% overhead, but 87% token savings)
```

---

## 🔍 Troubleshooting

### Issue: L0/L1 not generated

**Symptoms**: Only L2 file exists, no `.abstract.md` or `.overview.md`

**Causes & Solutions**:

1. **LLM client not configured**
   ```rust
   // ❌ Missing LLM
   let manager = LayerManager::new(fs);
   
   // ✅ With LLM
   let manager = LayerManager::with_llm(fs, llm_client);
   ```

2. **Not called generate_all_layers**
   ```rust
   // ❌ Direct filesystem write
   fs.write(uri, content).await?;
   
   // ✅ Use LayerManager
   layer_manager.generate_all_layers(uri, content).await?;
   ```

3. **Lazy generation not triggered**
   ```rust
   // Trigger generation by accessing
   let abstract = layer_manager.load(uri, ContextLayer::L0Abstract).await?;
   ```

### Issue: LLM generation fails

**Check**:
1. API key is valid: `echo $LLM_API_KEY`
2. Network connectivity to LLM endpoint
3. Model name is correct for your provider
4. Content is not too large (check max_tokens)

**Debug**:
```rust
// Enable debug logging
env_logger::init();
std::env::set_var("RUST_LOG", "cortex_mem_core=debug");
```

### Issue: Generated abstracts too long

**Solution**: Adjust prompt or use stricter model:
```rust
let config = LLMConfig {
    temperature: 0.0,  // More deterministic
    max_tokens: 150,   // Strict limit
    ..Default::default()
};
```

---

## 📚 Examples

### Example 1: Memory Search with Progressive Loading

```rust
use cortex_mem_core::{
    layers::LayerManager,
    ContextLayer,
    retrieval::RetrievalEngine,
};

async fn search_with_progressive_loading(
    query: &str,
    layer_manager: &LayerManager,
    retrieval_engine: &RetrievalEngine,
) -> Result<Vec<String>> {
    // Step 1: Search at L0 level (fast scan)
    let l0_results = retrieval_engine
        .search(query, ContextLayer::L0Abstract)
        .await?;
    
    println!("L0 scan: {} results", l0_results.len());
    // Token cost: 100 × results.len()
    
    // Step 2: Filter and load L1 for top 5
    let top_5: Vec<_> = l0_results.iter().take(5).collect();
    let mut candidates = Vec::new();
    
    for result in top_5 {
        let overview = layer_manager
            .load(&result.uri, ContextLayer::L1Overview)
            .await?;
        
        if is_highly_relevant(&overview, query) {
            candidates.push(result.uri.clone());
        }
    }
    
    println!("L1 evaluation: {} candidates", candidates.len());
    // Additional token cost: 2000 × 5 = 10,000 tokens
    
    // Step 3: Load full content for final candidate
    if let Some(uri) = candidates.first() {
        let full_content = layer_manager
            .load(uri, ContextLayer::L2Detail)
            .await?;
        
        println!("L2 full read: {}", uri);
        // Additional token cost: ~5000 tokens
    }
    
    Ok(candidates)
}

// Total token usage: 
// - 20 results: 20×100 + 5×2000 + 1×5000 = 17,000 tokens
// - vs loading all L2: 20×5000 = 100,000 tokens
// - Savings: 83%
```

### Example 2: Batch Memory Import

```rust
async fn import_conversation_history(
    messages: Vec<Message>,
    layer_manager: &LayerManager,
) -> Result<()> {
    for (idx, message) in messages.iter().enumerate() {
        let uri = format!(
            "cortex://session/{}/messages/{:04}.md",
            session_id, idx
        );
        
        let content = format!(
            "**Role**: {}\n**Time**: {}\n\n{}",
            message.role,
            message.timestamp,
            message.content
        );
        
        // Auto-generate L0/L1
        layer_manager.generate_all_layers(&uri, &content).await?;
        
        println!("Imported message {} with L0/L1 generation", idx);
    }
    
    Ok(())
}
```

### Example 3: Integration with TARS (Memory Agent)

See `examples/cortex-mem-tars/` for complete working example of LLM-based generation in a TUI chat application.

---

## 🎯 Summary

### Key Takeaways

1. **Enable LLM**: Use `LayerManager::with_llm()` for automatic L0/L1 generation
2. **Progressive Loading**: Load L0 → L1 → L2 as needed to save tokens
3. **Auto-caching**: L0/L1 are generated once and cached for future use
4. **Token Efficiency**: Save 80-92% on token costs compared to always loading full content
5. **Flexible Config**: Works with OpenAI, Azure, local LLMs, or rule-based fallback

### Next Steps

1. **Configure your LLM**: Set environment variables or create `LLMConfig`
2. **Try it out**: Run examples in `examples/cortex-mem-tars/`
3. **Monitor usage**: Track token savings with debug logging
4. **Optimize prompts**: Customize prompts in `llm/prompts.rs` for your use case

---

**Documentation Version**: 1.0  
**Last Updated**: 2026-02-10  
**Maintainer**: Cortex Memory Team  
**Related Docs**: 
- `L0_L1_L2_LAYERED_LOADING_EXPLAINED.md`
- `PROJECT_EVALUATION_REPORT.md`
- `cortex-mem-core/src/layers/README.md`
