use cortex_mem_core::{
    CortexFilesystem, ContextLayer,
    layers::LayerManager,
    llm::{LLMClient, LLMClientImpl, LLMConfig},
};
use std::sync::Arc;
use tempfile::TempDir;

/// Test L0 abstract generation with LLM
#[tokio::test]
async fn test_l0_generation_with_llm() {
    // Skip if no LLM API key
    if std::env::var("LLM_API_KEY").is_err() {
        println!("Skipping LLM test - no API key configured");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
    fs.initialize().await.unwrap();

    // Create LLM client
    let llm_config = LLMConfig::default();
    let llm_client = Arc::new(LLMClientImpl::new(llm_config).unwrap());

    // Create LayerManager with LLM
    let layer_manager = LayerManager::with_llm(fs.clone(), llm_client);

    // Test content
    let uri = "cortex://session/test/timeline/2026-02/10/test.md";
    let content = r#"User SkyronJ discussed OAuth 2.0 security best practices during our conversation.

Key points covered:
- Always use HTTPS for token transmission to prevent man-in-the-middle attacks
- Implement PKCE (Proof Key for Code Exchange) for mobile applications
- Rotate refresh tokens regularly to minimize security risks
- Use short-lived access tokens (recommended: 15 minutes)
- Store tokens securely using platform-specific secure storage mechanisms

SkyronJ emphasized that these practices are critical for production systems and should never be compromised for convenience."#;

    // Generate all layers
    layer_manager
        .generate_all_layers(uri, content)
        .await
        .unwrap();

    // Verify L0 abstract
    let abstract_text = layer_manager
        .load(uri, ContextLayer::L0Abstract)
        .await
        .unwrap();

    println!("\n=== L0 Abstract ===");
    println!("{}", abstract_text);
    println!("==================\n");

    // Assertions for L0
    assert!(!abstract_text.is_empty(), "L0 abstract should not be empty");
    assert!(
        abstract_text.len() <= 500,
        "L0 abstract should be concise (got {} chars)",
        abstract_text.len()
    );
    assert!(
        abstract_text.to_lowercase().contains("oauth")
            || abstract_text.to_lowercase().contains("security"),
        "L0 should mention core topics"
    );

    // Verify files exist
    let abstract_uri = "cortex://session/test/timeline/2026-02/10/.abstract.md";
    assert!(
        fs.exists(abstract_uri).await.unwrap(),
        "L0 abstract file should exist"
    );
}

/// Test L1 overview generation with LLM
#[tokio::test]
async fn test_l1_generation_with_llm() {
    // Skip if no LLM API key
    if std::env::var("LLM_API_KEY").is_err() {
        println!("Skipping LLM test - no API key configured");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
    fs.initialize().await.unwrap();

    // Create LLM client
    let llm_config = LLMConfig::default();
    let llm_client = Arc::new(LLMClientImpl::new(llm_config).unwrap());

    // Create LayerManager with LLM
    let layer_manager = LayerManager::with_llm(fs.clone(), llm_client);

    // Test content
    let uri = "cortex://session/test/timeline/2026-02/10/test.md";
    let content = r#"User SkyronJ discussed OAuth 2.0 security best practices during our conversation.

Key points covered:
- Always use HTTPS for token transmission to prevent man-in-the-middle attacks
- Implement PKCE (Proof Key for Code Exchange) for mobile applications
- Rotate refresh tokens regularly to minimize security risks
- Use short-lived access tokens (recommended: 15 minutes)
- Store tokens securely using platform-specific secure storage mechanisms

SkyronJ emphasized that these practices are critical for production systems and should never be compromised for convenience."#;

    // Generate all layers
    layer_manager
        .generate_all_layers(uri, content)
        .await
        .unwrap();

    // Verify L1 overview
    let overview = layer_manager
        .load(uri, ContextLayer::L1Overview)
        .await
        .unwrap();

    println!("\n=== L1 Overview ===");
    println!("{}", overview);
    println!("===================\n");

    // Assertions for L1
    assert!(!overview.is_empty(), "L1 overview should not be empty");
    assert!(
        overview.len() >= 200,
        "L1 overview should be detailed (got {} chars)",
        overview.len()
    );

    // Check for expected structure (markdown sections)
    let has_structure = overview.contains("##") || overview.contains("Summary");
    assert!(
        has_structure,
        "L1 should have structured markdown format"
    );

    // Verify files exist
    let overview_uri = "cortex://session/test/timeline/2026-02/10/.overview.md";
    assert!(
        fs.exists(overview_uri).await.unwrap(),
        "L1 overview file should exist"
    );
}

/// Test lazy generation (on-demand)
#[tokio::test]
async fn test_lazy_generation() {
    // Skip if no LLM API key
    if std::env::var("LLM_API_KEY").is_err() {
        println!("Skipping LLM test - no API key configured");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
    fs.initialize().await.unwrap();

    // Create LLM client
    let llm_config = LLMConfig::default();
    let llm_client = Arc::new(LLMClientImpl::new(llm_config).unwrap());

    // Create LayerManager with LLM
    let layer_manager = LayerManager::with_llm(fs.clone(), llm_client);

    // Test content
    let uri = "cortex://session/test/timeline/2026-02/10/lazy_test.md";
    let content = "This is a test message about lazy L0/L1 generation.";

    // Write only L2 directly
    fs.write(uri, content).await.unwrap();

    // Verify L0/L1 don't exist yet
    let abstract_uri = "cortex://session/test/timeline/2026-02/10/.abstract.md";
    assert!(
        !fs.exists(abstract_uri).await.unwrap(),
        "L0 should not exist before first access"
    );

    // First access to L0 - should trigger generation
    let abstract_text = layer_manager
        .load(uri, ContextLayer::L0Abstract)
        .await
        .unwrap();

    assert!(!abstract_text.is_empty(), "L0 should be generated");

    // Now L0 file should exist
    assert!(
        fs.exists(abstract_uri).await.unwrap(),
        "L0 file should exist after lazy generation"
    );

    // Second access - should read from cache (fast)
    let abstract_text_2 = layer_manager
        .load(uri, ContextLayer::L0Abstract)
        .await
        .unwrap();

    assert_eq!(
        abstract_text, abstract_text_2,
        "Cached L0 should match generated L0"
    );
}

/// Test progressive loading workflow
#[tokio::test]
async fn test_progressive_loading_workflow() {
    // Skip if no LLM API key
    if std::env::var("LLM_API_KEY").is_err() {
        println!("Skipping LLM test - no API key configured");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
    fs.initialize().await.unwrap();

    // Create LLM client
    let llm_config = LLMConfig::default();
    let llm_client = Arc::new(LLMClientImpl::new(llm_config).unwrap());

    // Create LayerManager with LLM
    let layer_manager = LayerManager::with_llm(fs.clone(), llm_client);

    // Create multiple test memories
    let memories = vec![
        (
            "cortex://session/test/timeline/2026-02/10/msg1.md",
            "Discussed OAuth 2.0 security best practices with user.",
        ),
        (
            "cortex://session/test/timeline/2026-02/10/msg2.md",
            "User asked about Rust async programming patterns.",
        ),
        (
            "cortex://session/test/timeline/2026-02/10/msg3.md",
            "Conversation about database indexing strategies.",
        ),
    ];

    // Store all memories with auto L0/L1 generation
    for (uri, content) in &memories {
        layer_manager
            .generate_all_layers(uri, content)
            .await
            .unwrap();
    }

    println!("\n=== Progressive Loading Test ===\n");

    // Step 1: Quick scan with L0
    println!("Step 1: L0 quick scan");
    for (uri, _) in &memories {
        let abstract_text = layer_manager
            .load(uri, ContextLayer::L0Abstract)
            .await
            .unwrap();
        println!("  L0 for {}: {}", uri, abstract_text);
    }

    // Step 2: Detailed evaluation with L1 for first memory
    println!("\nStep 2: L1 detailed evaluation");
    let overview = layer_manager
        .load(memories[0].0, ContextLayer::L1Overview)
        .await
        .unwrap();
    println!("  L1 for {}: {}", memories[0].0, &overview[..200.min(overview.len())]);

    // Step 3: Full read with L2
    println!("\nStep 3: L2 full content");
    let detail = layer_manager
        .load(memories[0].0, ContextLayer::L2Detail)
        .await
        .unwrap();
    println!("  L2 for {}: {}", memories[0].0, detail);

    println!("\n================================\n");
}

/// Test without LLM (fallback to rule-based)
#[tokio::test]
async fn test_fallback_without_llm() {
    let temp_dir = TempDir::new().unwrap();
    let fs = Arc::new(CortexFilesystem::new(temp_dir.path()));
    fs.initialize().await.unwrap();

    // Create LayerManager WITHOUT LLM
    let layer_manager = LayerManager::new(fs.clone());

    let uri = "cortex://session/test/timeline/2026-02/10/fallback.md";
    let content = r#"# OAuth 2.0 Best Practices

## Introduction
OAuth 2.0 is an authorization framework.

## Key Points
- Use HTTPS
- Implement PKCE
- Rotate tokens"#;

    // Generate all layers (should use fallback)
    layer_manager
        .generate_all_layers(uri, content)
        .await
        .unwrap();

    // Verify L0 was generated using fallback
    let abstract_text = layer_manager
        .load(uri, ContextLayer::L0Abstract)
        .await
        .unwrap();

    println!("\n=== Fallback L0 ===");
    println!("{}", abstract_text);
    println!("===================\n");

    assert!(!abstract_text.is_empty(), "Fallback L0 should be generated");

    // Verify L1 was generated using fallback
    let overview = layer_manager
        .load(uri, ContextLayer::L1Overview)
        .await
        .unwrap();

    println!("\n=== Fallback L1 ===");
    println!("{}", overview);
    println!("===================\n");

    assert!(!overview.is_empty(), "Fallback L1 should be generated");
    assert!(
        overview.contains("# Overview"),
        "Fallback L1 should have markdown structure"
    );
}
