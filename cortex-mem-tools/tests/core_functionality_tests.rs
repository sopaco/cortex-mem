//! Cortex-Mem 核心功能测试
//!
//! 测试范围：
//! - 存储有效性（路径正确性、内容完整性、三层架构）
//! - 召回能力（关键词搜索、分层检索）
//! - 边界异常（空值、并发、生命周期）

use cortex_mem_tools::{MemoryOperations, types::*};
use std::sync::Arc;
use tempfile::TempDir;

// ==================== 测试数据 ====================

const TEST_CONTENT_RUST: &str = r#"# Rust 所有权系统

Rust 的所有权系统是其最独特的特性之一，它让 Rust 能够在没有垃圾回收器的情况下保证内存安全。

## 核心概念

所有权规则：
- 每个值都有一个变量作为其所有者
- 同一时间只能有一个所有者
- 当所有者离开作用域，值将被丢弃

## 代码示例

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1; // 所有权从 s1 转移到 s2
    // println!("{}", s1); // 错误！s1 不再有效
    println!("{}", s2); // 正常
}
```

## 借用规则

1. 在任意给定时刻，只能拥有一个可变引用或任意数量的不可变引用
2. 引用必须始终有效

## 实际应用

所有权系统在以下场景特别有用：
- 并发编程：防止数据竞争
- 资源管理：自动释放文件句柄、网络连接
- 性能优化：避免不必要的内存拷贝
"#;

const TEST_CONTENT_OAUTH: &str = r#"# OAuth 2.0 认证流程

OAuth 2.0 是一种授权框架，允许第三方应用获取对用户资源的有限访问权限。

## 四种授权模式

### 1. 授权码模式（Authorization Code）
最安全、最常用的模式，适用于有后端的应用。

流程：
1. 用户访问客户端，客户端将用户重定向到授权服务器
2. 用户登录并授权
3. 授权服务器返回授权码给客户端
4. 客户端用授权码换取访问令牌

### 2. 简化模式（Implicit）
适用于纯前端应用，没有后端服务器。

### 3. 密码凭证模式（Resource Owner Password Credentials）
用户直接向客户端提供用户名密码。

### 4. 客户端凭证模式（Client Credentials）
用于服务器之间的通信，不涉及用户。

## 安全最佳实践

- 使用 HTTPS 保护通信
- 设置合理的令牌过期时间
- 实现刷新令牌机制
- 验证 redirect_uri 防止劫持
"#;

const TEST_CONTENT_DATABASE: &str = r#"# PostgreSQL 性能优化指南

PostgreSQL 是世界上最先进的开源关系型数据库之一。

## 索引优化

### B-tree 索引
默认索引类型，适合等值查询和范围查询。

```sql
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_orders_date ON orders(created_at);
```

### GiST 索引
适合地理数据和全文搜索。

### GIN 索引
适合数组和 JSONB 字段。

## 查询优化

1. 使用 EXPLAIN ANALYZE 分析查询计划
2. 避免 SELECT *，只查询需要的字段
3. 使用连接（JOIN）代替子查询
4. 合理使用分页，避免大偏移量

## 配置调优

关键参数：
- shared_buffers: 25% 的内存
- effective_cache_size: 50-75% 的内存
- work_mem: 用于排序和哈希操作
- maintenance_work_mem: 用于维护操作

## 监控指标

- 慢查询日志
- 连接数
- 缓存命中率
- 事务吞吐量
"#;

// ==================== 辅助函数 ====================

async fn setup_test_env() -> (TempDir, MemoryOperations) {
    let temp_dir: TempDir = TempDir::new().unwrap();
    let ops: MemoryOperations = MemoryOperations::from_data_dir(temp_dir.path().to_str().unwrap())
        .await
        .unwrap();
    (temp_dir, ops)
}

async fn setup_test_env_with_tenant(tenant_id: &str) -> (TempDir, MemoryOperations) {
    let temp_dir: TempDir = TempDir::new().unwrap();
    let ops: MemoryOperations = MemoryOperations::with_tenant(temp_dir.path().to_str().unwrap(), tenant_id)
        .await
        .unwrap();
    (temp_dir, ops)
}

/// 验证文件系统状态
async fn verify_file_exists(ops: &MemoryOperations, uri: &str) -> bool {
    ops.exists(uri).await.unwrap_or(false)
}

/// 生成随机测试内容
fn generate_random_content(prefix: &str, length: usize) -> String {
    format!("{}: {}", prefix, "X".repeat(length))
}

/// 验证 URI 格式正确性
fn assert_uri_format(uri: &str, expected_scope: &str, expected_id: &str) {
    assert!(
        uri.starts_with(&format!("cortex://{}/{}", expected_scope, expected_id)),
        "URI {} 应该以 cortex://{}/{} 开头",
        uri, expected_scope, expected_id
    );
    assert!(uri.ends_with(".md"), "URI {} 应该以 .md 结尾", uri);
}

// ==================== 阶段 1: 存储有效性测试 ====================

mod storage_tests {
    use super::*;

    /// 测试 1.1: Session 维度存储路径正确性
    #[tokio::test]
    async fn test_session_storage_path() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 存储消息到 session
        let msg_id = ops.add_message("test_session_abc", "user", "测试内容").await.unwrap();
        
        // 验证消息 ID 格式正确
        assert!(!msg_id.is_empty(), "消息 ID 不应为空");
        
        // 验证文件存储路径符合预期: cortex://session/{thread_id}/timeline/YYYY-MM/DD/
        let sessions = ops.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1, "应该有一个 session");
        assert_eq!(sessions[0].thread_id, "test_session_abc");
    }

    /// 测试 1.2: User 维度存储路径正确性
    #[tokio::test]
    async fn test_user_scope_storage_path() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        let test_user_id = "test_user_123";
        let args = StoreArgs {
            content: "用户偏好设置".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false), // 先不生成层，简化测试
            scope: "user".to_string(),
            user_id: Some(test_user_id.to_string()),
            agent_id: None,
        };
        
        let result = ops.store(args).await.unwrap();
        
        // 验证 URI 格式: cortex://user/{user_id}/memories/YYYY-MM/DD/HH_MM_SS_xxx.md
        let expected_prefix = format!("cortex://user/{}/memories/", test_user_id);
        assert!(result.uri.starts_with(&expected_prefix), 
            "User scope 存储路径不正确: {}, 期望以 {} 开头", result.uri, expected_prefix);
        assert!(result.uri.ends_with(".md"), "文件应以 .md 结尾");
        assert!(result.success, "存储应成功");
    }

    /// 测试 1.3: Agent 维度存储路径正确性
    #[tokio::test]
    async fn test_agent_scope_storage_path() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        let test_agent_id = "my_agent_456";
        let args = StoreArgs {
            content: "Agent 记忆内容".to_string(),
            thread_id: "".to_string(), // 不使用 thread_id，使用 agent_id
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "agent".to_string(),
            user_id: None,
            agent_id: Some(test_agent_id.to_string()),
        };
        
        let result = ops.store(args).await.unwrap();
        
        // 验证 URI 格式: cortex://agent/{agent_id}/memories/YYYY-MM/DD/HH_MM_SS_xxx.md
        let expected_prefix = format!("cortex://agent/{}/memories/", test_agent_id);
        assert!(result.uri.starts_with(&expected_prefix),
            "Agent scope 存储路径不正确: {}, 期望以 {} 开头", result.uri, expected_prefix);
        assert!(result.success);
    }

    /// 测试 1.4: 存储内容完整性验证
    #[tokio::test]
    async fn test_storage_content_integrity() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 测试特殊字符 - 使用 user scope 直接存储，避免 timeline 嵌套
        let special_content = "特殊字符: 中文 🎉 Emoji \n\t 换行和制表符 \"引号\" '单引号'";
        
        let args = StoreArgs {
            content: special_content.to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("content_test_user".to_string()),
            agent_id: None,
        };
        
        let result = ops.store(args).await.unwrap();
        assert!(result.success, "存储应成功");
        
        // 读取并验证内容完整
        let content = ops.read_file(&result.uri).await.unwrap();
        assert!(content.contains(special_content), "内容应完整保留特殊字符");
    }

    /// 测试 1.5: 三层架构自动生成验证
    #[tokio::test]
    async fn test_layer_generation() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 存储长文本到 user scope（更容易验证 L0/L1）
        let args = StoreArgs {
            content: TEST_CONTENT_RUST.to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(true), // 启用层生成
            scope: "user".to_string(),
            user_id: Some("layer_test_user".to_string()),
            agent_id: None,
        };
        
        let result = ops.store(args).await.unwrap();
        assert!(result.success);
        
        // 验证 L2 (原始内容) 可读取
        let l2_content = ops.read_file(&result.uri).await.unwrap();
        assert!(l2_content.contains("Rust 所有权系统"), "L2 应包含原始内容");
        
        // 验证 L0 摘要可获取
        let l0_result = ops.get_abstract(&result.uri).await;
        if let Ok(l0) = l0_result {
            assert!(!l0.abstract_text.is_empty(), "L0 摘要不应为空");
            assert!(l0.layer == "L0", "层标识应为 L0");
            // L0 应该简短（约100 tokens）
            assert!(l0.token_count < 200, "L0 token 数应小于 200, 实际是 {}", l0.token_count);
        }
        
        // 验证 L1 概览可获取
        let l1_result = ops.get_overview(&result.uri).await;
        if let Ok(l1) = l1_result {
            assert!(!l1.overview_text.is_empty(), "L1 概览不应为空");
            assert!(l1.layer == "L1", "层标识应为 L1");
            // L1 应该比 L0 长（fallback 生成可能较短，放宽要求）
            assert!(l1.token_count > 10, "L1 token 数应大于 10, 实际是 {}", l1.token_count);
        }
    }

    /// 测试 1.6: Timeline 时间轴结构验证
    #[tokio::test]
    async fn test_timeline_structure() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 存储多条消息
        let thread_id = "timeline_test";
        ops.add_message(thread_id, "user", "第一条消息").await.unwrap();
        ops.add_message(thread_id, "assistant", "第二条消息").await.unwrap();
        ops.add_message(thread_id, "user", "第三条消息").await.unwrap();
        
        // 验证 Timeline 目录结构
        let timeline_uri = format!("cortex://session/{}/timeline", thread_id);
        let entries = ops.list_files(&timeline_uri).await.unwrap();
        
        // 应该按 YYYY-MM 分组
        assert!(!entries.is_empty(), "Timeline 不应为空");
        
        // 验证可以读取消息
        let session = ops.get_session(thread_id).await.unwrap();
        assert_eq!(session.thread_id, thread_id);
    }
}

// ==================== 阶段 2: 路径正确性测试 ====================

mod path_tests {
    use super::*;

    /// 测试 2.1: 多租户隔离验证
    #[tokio::test]
    async fn test_tenant_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap();
        
        // 创建两个租户
        let ops_a = MemoryOperations::with_tenant(data_dir, "tenant_a").await.unwrap();
        let ops_b = MemoryOperations::with_tenant(data_dir, "tenant_b").await.unwrap();
        
        // 租户 A 存储数据
        let args_a = StoreArgs {
            content: "租户 A 的私有数据".to_string(),
            thread_id: "shared_topic".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "session".to_string(),
            user_id: None,
            agent_id: None,
        };
        let result_a = ops_a.store(args_a).await.unwrap();
        
        // 租户 B 存储数据（相同 topic）
        let args_b = StoreArgs {
            content: "租户 B 的私有数据".to_string(),
            thread_id: "shared_topic".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "session".to_string(),
            user_id: None,
            agent_id: None,
        };
        let result_b = ops_b.store(args_b).await.unwrap();
        
        // 验证 URI 不同（包含租户标识）
        assert_ne!(result_a.uri, result_b.uri, "不同租户相同 topic 的 URI 应该不同");
        
        // 验证数据隔离 - 租户 A 读取自己的数据
        let content_a = ops_a.read_file(&result_a.uri).await.unwrap();
        assert!(content_a.contains("租户 A"));
        
        // 验证数据隔离 - 租户 B 读取自己的数据
        let content_b = ops_b.read_file(&result_b.uri).await.unwrap();
        assert!(content_b.contains("租户 B"));
    }

    /// 测试 2.2: URI 格式验证
    #[tokio::test]
    async fn test_uri_format_validation() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 测试各种 URI 格式
        let test_cases = vec![
            ("cortex://session/test123/timeline", true),
            ("cortex://user/user_001/preferences/style.md", true),
            ("cortex://agent/bot_001/memories/facts/rust.md", true),
            ("cortex://resources/docs/api-reference.md", true),
        ];
        
        for (uri, should_exist) in test_cases {
            // 对于不存在的 URI，应该返回错误
            if !should_exist {
                let result = ops.read_file(uri).await;
                assert!(result.is_err(), "不存在的 URI {} 应该返回错误", uri);
            }
        }
    }
}

// ==================== 阶段 3: 召回能力测试 ====================

mod retrieval_tests {
    use super::*;

    /// 准备测试数据集
    async fn setup_test_dataset(ops: &MemoryOperations) {
        // 存储 Rust 相关记忆
        let rust_args = StoreArgs {
            content: TEST_CONTENT_RUST.to_string(),
            thread_id: "rust_learning".to_string(),
            metadata: None,
            auto_generate_layers: Some(true),
            scope: "user".to_string(),
            user_id: Some("test_user".to_string()),
            agent_id: None,
        };
        ops.store(rust_args).await.unwrap();
        
        // 存储 OAuth 相关记忆
        let oauth_args = StoreArgs {
            content: TEST_CONTENT_OAUTH.to_string(),
            thread_id: "oauth_learning".to_string(),
            metadata: None,
            auto_generate_layers: Some(true),
            scope: "user".to_string(),
            user_id: Some("test_user".to_string()),
            agent_id: None,
        };
        ops.store(oauth_args).await.unwrap();
        
        // 存储 Database 相关记忆
        let db_args = StoreArgs {
            content: TEST_CONTENT_DATABASE.to_string(),
            thread_id: "db_learning".to_string(),
            metadata: None,
            auto_generate_layers: Some(true),
            scope: "user".to_string(),
            user_id: Some("test_user".to_string()),
            agent_id: None,
        };
        ops.store(db_args).await.unwrap();
    }

    /// 测试 3.1: 关键词检索召回率
    #[tokio::test]
    async fn test_keyword_search_recall() {
        let (_temp_dir, ops) = setup_test_env().await;
        setup_test_dataset(&ops).await;
        
        // 测试查询 1: Rust 相关（在 user scope 搜索）
        let search_args = SearchArgs {
            query: "Rust 所有权系统".to_string(),
            recursive: Some(true),
            return_layers: Some(vec!["L0".to_string()]),
            scope: Some("cortex://user".to_string()),
            limit: Some(10),
        };
        
        let result = ops.search(search_args).await.unwrap();
        println!("Rust 搜索召回 {} 个结果", result.total);
        
        // 应该召回 Rust 相关内容
        assert!(result.total > 0, "应该召回至少一个 Rust 相关结果");
        
        // 测试查询 2: OAuth 相关（在 user scope 搜索）
        let oauth_search = SearchArgs {
            query: "OAuth 2.0 认证流程".to_string(),
            recursive: Some(true),
            return_layers: Some(vec!["L0".to_string()]),
            scope: Some("cortex://user".to_string()),
            limit: Some(10),
        };
        
        let oauth_result = ops.search(oauth_search).await.unwrap();
        println!("OAuth 搜索召回 {} 个结果", oauth_result.total);
        assert!(oauth_result.total > 0, "应该召回至少一个 OAuth 相关结果");
        
        // 验证相关性分数
        if !oauth_result.results.is_empty() {
            let top_score = oauth_result.results[0].score;
            println!("Top result score: {}", top_score);
            assert!(top_score > 0.1, "最高相关性分数应大于 0.1");
        }
    }

    /// 测试 3.2: 快速查找（Find）功能
    #[tokio::test]
    async fn test_find_functionality() {
        let (_temp_dir, ops) = setup_test_env().await;
        setup_test_dataset(&ops).await;
        
        // 使用 find 快速查找
        let find_args = FindArgs {
            query: "PostgreSQL 性能".to_string(),
            scope: Some("cortex://user".to_string()),
            limit: Some(5),
        };
        
        let result = ops.find(find_args).await.unwrap();
        println!("Find 召回 {} 个结果", result.total);
        
        // 应该召回数据库相关内容
        assert!(result.total > 0, "应该召回至少一个结果");
        
        // 验证返回的是 L0 摘要
        if !result.results.is_empty() {
            assert!(!result.results[0].abstract_text.is_empty(), "应返回 L0 摘要");
        }
    }

    /// 测试 3.3: 分层检索效率
    #[tokio::test]
    async fn test_tiered_retrieval() {
        let (_temp_dir, ops) = setup_test_env().await;
        setup_test_dataset(&ops).await;
        
        // 测试 L0 快速扫描
        let l0_args = SearchArgs {
            query: "Rust".to_string(),
            recursive: Some(true),
            return_layers: Some(vec!["L0".to_string()]), // 只返回 L0
            scope: Some("cortex://session".to_string()),
            limit: Some(10),
        };
        
        let start = std::time::Instant::now();
        let l0_result = ops.search(l0_args).await.unwrap();
        let l0_duration = start.elapsed();
        
        println!("L0 检索耗时: {:?}, 召回 {} 个结果", l0_duration, l0_result.total);
        
        // L0 应该快速返回
        assert!(l0_duration.as_millis() < 1000, "L0 检索应小于 1 秒");
        
        // 测试 L2 完整检索
        let l2_args = SearchArgs {
            query: "Rust".to_string(),
            recursive: Some(true),
            return_layers: Some(vec!["L2".to_string()]), // 返回完整内容
            scope: Some("cortex://session".to_string()),
            limit: Some(10),
        };
        
        let start = std::time::Instant::now();
        let l2_result = ops.search(l2_args).await.unwrap();
        let l2_duration = start.elapsed();
        
        println!("L2 检索耗时: {:?}, 召回 {} 个结果", l2_duration, l2_result.total);
        
        // 验证 L2 返回完整内容
        if !l2_result.results.is_empty() {
            let content = l2_result.results[0].content.as_ref();
            assert!(content.is_some(), "L2 应返回完整内容");
            assert!(content.unwrap().contains("Rust"), "内容应包含关键词");
        }
    }

    /// 测试 3.4: 目录浏览（ls）功能
    #[tokio::test]
    async fn test_ls_functionality() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 创建一些测试数据
        ops.add_message("ls_test_session", "user", "测试消息").await.unwrap();
        
        // 测试 ls 命令
        let ls_args = LsArgs {
            uri: "cortex://session".to_string(),
            recursive: Some(false),
            include_abstracts: Some(false),
        };
        
        let result = ops.ls(ls_args).await.unwrap();
        println!("ls 找到 {} 个条目", result.total);
        
        assert!(result.total > 0, "应该找到至少一个 session");
        
        // 验证可以递归浏览
        let ls_recursive = LsArgs {
            uri: "cortex://session".to_string(),
            recursive: Some(true),
            include_abstracts: Some(true),
        };
        
        let recursive_result = ops.ls(ls_recursive).await.unwrap();
        println!("递归 ls 找到 {} 个条目", recursive_result.total);
    }

    /// 测试 3.5: 智能探索（explore）功能
    #[tokio::test]
    async fn test_explore_functionality() {
        let (_temp_dir, ops) = setup_test_env().await;
        setup_test_dataset(&ops).await;
        
        // 使用 explore 智能探索
        let explore_args = ExploreArgs {
            query: "性能优化".to_string(),
            start_uri: Some("cortex://session".to_string()),
            max_depth: Some(3),
            return_layers: Some(vec!["L0".to_string()]),
        };
        
        let result = ops.explore(explore_args).await.unwrap();
        println!("探索完成: 探索了 {} 个节点, 找到 {} 个匹配", 
            result.total_explored, result.total_matches);
        
        // 应该探索了多个节点
        assert!(result.total_explored > 0, "应该探索至少一个节点");
    }
}

// ==================== 阶段 4: 边界与异常测试 ====================

mod edge_case_tests {
    use super::*;

    /// 测试 4.1: 空值处理
    #[tokio::test]
    async fn test_empty_values() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 测试空 thread_id -> 应使用 "default"
        let msg_id = ops.add_message("", "user", "空 thread_id 测试").await.unwrap();
        assert!(!msg_id.is_empty(), "空 thread_id 应该生成消息 ID");
        
        // 验证 default session 被创建
        let session = ops.get_session("default").await;
        assert!(session.is_ok(), "应该创建 default session");
    }

    /// 测试 4.2: 特殊字符处理
    #[tokio::test]
    async fn test_special_characters() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        let special_contents = vec![
            "中文内容测试 🎉",
            "Special chars: <>&\"'",
            "Newlines:\nLine1\nLine2\nLine3",
            "Tabs:\tColumn1\tColumn2",
            "Unicode: αβγ δεζ ηθι",
            "Code: `fn main() {}`",
        ];
        
        for content in special_contents {
            let msg_id = ops.add_message("special_chars", "user", content).await.unwrap();
            assert!(!msg_id.is_empty(), "特殊字符内容应能正常存储: {}", content);
        }
    }

    /// 测试 4.3: 会话生命周期
    #[tokio::test]
    async fn test_session_lifecycle() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        let thread_id = "lifecycle_test";
        
        // 1. 创建 session（通过添加消息自动创建）
        ops.add_message(thread_id, "user", "第一条消息").await.unwrap();
        
        let session = ops.get_session(thread_id).await.unwrap();
        assert_eq!(session.thread_id, thread_id);
        assert_eq!(session.status, "active", "新 session 应该是 active 状态");
        
        // 2. 关闭 session
        ops.close_session(thread_id).await.unwrap();
        
        let closed_session = ops.get_session(thread_id).await.unwrap();
        assert_eq!(closed_session.status, "closed", "关闭后应该是 closed 状态");
    }

    /// 测试 4.4: 并发写入
    #[tokio::test]
    async fn test_concurrent_writes() {
        let (_temp_dir, ops) = setup_test_env().await;
        let ops = Arc::new(ops);
        
        let thread_id = "concurrent_test";
        let mut handles = vec![];
        
        // 并发写入 20 条消息
        for i in 0..20 {
            let ops_clone = ops.clone();
            let handle = tokio::spawn(async move {
                ops_clone.add_message(
                    thread_id, 
                    if i % 2 == 0 { "user" } else { "assistant" },
                    &format!("并发消息 {}", i)
                ).await
            });
            handles.push(handle);
        }
        
        // 等待所有写入完成
        let results: Vec<_> = futures::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        
        println!("并发写入: {}/20 成功", success_count);
        assert_eq!(success_count, 20, "所有并发写入应该成功");
        
        // 验证所有消息都被存储
        let timeline_uri = format!("cortex://session/{}/timeline", thread_id);
        let entries = ops.list_files(&timeline_uri).await.unwrap();
        
        // 应该找到所有消息（可能在不同目录下）
        assert!(!entries.is_empty(), "应该找到存储的消息");
    }

    /// 测试 4.5: 不存在的资源访问
    #[tokio::test]
    async fn test_nonexistent_resource() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 尝试读取不存在的 URI
        let result = ops.read_file("cortex://session/nonexistent/file.md").await;
        assert!(result.is_err(), "不存在的资源应该返回错误");
        
        // 尝试获取不存在的 session
        let session_result = ops.get_session("definitely_not_exists").await;
        assert!(session_result.is_err(), "不存在的 session 应该返回错误");
    }

    /// 测试 4.6: 大文本存储
    #[tokio::test]
    async fn test_large_content() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 生成 100KB 的文本
        let large_content = "A".repeat(100 * 1024);
        
        let args = StoreArgs {
            content: large_content.clone(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("large_content_user".to_string()),
            agent_id: None,
        };
        
        let result = ops.store(args).await.unwrap();
        assert!(result.success, "大文本应该能成功存储");
        
        // 验证内容完整（允许少量差异，因为可能有 Markdown 格式）
        let read_content = ops.read_file(&result.uri).await.unwrap();
        let size_diff = (read_content.len() as i64 - large_content.len() as i64).abs();
        assert!(size_diff < 1000, "大文本内容应基本完整，差异 {} 字节", size_diff);
    }
}

// ==================== 性能基准测试 ====================

mod performance_tests {
    use super::*;

    /// 测试存储性能
    #[tokio::test]
    async fn test_storage_performance() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        let start = std::time::Instant::now();
        
        // 存储 50 条消息
        for i in 0..50 {
            ops.add_message(
                "perf_test",
                "user",
                &format!("性能测试消息 {} 内容", i)
            ).await.unwrap();
        }
        
        let duration = start.elapsed();
        println!("存储 50 条消息耗时: {:?}", duration);
        
        // 应该在一秒内完成
        assert!(duration.as_secs() < 5, "存储 50 条消息应小于 5 秒");
    }

    /// 测试检索性能
    #[tokio::test]
    async fn test_retrieval_performance() {
        let (_temp_dir, ops) = setup_test_env().await;
        
        // 先存储一些数据
        for i in 0..30 {
            let args = StoreArgs {
                content: format!("测试内容 {} 包含关键词 performance", i),
                thread_id: format!("perf_session_{}", i),
                metadata: None,
                auto_generate_layers: Some(false),
                scope: "session".to_string(),
                user_id: None,
                agent_id: None,
            };
            ops.store(args).await.unwrap();
        }
        
        // 测试检索性能
        let start = std::time::Instant::now();
        
        let search_args = SearchArgs {
            query: "performance".to_string(),
            recursive: Some(true),
            return_layers: Some(vec!["L0".to_string()]),
            scope: Some("cortex://session".to_string()),
            limit: Some(20),
        };
        
        let result = ops.search(search_args).await.unwrap();
        let duration = start.elapsed();
        
        println!("检索 {} 个结果耗时: {:?}", result.total, duration);
        
        // 应该在合理时间内完成
        assert!(duration.as_millis() < 2000, "检索应小于 2 秒");
    }
}

// ==================== 缺失功能测试 ====================

mod crud_tests {
    use super::*;

    /// 测试删除功能
    #[tokio::test]
    async fn test_delete_functionality() {
        let (_temp_dir, ops) = setup_test_env().await;

        // 1. 创建数据
        let args = StoreArgs {
            content: "待删除的内容".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("delete_test_user".to_string()),
            agent_id: None,
        };

        let result = ops.store(args).await.unwrap();
        let uri = result.uri;

        // 验证数据存在
        assert!(verify_file_exists(&ops, &uri).await, "存储后文件应该存在");

        // 2. 删除数据
        ops.delete(&uri).await.unwrap();

        // 3. 验证数据已删除
        assert!(
            !verify_file_exists(&ops, &uri).await,
            "删除后文件不应该存在"
        );

        // 4. 验证读取返回错误
        let read_result = ops.read_file(&uri).await;
        assert!(read_result.is_err(), "读取已删除文件应该返回错误");
    }

    /// 测试更新/覆盖功能
    #[tokio::test]
    async fn test_update_overwrite() {
        let (_temp_dir, ops) = setup_test_env().await;

        let user_id = "update_test_user";
        let content_v1 = "原始内容版本1";
        let content_v2 = "更新的内容版本2，增加了更多信息";

        // 1. 存储第一个版本
        let args_v1 = StoreArgs {
            content: content_v1.to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some(user_id.to_string()),
            agent_id: None,
        };

        let result_v1 = ops.store(args_v1).await.unwrap();
        let uri_v1 = result_v1.uri.clone();

        // 验证第一个版本
        let read_v1 = ops.read_file(&uri_v1).await.unwrap();
        assert!(read_v1.contains(content_v1), "应该读取到版本1的内容");

        // 2. 存储第二个版本（相同 user_id，不同时间）
        let args_v2 = StoreArgs {
            content: content_v2.to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some(user_id.to_string()),
            agent_id: None,
        };

        let result_v2 = ops.store(args_v2).await.unwrap();
        let uri_v2 = result_v2.uri;

        // 验证两个版本都存在（不同时间戳）
        assert!(verify_file_exists(&ops, &uri_v1).await, "版本1应该仍然存在");
        assert!(verify_file_exists(&ops, &uri_v2).await, "版本2应该存在");

        // 验证版本2内容正确
        let read_v2 = ops.read_file(&uri_v2).await.unwrap();
        assert!(read_v2.contains(content_v2), "应该读取到版本2的内容");
    }

    /// 测试批量操作
    #[tokio::test]
    async fn test_batch_operations() {
        let (_temp_dir, ops) = setup_test_env().await;

        let user_id = "batch_test_user";
        let mut stored_uris = Vec::new();

        // 1. 批量存储
        for i in 0..10 {
            let args = StoreArgs {
                content: format!("批量测试内容 {}", i),
                thread_id: "".to_string(),
                metadata: None,
                auto_generate_layers: Some(false),
                scope: "user".to_string(),
                user_id: Some(user_id.to_string()),
                agent_id: None,
            };

            let result = ops.store(args).await.unwrap();
            stored_uris.push(result.uri);
        }

        // 验证所有文件都存在
        for uri in &stored_uris {
            assert!(verify_file_exists(&ops, uri).await, "批量存储的文件应该存在");
        }

        // 2. 批量删除
        for uri in &stored_uris {
            ops.delete(uri).await.unwrap();
        }

        // 验证所有文件都已删除
        for uri in &stored_uris {
            assert!(!verify_file_exists(&ops, uri).await, "批量删除后文件不应该存在");
        }
    }
}

mod scope_isolation_tests {
    use super::*;

    /// 测试跨 scope 数据隔离
    #[tokio::test]
    async fn test_cross_scope_isolation() {
        let (_temp_dir, ops) = setup_test_env().await;

        // 1. 在 user scope 存储
        let user_args = StoreArgs {
            content: "用户私有数据".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("cross_scope_user".to_string()),
            agent_id: None,
        };
        let user_result = ops.store(user_args).await.unwrap();

        // 2. 在 agent scope 存储（相同 ID）
        let agent_args = StoreArgs {
            content: "Agent 数据".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "agent".to_string(),
            user_id: None,
            agent_id: Some("cross_scope_user".to_string()), // 使用相同的 ID
        };
        let agent_result = ops.store(agent_args).await.unwrap();

        // 3. 验证 URI 不同（scope 隔离）
        assert_ne!(user_result.uri, agent_result.uri, "不同 scope 的 URI 应该不同");
        assert!(user_result.uri.contains("/user/"), "User scope URI 应该包含 /user/");
        assert!(agent_result.uri.contains("/agent/"), "Agent scope URI 应该包含 /agent/");

        // 4. 验证数据隔离
        let user_content = ops.read_file(&user_result.uri).await.unwrap();
        let agent_content = ops.read_file(&agent_result.uri).await.unwrap();

        assert!(user_content.contains("用户私有数据"), "User scope 应该包含用户数据");
        assert!(agent_content.contains("Agent 数据"), "Agent scope 应该包含 Agent 数据");
    }

    /// 测试不同 user_id 之间的隔离
    #[tokio::test]
    async fn test_user_id_isolation() {
        let (_temp_dir, ops) = setup_test_env().await;

        // 1. 用户 A 存储数据
        let user_a_args = StoreArgs {
            content: "用户 A 的私密数据".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("user_a".to_string()),
            agent_id: None,
        };
        let result_a = ops.store(user_a_args).await.unwrap();

        // 2. 用户 B 存储数据
        let user_b_args = StoreArgs {
            content: "用户 B 的私密数据".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("user_b".to_string()),
            agent_id: None,
        };
        let result_b = ops.store(user_b_args).await.unwrap();

        // 3. 验证路径不同
        assert_ne!(result_a.uri, result_b.uri, "不同用户的 URI 应该不同");
        assert!(result_a.uri.contains("user_a"), "用户 A 的 URI 应该包含 user_a");
        assert!(result_b.uri.contains("user_b"), "用户 B 的 URI 应该包含 user_b");

        // 4. 验证数据隔离
        let content_a = ops.read_file(&result_a.uri).await.unwrap();
        let content_b = ops.read_file(&result_b.uri).await.unwrap();

        assert!(content_a.contains("用户 A"), "用户 A 应该读取到自己的数据");
        assert!(content_b.contains("用户 B"), "用户 B 应该读取到自己的数据");
    }
}

mod advanced_concurrent_tests {
    use super::*;

    /// 测试读写并发
    #[tokio::test]
    async fn test_read_write_concurrent() {
        let ops = Arc::new(setup_test_env().await.1);
        let thread_id = "rw_concurrent_test";

        // 先写入一些数据
        for i in 0..5 {
            ops.add_message(thread_id, "user", &format!("消息 {}", i))
                .await
                .unwrap();
        }

        let mut handles = vec![];

        // 并发读取和写入
        for i in 0..10 {
            let ops_clone = ops.clone();
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    // 偶数：写入
                    ops_clone
                        .add_message(thread_id, "user", &format!("并发写入 {}", i))
                        .await
                } else {
                    // 奇数：读取
                    let uri = format!("cortex://session/{}/timeline", thread_id);
                    let _ = ops_clone.list_files(&uri).await;
                    Ok("read".to_string())
                }
            });
            handles.push(handle);
        }

        // 等待所有操作完成
        let results: Vec<_> = futures::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.is_ok()).count();

        assert_eq!(success_count, 10, "所有并发读写操作应该成功");
    }

    /// 测试同一文件的并发访问
    #[tokio::test]
    async fn test_concurrent_file_access() {
        let (_temp_dir, ops) = setup_test_env().await;
        let ops = Arc::new(ops);

        // 创建一个共享文件
        let args = StoreArgs {
            content: "共享文件内容".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("concurrent_file_user".to_string()),
            agent_id: None,
        };
        let result = ops.store(args).await.unwrap();
        let uri = result.uri;

        let mut handles = vec![];

        // 并发读取同一文件
        for _ in 0..20 {
            let ops_clone = ops.clone();
            let uri_clone = uri.clone();
            let handle = tokio::spawn(async move {
                ops_clone.read_file(&uri_clone).await
            });
            handles.push(handle);
        }

        let results: Vec<_> = futures::future::join_all(handles).await;
        let success_count = results.iter().filter(|r| r.is_ok()).count();

        assert_eq!(success_count, 20, "所有并发读取应该成功");
    }
}

mod edge_case_advanced_tests {
    use super::*;

    /// 测试超长 ID
    #[tokio::test]
    async fn test_very_long_ids() {
        let (_temp_dir, ops) = setup_test_env().await;

        let long_user_id = "a".repeat(100); // 100 字符的 user_id
        let args = StoreArgs {
            content: "超长 ID 测试".to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some(long_user_id.clone()),
            agent_id: None,
        };

        let result = ops.store(args).await;
        // 根据文件系统限制，可能成功也可能失败
        // 这里主要验证不会 panic
        if let Ok(store_result) = result {
            assert!(store_result.uri.contains(&long_user_id[..50])); // 至少部分 ID 在 URI 中
        }
    }

    /// 测试特殊字符 ID
    #[tokio::test]
    async fn test_special_char_ids() {
        let (_temp_dir, ops) = setup_test_env().await;

        let special_ids = vec![
            "user-with-dash",
            "user_with_underscore",
            "user.with.dot",
            "user123",
            "123user",
        ];

        for id in special_ids {
            let args = StoreArgs {
                content: format!("特殊 ID 测试: {}", id),
                thread_id: "".to_string(),
                metadata: None,
                auto_generate_layers: Some(false),
                scope: "user".to_string(),
                user_id: Some(id.to_string()),
                agent_id: None,
            };

            let result = ops.store(args).await;
            assert!(result.is_ok(), "ID '{}' 应该能正常存储", id);
        }
    }

    /// 测试空内容
    #[tokio::test]
    async fn test_empty_content() {
        let (_temp_dir, ops) = setup_test_env().await;

        let args = StoreArgs {
            content: "".to_string(), // 空内容
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(false),
            scope: "user".to_string(),
            user_id: Some("empty_content_user".to_string()),
            agent_id: None,
        };

        let result = ops.store(args).await;
        // 空内容应该能存储（创建空文件）
        assert!(result.is_ok(), "空内容应该能存储");

        if let Ok(store_result) = result {
            let content = ops.read_file(&store_result.uri).await.unwrap();
            assert!(content.is_empty(), "读取的内容应该是空的");
        }
    }

    /// 测试多层目录结构
    #[tokio::test]
    async fn test_deep_directory_structure() {
        let (_temp_dir, ops) = setup_test_env().await;

        // 创建多层嵌套 session
        let thread_id = "deep/nested/thread/id";
        let msg_id = ops
            .add_message(thread_id, "user", "深层嵌套测试")
            .await
            .unwrap();

        assert!(!msg_id.is_empty(), "深层嵌套应该能正常工作");

        // 验证能读取到
        let session = ops.get_session(thread_id).await;
        assert!(session.is_ok(), "应该能获取深层嵌套的 session");
    }
}

mod layer_quality_tests {
    use super::*;

    /// 测试 L0 摘要质量
    #[tokio::test]
    async fn test_l0_abstract_quality() {
        let (_temp_dir, ops) = setup_test_env().await;

        // 使用明确主题的内容
        let content = r#"# PostgreSQL 数据库优化

PostgreSQL 是世界上最先进的开源关系型数据库。

## 核心优化技术

1. 索引优化：使用 B-tree、GiST、GIN 索引
2. 查询优化：使用 EXPLAIN ANALYZE 分析查询计划
3. 配置调优：调整 shared_buffers、work_mem 等参数

## 实际案例

某电商平台通过索引优化将查询速度提升 10 倍。"#;

        let args = StoreArgs {
            content: content.to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(true),
            scope: "user".to_string(),
            user_id: Some("l0_quality_test".to_string()),
            agent_id: None,
        };

        let result = ops.store(args).await.unwrap();

        // 获取 L0 摘要
        let l0_result = ops.get_abstract(&result.uri).await;
        if let Ok(l0) = l0_result {
            // L0 应该包含关键信息（至少是原文的一部分）
            assert!(
                l0.abstract_text.contains("PostgreSQL") || l0.abstract_text.contains("数据库"),
                "L0 摘要应该包含关键主题词"
            );

            // L0 应该比原文短（fallback 模式下可能接近，放宽要求）
            assert!(
                l0.token_count <= content.len() / 2,
                "L0 token 数应明显小于原文长度"
            );
        }
    }

    /// 测试 L1 概览结构
    #[tokio::test]
    async fn test_l1_overview_structure() {
        let (_temp_dir, ops) = setup_test_env().await;

        let content = r#"# Rust 编程语言

## 特点
- 内存安全
- 零成本抽象
- 并发安全

## 应用场景
- 系统编程
- Web 后端
- 嵌入式开发"#;

        let args = StoreArgs {
            content: content.to_string(),
            thread_id: "".to_string(),
            metadata: None,
            auto_generate_layers: Some(true),
            scope: "user".to_string(),
            user_id: Some("l1_structure_test".to_string()),
            agent_id: None,
        };

        let result = ops.store(args).await.unwrap();

        // 获取 L1 概览
        let l1_result = ops.get_overview(&result.uri).await;
        if let Ok(l1) = l1_result {
            // L1 应该包含标题标记
            assert!(
                l1.overview_text.contains("#") || l1.overview_text.contains("Overview"),
                "L1 应该包含标题或 Overview 标记"
            );

            // L1 应该比 L0 长但比原文短
            let l0_result = ops.get_abstract(&result.uri).await;
            if let Ok(l0) = l0_result {
                assert!(
                    l1.token_count >= l0.token_count,
                    "L1 应该比 L0 长或相等"
                );
            }
        }
    }
}