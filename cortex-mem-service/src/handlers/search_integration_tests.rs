#[cfg(all(test, feature = "vector-search"))]
mod integration_tests {
    use super::*;
    use crate::{state::AppState, models::*};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use serde_json::json;
    use std::sync::Arc;

    /// 创建测试用的AppState
    async fn setup_test_state() -> Arc<AppState> {
        let test_dir = format!("/tmp/cortex-test-service-{}", uuid::Uuid::new_v4());
        Arc::new(AppState::new(&test_dir).await.unwrap())
    }

    /// 辅助函数：发送POST请求并解析JSON响应
    async fn post_json<T: serde::de::DeserializeOwned>(
        app: &axum::Router,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, Option<T>) {
        let request = Request::builder()
            .uri(path)
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let parsed = if !body_bytes.is_empty() {
            serde_json::from_slice(&body_bytes).ok()
        } else {
            None
        };

        (status, parsed)
    }

    #[tokio::test]
    #[ignore] // 需要真实的embedding服务和Qdrant
    async fn test_search_endpoint_vector_mode() {
        let state = setup_test_state().await;
        let app = crate::routes::api_routes().with_state(state);

        // 测试向量搜索
        let request_body = json!({
            "query": "test query",
            "mode": "Vector",
            "limit": 10,
            "min_score": 0.5
        });

        let (status, response): (_, Option<ApiResponse<Vec<SearchResultResponse>>>) = 
            post_json(&app, "/search", request_body).await;

        assert!(status.is_success() || status == StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[ignore]
    async fn test_search_endpoint_filesystem_mode() {
        let state = setup_test_state().await;
        let app = crate::routes::api_routes().with_state(state.clone());

        // 先创建一些测试数据
        let timeline_uri = "cortex://threads/test-thread-1/timeline";
        state.filesystem.write(
            &format!("{}/000001-msg1.md", timeline_uri),
            "# Test Message\n\nThis is a test message content"
        ).await.unwrap();

        // 测试文件系统搜索
        let request_body = json!({
            "query": "test",
            "mode": "Filesystem",
            "thread": "test-thread-1",
            "limit": 10
        });

        let (status, response): (_, Option<ApiResponse<Vec<SearchResultResponse>>>) = 
            post_json(&app, "/search", request_body).await;

        assert!(status.is_success());
        if let Some(resp) = response {
            assert!(resp.success);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_indexing_endpoint() {
        let state = setup_test_state().await;
        let app = crate::routes::api_routes().with_state(state.clone());

        // 创建测试线程数据
        let thread_id = "test-thread-index";
        let timeline_uri = format!("cortex://threads/{}/timeline", thread_id);
        
        for i in 0..5 {
            state.filesystem.write(
                &format!("{}/{:06}-msg{}.md", timeline_uri, i, i),
                &format!("# Message {}\n\n**ID**: `msg-{}`\n\n## Content\n\nTest content {}", i, i, i)
            ).await.unwrap();
        }

        // 触发索引
        let (status, _): (_, Option<ApiResponse<serde_json::Value>>) = 
            post_json(&app, &format!("/automation/index/{}", thread_id), json!({})).await;

        // 如果没有配置embedding服务，会返回错误
        assert!(status.is_success() || status == StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_concurrent_search_requests() {
        let state = setup_test_state().await;
        let app = Arc::new(crate::routes::api_routes().with_state(state.clone()));

        // 创建测试数据
        for i in 0..10 {
            let uri = format!("cortex://threads/concurrent-test/timeline/{:06}-msg{}.md", i, i);
            state.filesystem.write(&uri, &format!("Message {}", i)).await.unwrap();
        }

        // 并发发送多个搜索请求
        let mut handles = vec![];
        
        for i in 0..10 {
            let app_clone = app.clone();
            let handle = tokio::spawn(async move {
                let request_body = json!({
                    "query": format!("Message {}", i),
                    "mode": "Filesystem",
                    "limit": 5
                });

                let request = Request::builder()
                    .uri("/search")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap();

                let response = app_clone.clone().oneshot(request).await.unwrap();
                response.status()
            });
            
            handles.push(handle);
        }

        // 等待所有请求完成
        let results = futures::future::join_all(handles).await;
        
        // 验证所有请求都成功
        for result in results {
            let status = result.unwrap();
            assert!(status.is_success());
        }
    }

    #[tokio::test]
    async fn test_error_handling_invalid_query() {
        let state = setup_test_state().await;
        let app = crate::routes::api_routes().with_state(state);

        // 发送无效的搜索请求（空query）
        let request_body = json!({
            "query": "",
            "mode": "Filesystem"
        });

        let (status, _): (_, Option<ApiResponse<Vec<SearchResultResponse>>>) = 
            post_json(&app, "/search", request_body).await;

        // 应该成功处理（返回空结果）
        assert!(status.is_success());
    }

    #[tokio::test]
    #[ignore]
    async fn test_hybrid_search_mode() {
        let state = setup_test_state().await;
        let app = crate::routes::api_routes().with_state(state.clone());

        // 创建测试数据
        state.filesystem.write(
            "cortex://threads/hybrid-test/timeline/msg1.md",
            "# Test\n\nHybrid search test content"
        ).await.unwrap();

        // 测试混合搜索
        let request_body = json!({
            "query": "hybrid search",
            "mode": "Hybrid",
            "limit": 10,
            "min_score": 0.3
        });

        let (status, _): (_, Option<ApiResponse<Vec<SearchResultResponse>>>) = 
            post_json(&app, "/search", request_body).await;

        // 混合搜索可能降级到文件系统搜索
        assert!(status.is_success() || status == StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = setup_test_state().await;
        let app = axum::Router::new()
            .route("/health", axum::routing::get(crate::handlers::health::health_check))
            .with_state(state);

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
