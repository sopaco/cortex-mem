#[cfg(all(test, feature = "vector-search"))]
mod tests {
    use crate::{
        embedding::EmbeddingClient,
        vector_store::{QdrantVectorStore, VectorStore},
        types::{Memory, MemoryMetadata, MemoryType, Filters, ScoredMemory},
    };
    use std::sync::Arc;

    async fn setup_test_vector_store() -> Arc<QdrantVectorStore> {
        let config = crate::QdrantConfig {
            url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6334".to_string()),
            collection_name: format!("test-vector-search-{}", uuid::Uuid::new_v4()),
            embedding_dim: Some(1536),
            timeout_secs: 30,
        };
        Arc::new(QdrantVectorStore::new(&config).await.unwrap())
    }

    async fn setup_test_embedding_client() -> Arc<EmbeddingClient> {
        let config = crate::embedding::EmbeddingConfig {
            api_base_url: std::env::var("EMBEDDING_API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("EMBEDDING_API_KEY")
                .unwrap_or_else(|_| "test-key".to_string()),
            model_name: "test-model".to_string(),
            batch_size: 10,
            timeout_secs: 30,
        };
        Arc::new(EmbeddingClient::new(config).unwrap())
    }

    #[tokio::test]
    #[ignore] // 需要真实的服务
    async fn test_embedding_generation() {
        let client = setup_test_embedding_client().await;

        let text = "This is a test sentence for embedding generation";
        let embedding = client.embed(text).await.unwrap();

        assert!(!embedding.is_empty());
        assert_eq!(embedding.len(), 1536); // 假设使用的模型维度是1536
    }

    #[tokio::test]
    #[ignore]
    async fn test_embedding_batch() {
        let client = setup_test_embedding_client().await;

        let texts = vec![
            "First test sentence".to_string(),
            "Second test sentence".to_string(),
            "Third test sentence".to_string(),
        ];

        let embeddings = client.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in embeddings {
            assert_eq!(emb.len(), 1536);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_vector_search_insert_and_search() {
        let vector_store = setup_test_vector_store().await;
        let embedding_client = setup_test_embedding_client().await;

        // 插入测试数据
        let contents = vec![
            "Rust is a systems programming language",
            "Python is great for data science",
            "JavaScript is used for web development",
        ];

        for (idx, content) in contents.iter().enumerate() {
            let embedding = embedding_client.embed(content).await.unwrap();
            let memory = Memory {
                id: format!("test-mem-{}", idx),
                content: content.to_string(),
                embedding,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                metadata: MemoryMetadata {
                    user_id: Some("test-user".to_string()),
                    agent_id: None,
                    run_id: None,
                    actor_id: None,
                    role: None,
                    memory_type: MemoryType::Conversational,
                    hash: format!("hash-{}", idx),
                    importance_score: 0.5,
                    entities: vec![],
                    topics: vec![],
                    custom: std::collections::HashMap::new(),
                },
            };
            vector_store.insert(&memory).await.unwrap();
        }

        // 等待索引更新
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // 测试搜索
        let query = "programming languages";
        let query_embedding = embedding_client.embed(query).await.unwrap();
        let filters = Filters::default();
        
        let results = vector_store
            .search(&query_embedding, &filters, 3)
            .await
            .unwrap();

        assert!(!results.is_empty());
        // 验证返回的是相关的结果
        assert!(results[0].memory.content.contains("Rust") 
                || results[0].memory.content.contains("Python")
                || results[0].memory.content.contains("JavaScript"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_vector_search_with_threshold() {
        let vector_store = setup_test_vector_store().await;
        let embedding_client = setup_test_embedding_client().await;

        // 插入测试数据
        let content = "Machine learning is a subset of artificial intelligence";
        let embedding = embedding_client.embed(content).await.unwrap();
        let memory = Memory {
            id: "ml-memory".to_string(),
            content: content.to_string(),
            embedding,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: MemoryMetadata::default(),
        };
        vector_store.insert(&memory).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // 高相似度查询
        let query_high = "What is machine learning?";
        let query_emb_high = embedding_client.embed(query_high).await.unwrap();
        
        let results_high = vector_store
            .search_with_threshold(&query_emb_high, &Filters::default(), 10, Some(0.7))
            .await
            .unwrap();

        // 低相似度查询
        let query_low = "How to cook pasta?";
        let query_emb_low = embedding_client.embed(query_low).await.unwrap();
        
        let results_low = vector_store
            .search_with_threshold(&query_emb_low, &Filters::default(), 10, Some(0.7))
            .await
            .unwrap();

        // 高相似度查询应该返回结果
        assert!(!results_high.is_empty());
        // 低相似度查询可能不返回结果（取决于阈值）
        // results_low可能为空
    }

    #[tokio::test]
    #[ignore]
    async fn test_vector_search_with_filters() {
        let vector_store = setup_test_vector_store().await;
        let embedding_client = setup_test_embedding_client().await;

        // 插入不同用户的数据
        let users = vec!["user-1", "user-2"];
        for user in users {
            for i in 0..3 {
                let content = format!("Message {} from {}", i, user);
                let embedding = embedding_client.embed(&content).await.unwrap();
                let memory = Memory {
                    id: format!("{}-msg-{}", user, i),
                    content,
                    embedding,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    metadata: MemoryMetadata {
                        user_id: Some(user.to_string()),
                        ..Default::default()
                    },
                };
                vector_store.insert(&memory).await.unwrap();
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // 搜索特定用户的消息
        let query = "message";
        let query_embedding = embedding_client.embed(query).await.unwrap();
        let filters = Filters {
            user_id: Some("user-1".to_string()),
            ..Default::default()
        };

        let results = vector_store
            .search(&query_embedding, &filters, 10)
            .await
            .unwrap();

        // 所有结果应该属于user-1
        for result in results {
            assert_eq!(result.memory.metadata.user_id.as_ref().unwrap(), "user-1");
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_scroll_ids() {
        let vector_store = setup_test_vector_store().await;
        let embedding_client = setup_test_embedding_client().await;

        // 插入测试数据
        for i in 0..10 {
            let content = format!("Test message {}", i);
            let embedding = embedding_client.embed(&content).await.unwrap();
            let memory = Memory {
                id: format!("scroll-test-{}", i),
                content,
                embedding,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                metadata: MemoryMetadata::default(),
            };
            vector_store.insert(&memory).await.unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // 测试scroll_ids
        let filters = Filters::default();
        let ids = vector_store.scroll_ids(&filters, 100).await.unwrap();

        assert_eq!(ids.len(), 10);
        for i in 0..10 {
            assert!(ids.contains(&format!("scroll-test-{}", i)));
        }
    }
}
