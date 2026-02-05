#[cfg(all(test, feature = "vector-search"))]
mod tests {
    use super::*;
    use crate::{
        embedding::EmbeddingClient,
        filesystem::CortexFilesystem,
        vector_store::QdrantVectorStore,
        session::{Message, MessageRole},
    };
    use std::sync::Arc;

    async fn setup_test_environment() -> (Arc<CortexFilesystem>, Arc<EmbeddingClient>, Arc<QdrantVectorStore>) {
        // 创建临时测试目录
        let test_dir = format!("/tmp/cortex-test-{}", uuid::Uuid::new_v4());
        let filesystem = Arc::new(CortexFilesystem::new(&test_dir));
        filesystem.initialize().await.unwrap();

        // 创建embedding客户端（需要真实的API配置或mock）
        let embedding_config = crate::embedding::EmbeddingConfig {
            api_base_url: std::env::var("EMBEDDING_API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("EMBEDDING_API_KEY")
                .unwrap_or_else(|_| "test-key".to_string()),
            model_name: "test-model".to_string(),
            batch_size: 5,
            timeout_secs: 30,
        };
        let embedding_client = Arc::new(EmbeddingClient::new(embedding_config).unwrap());

        // 创建Qdrant向量存储
        let qdrant_config = crate::QdrantConfig {
            url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6334".to_string()),
            collection_name: format!("test-{}", uuid::Uuid::new_v4()),
            embedding_dim: Some(1536),
            timeout_secs: 30,
        };
        let vector_store = Arc::new(QdrantVectorStore::new(&qdrant_config).await.unwrap());

        (filesystem, embedding_client, vector_store)
    }

    #[tokio::test]
    #[ignore] // 需要真实的embedding服务和Qdrant
    async fn test_index_thread() {
        let (filesystem, embedding_client, vector_store) = setup_test_environment().await;

        // 创建测试线程和消息
        let thread_id = "test-thread-1";
        let timeline_uri = format!("cortex://threads/{}/timeline", thread_id);
        
        // 创建测试消息
        let messages = vec![
            Message {
                id: "msg-1".to_string(),
                role: MessageRole::User,
                content: "Hello, this is a test message".to_string(),
                timestamp: chrono::Utc::now(),
                created_at: chrono::Utc::now(),
                metadata: None,
            },
            Message {
                id: "msg-2".to_string(),
                role: MessageRole::Assistant,
                content: "Hi! I'm here to help with testing".to_string(),
                timestamp: chrono::Utc::now(),
                created_at: chrono::Utc::now(),
                metadata: None,
            },
        ];

        // 保存消息到文件系统
        for (idx, msg) in messages.iter().enumerate() {
            let msg_uri = format!("{}/{:06}-{}.md", timeline_uri, idx, msg.id);
            let content = format!(
                "# {} {}\n\n**ID**: `{}`\n**Timestamp**: {}\n\n## Content\n\n{}",
                if matches!(msg.role, MessageRole::User) { "👤 User" } else { "🤖 Assistant" },
                msg.id,
                msg.id,
                msg.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                msg.content
            );
            filesystem.write(&msg_uri, &content).await.unwrap();
        }

        // 创建索引器
        let config = IndexerConfig {
            auto_index: true,
            batch_size: 5,
            async_index: false,
        };
        let indexer = AutoIndexer::new(filesystem, embedding_client, vector_store, config);

        // 执行索引
        let stats = indexer.index_thread(thread_id).await.unwrap();

        // 验证结果
        assert_eq!(stats.total_indexed, 2);
        assert_eq!(stats.total_errors, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_incremental_indexing() {
        let (filesystem, embedding_client, vector_store) = setup_test_environment().await;

        let thread_id = "test-thread-2";
        let timeline_uri = format!("cortex://threads/{}/timeline", thread_id);

        // 第一次索引：2条消息
        let messages = vec![
            Message {
                id: "msg-1".to_string(),
                role: MessageRole::User,
                content: "First message".to_string(),
                timestamp: chrono::Utc::now(),
                created_at: chrono::Utc::now(),
                metadata: None,
            },
            Message {
                id: "msg-2".to_string(),
                role: MessageRole::Assistant,
                content: "Second message".to_string(),
                timestamp: chrono::Utc::now(),
                created_at: chrono::Utc::now(),
                metadata: None,
            },
        ];

        for (idx, msg) in messages.iter().enumerate() {
            let msg_uri = format!("{}/{:06}-{}.md", timeline_uri, idx, msg.id);
            let content = format!(
                "# {} {}\n\n**ID**: `{}`\n**Timestamp**: {}\n\n## Content\n\n{}",
                if matches!(msg.role, MessageRole::User) { "👤 User" } else { "🤖 Assistant" },
                msg.id,
                msg.id,
                msg.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                msg.content
            );
            filesystem.write(&msg_uri, &content).await.unwrap();
        }

        let config = IndexerConfig::default();
        let indexer = AutoIndexer::new(
            filesystem.clone(),
            embedding_client.clone(),
            vector_store.clone(),
            config.clone(),
        );

        let stats1 = indexer.index_thread(thread_id).await.unwrap();
        assert_eq!(stats1.total_indexed, 2);
        assert_eq!(stats1.total_skipped, 0);

        // 添加第3条消息
        let new_msg = Message {
            id: "msg-3".to_string(),
            role: MessageRole::User,
            content: "Third message".to_string(),
            timestamp: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            metadata: None,
        };
        let msg_uri = format!("{}/{:06}-{}.md", timeline_uri, 2, new_msg.id);
        let content = format!(
            "# 👤 User {}\n\n**ID**: `{}`\n**Timestamp**: {}\n\n## Content\n\n{}",
            new_msg.id,
            new_msg.id,
            new_msg.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            new_msg.content
        );
        filesystem.write(&msg_uri, &content).await.unwrap();

        // 第二次索引：应该只索引新消息
        let stats2 = indexer.index_thread(thread_id).await.unwrap();
        assert_eq!(stats2.total_indexed, 1, "Should only index the new message");
        assert_eq!(stats2.total_skipped, 2, "Should skip already indexed messages");
    }

    #[tokio::test]
    #[ignore]
    async fn test_batch_processing() {
        let (filesystem, embedding_client, vector_store) = setup_test_environment().await;

        let thread_id = "test-thread-3";
        let timeline_uri = format!("cortex://threads/{}/timeline", thread_id);

        // 创建15条消息（超过batch_size）
        for i in 0..15 {
            let msg = Message {
                id: format!("msg-{}", i),
                role: if i % 2 == 0 { MessageRole::User } else { MessageRole::Assistant },
                content: format!("Test message number {}", i),
                timestamp: chrono::Utc::now(),
                created_at: chrono::Utc::now(),
                metadata: None,
            };
            let msg_uri = format!("{}/{:06}-{}.md", timeline_uri, i, msg.id);
            let content = format!(
                "# {} {}\n\n**ID**: `{}`\n**Timestamp**: {}\n\n## Content\n\n{}",
                if matches!(msg.role, MessageRole::User) { "👤 User" } else { "🤖 Assistant" },
                msg.id,
                msg.id,
                msg.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                msg.content
            );
            filesystem.write(&msg_uri, &content).await.unwrap();
        }

        let config = IndexerConfig {
            auto_index: true,
            batch_size: 5, // 应该分3批处理
            async_index: false,
        };
        let indexer = AutoIndexer::new(filesystem, embedding_client, vector_store, config);

        let stats = indexer.index_thread(thread_id).await.unwrap();

        assert_eq!(stats.total_indexed, 15);
        assert_eq!(stats.total_errors, 0);
    }
}
