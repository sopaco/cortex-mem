# Cortex-Memory 3.0 测试用例设计

> 单元测试、集成测试、性能基准测试的详细测试用例

---

## 一、测试策略

### 1.1 测试金字塔

```
        /\
       /  \
      / UI \         - E2E 测试（5%）
     /------\
    /        \       - 集成测试（25%）
   / Integration\
  /--------------\
 /  Unit Tests   \  - 单元测试（70%）
/------------------\
```

### 1.2 测试覆盖率目标

| 模块 | 单元测试 | 集成测试 | 性能测试 |
|------|---------|---------|---------|
| 层级生成 | > 85% | ✓ | - |
| 递归检索 | > 85% | ✓ | ✓ |
| 意图分析 | > 80% | ✓ | ✓ |
| 记忆去重 | > 85% | ✓ | - |
| 性能优化 | > 75% | - | ✓ |

### 1.3 测试环境

- **Unit**: Mock 所有外部依赖
- **Integration**: 本地 Qdrant + 本地文件系统
- **Performance**: 真实数据集（LOMOCO）

---

## 二、阶段 0: 当前问题修复测试

### 2.1 三层文件补全测试

#### UT-0.1.1: 目录扫描测试

**测试文件**: `cortex-mem-core/src/automation/layer_generator_test.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_scan_all_directories() {
        // Setup: 创建测试文件系统
        let fs = setup_test_filesystem().await;
        fs.mkdir("cortex://user/memories/preferences").await.unwrap();
        fs.mkdir("cortex://agent/cases").await.unwrap();
        fs.mkdir("cortex://resources/docs").await.unwrap();
        
        // Execute
        let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
        let dirs = generator.scan_all_directories().await.unwrap();
        
        // Verify
        assert!(dirs.contains(&"cortex://user/memories/preferences".to_string()));
        assert!(dirs.contains(&"cortex://agent/cases".to_string()));
        assert!(dirs.contains(&"cortex://resources/docs".to_string()));
        assert_eq!(dirs.len(), 3);
    }
    
    #[tokio::test]
    async fn test_scan_empty_filesystem() {
        let fs = setup_test_filesystem().await;
        let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
        
        let dirs = generator.scan_all_directories().await.unwrap();
        
        assert_eq!(dirs.len(), 0);
    }
    
    #[tokio::test]
    async fn test_scan_nested_directories() {
        let fs = setup_test_filesystem().await;
        fs.mkdir("cortex://user/memories/preferences/communication").await.unwrap();
        fs.mkdir("cortex://user/memories/preferences/code_style").await.unwrap();
        
        let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
        let dirs = generator.scan_all_directories().await.unwrap();
        
        // 应该包含所有层级的目录
        assert!(dirs.contains(&"cortex://user/memories/preferences".to_string()));
        assert!(dirs.contains(&"cortex://user/memories/preferences/communication".to_string()));
        assert!(dirs.contains(&"cortex://user/memories/preferences/code_style".to_string()));
    }
}
```

**验收标准**:
- [ ] 覆盖空文件系统场景
- [ ] 覆盖多维度目录场景
- [ ] 覆盖嵌套目录场景
- [ ] 测试覆盖率 > 90%

---

#### UT-0.1.2: 缺失检测测试

```rust
#[tokio::test]
async fn test_has_layers_with_both_files() {
    let fs = setup_test_filesystem().await;
    fs.mkdir("cortex://user/memories").await.unwrap();
    fs.write("cortex://user/memories/.abstract", "Summary").await.unwrap();
    fs.write("cortex://user/memories/.overview", "Overview").await.unwrap();
    
    let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
    
    let has_layers = generator.has_layers("cortex://user/memories").await.unwrap();
    assert!(has_layers);
}

#[tokio::test]
async fn test_has_layers_missing_abstract() {
    let fs = setup_test_filesystem().await;
    fs.mkdir("cortex://user/memories").await.unwrap();
    fs.write("cortex://user/memories/.overview", "Overview").await.unwrap();
    
    let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
    
    let has_layers = generator.has_layers("cortex://user/memories").await.unwrap();
    assert!(!has_layers);
}

#[tokio::test]
async fn test_has_layers_missing_overview() {
    let fs = setup_test_filesystem().await;
    fs.mkdir("cortex://user/memories").await.unwrap();
    fs.write("cortex://user/memories/.abstract", "Summary").await.unwrap();
    
    let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
    
    let has_layers = generator.has_layers("cortex://user/memories").await.unwrap();
    assert!(!has_layers);
}

#[tokio::test]
async fn test_filter_missing_layers() {
    let fs = setup_test_filesystem().await;
    
    // 创建三个目录：一个完整，两个缺失
    fs.mkdir("cortex://user/complete").await.unwrap();
    fs.write("cortex://user/complete/.abstract", "A").await.unwrap();
    fs.write("cortex://user/complete/.overview", "O").await.unwrap();
    
    fs.mkdir("cortex://user/missing1").await.unwrap();
    fs.mkdir("cortex://user/missing2").await.unwrap();
    
    let generator = LayerGenerator::new(fs, mock_llm_client(), default_config());
    
    let all_dirs = vec![
        "cortex://user/complete".to_string(),
        "cortex://user/missing1".to_string(),
        "cortex://user/missing2".to_string(),
    ];
    
    let missing = generator.filter_missing_layers(&all_dirs).await.unwrap();
    
    assert_eq!(missing.len(), 2);
    assert!(missing.contains(&"cortex://user/missing1".to_string()));
    assert!(missing.contains(&"cortex://user/missing2".to_string()));
}
```

**验收标准**:
- [ ] 覆盖所有缺失场景（无文件、缺 abstract、缺 overview）
- [ ] 测试覆盖率 > 95%

---

#### UT-0.1.3: 渐进式生成测试

```rust
#[tokio::test]
async fn test_ensure_all_layers_empty() {
    let fs = setup_test_filesystem().await;
    let llm = mock_llm_client();
    let generator = LayerGenerator::new(fs, llm, default_config());
    
    let stats = generator.ensure_all_layers().await.unwrap();
    
    assert_eq!(stats.total, 0);
    assert_eq!(stats.generated, 0);
    assert_eq!(stats.failed, 0);
}

#[tokio::test]
async fn test_ensure_all_layers_with_missing() {
    let fs = setup_test_filesystem().await;
    fs.mkdir("cortex://user/dir1").await.unwrap();
    fs.mkdir("cortex://user/dir2").await.unwrap();
    
    let llm = mock_llm_client_with_responses(vec![
        ("abstract for dir1", "overview for dir1"),
        ("abstract for dir2", "overview for dir2"),
    ]);
    
    let generator = LayerGenerator::new(fs.clone(), llm, default_config());
    
    let stats = generator.ensure_all_layers().await.unwrap();
    
    assert_eq!(stats.total, 2);
    assert_eq!(stats.generated, 2);
    assert_eq!(stats.failed, 0);
    
    // 验证文件已生成
    assert!(fs.exists("cortex://user/dir1/.abstract").await.unwrap());
    assert!(fs.exists("cortex://user/dir1/.overview").await.unwrap());
    assert!(fs.exists("cortex://user/dir2/.abstract").await.unwrap());
    assert!(fs.exists("cortex://user/dir2/.overview").await.unwrap());
}

#[tokio::test]
async fn test_ensure_all_layers_with_partial_failure() {
    let fs = setup_test_filesystem().await;
    fs.mkdir("cortex://user/dir1").await.unwrap();
    fs.mkdir("cortex://user/dir2").await.unwrap();
    
    // Mock LLM: dir1 成功，dir2 失败
    let llm = mock_llm_client_with_failure_on_second_call();
    
    let generator = LayerGenerator::new(fs.clone(), llm, default_config());
    
    let stats = generator.ensure_all_layers().await.unwrap();
    
    assert_eq!(stats.total, 2);
    assert_eq!(stats.generated, 1);
    assert_eq!(stats.failed, 1);
}

#[tokio::test]
async fn test_batch_generation_with_delay() {
    let fs = setup_test_filesystem().await;
    
    // 创建 25 个目录，配置 batch_size=10
    for i in 0..25 {
        fs.mkdir(&format!("cortex://user/dir{}", i)).await.unwrap();
    }
    
    let llm = mock_llm_client();
    let config = LayerGenerationConfig {
        batch_size: 10,
        delay_ms: 100,
        ..default_config()
    };
    
    let generator = LayerGenerator::new(fs, llm, config);
    
    let start = Instant::now();
    let stats = generator.ensure_all_layers().await.unwrap();
    let duration = start.elapsed();
    
    assert_eq!(stats.generated, 25);
    
    // 应该有 2 次延迟（3 个批次，2 个间隔）
    assert!(duration.as_millis() >= 200); // 至少 2 * 100ms
}
```

**验收标准**:
- [ ] 覆盖空场景
- [ ] 覆盖正常生成
- [ ] 覆盖部分失败
- [ ] 覆盖批次延迟
- [ ] 测试覆盖率 > 85%

---

### 2.2 .abstract 大小控制测试

#### UT-0.2.1: Prompt 约束测试

```rust
#[tokio::test]
async fn test_generate_abstract_within_limit() {
    let llm = mock_llm_client_with_response("这是一个简洁的摘要。");
    let config = LayerGenerationConfig {
        abstract_config: AbstractConfig {
            max_chars: 2000,
            max_tokens: 400,
            target_sentences: 2,
        },
        ..default_config()
    };
    
    let generator = LayerGenerator::new(mock_filesystem(), llm, config);
    
    let content = "很长的内容...".repeat(100);
    let abstract_text = generator.generate_abstract_v2(&content, "用户偏好").await.unwrap();
    
    assert!(abstract_text.len() <= 2000, "Abstract 超过 2K 字符: {}", abstract_text.len());
}

#[tokio::test]
async fn test_enforce_limits_truncate_at_sentence() {
    let config = LayerGenerationConfig {
        abstract_config: AbstractConfig {
            max_chars: 100,
            ..default_config().abstract_config
        },
        ..default_config()
    };
    
    let generator = LayerGenerator::new(mock_filesystem(), mock_llm_client(), config);
    
    let long_text = "这是第一句话。这是第二句话。这是第三句话，非常长，超过了限制。这是第四句话。";
    let result = generator.enforce_limits(long_text.to_string()).unwrap();
    
    // 应该截断到第二句话
    assert!(result.len() <= 100);
    assert!(result.ends_with("。") || result.ends_with("."));
    assert!(!result.contains("第三句话"));
}

#[tokio::test]
async fn test_enforce_limits_with_ellipsis() {
    let config = LayerGenerationConfig {
        abstract_config: AbstractConfig {
            max_chars: 50,
            ..default_config().abstract_config
        },
        ..default_config()
    };
    
    let generator = LayerGenerator::new(mock_filesystem(), mock_llm_client(), config);
    
    let long_text = "这是一段很长的文本没有句号所以无法在句子边界截断";
    let result = generator.enforce_limits(long_text.to_string()).unwrap();
    
    assert!(result.len() <= 50);
    assert!(result.ends_with("..."));
}
```

**验收标准**:
- [ ] 100% 的生成 abstract < 2K
- [ ] 正确截断到句子边界
- [ ] 无句号时添加省略号
- [ ] 测试覆盖率 > 90%

---

#### IT-0.2.1: 现有文件重新生成测试

```rust
#[tokio::test]
async fn test_regenerate_oversized_abstracts() {
    // Setup: 创建一些超大和正常的 .abstract 文件
    let fs = setup_test_filesystem().await;
    
    fs.mkdir("cortex://user/dir1").await.unwrap();
    fs.write("cortex://user/dir1/.abstract", &"X".repeat(5000)).await.unwrap(); // 超大
    
    fs.mkdir("cortex://user/dir2").await.unwrap();
    fs.write("cortex://user/dir2/.abstract", "正常大小的摘要。").await.unwrap(); // 正常
    
    fs.mkdir("cortex://user/dir3").await.unwrap();
    fs.write("cortex://user/dir3/.abstract", &"Y".repeat(3000)).await.unwrap(); // 超大
    
    // Execute
    let llm = mock_llm_client_with_response("新的简洁摘要。");
    let generator = LayerGenerator::new(fs.clone(), llm, default_config());
    
    let stats = generator.regenerate_oversized_abstracts().await.unwrap();
    
    // Verify
    assert_eq!(stats.regenerated, 2); // dir1 和 dir3
    
    let new_abstract1 = fs.read("cortex://user/dir1/.abstract").await.unwrap();
    assert!(new_abstract1.len() <= 2000);
    
    let unchanged = fs.read("cortex://user/dir2/.abstract").await.unwrap();
    assert_eq!(unchanged, "正常大小的摘要。");
}
```

**验收标准**:
- [ ] 仅重新生成超大文件
- [ ] 正常文件不受影响
- [ ] 统计信息准确

---

### 2.3 性能优化测试

#### UT-0.3.1: 并发读取测试

```rust
#[tokio::test]
async fn test_read_all_layers_concurrent() {
    let fs = setup_test_filesystem().await;
    
    // Setup: 创建 10 个目录，每个有 L0/L1/L2
    for i in 0..10 {
        let uri = format!("cortex://user/dir{}", i);
        fs.mkdir(&uri).await.unwrap();
        fs.write(&format!("{}/.abstract", uri), &format!("Abstract {}", i)).await.unwrap();
        fs.write(&format!("{}/.overview", uri), &format!("Overview {}", i)).await.unwrap();
        fs.write(&format!("{}/content.md", uri), &format!("Content {}", i)).await.unwrap();
    }
    
    let reader = LayerReader::new(fs);
    let uris: Vec<String> = (0..10).map(|i| format!("cortex://user/dir{}/content.md", i)).collect();
    
    let start = Instant::now();
    let results = reader.read_all_layers_concurrent(&uris).await.unwrap();
    let duration = start.elapsed();
    
    // Verify
    assert_eq!(results.len(), 10);
    
    for i in 0..10 {
        let uri = format!("cortex://user/dir{}/content.md", i);
        let bundle = results.get(&uri).unwrap();
        assert_eq!(bundle.abstract_text.as_ref().unwrap(), &format!("Abstract {}", i));
        assert_eq!(bundle.overview.as_ref().unwrap(), &format!("Overview {}", i));
        assert_eq!(bundle.content.as_ref().unwrap(), &format!("Content {}", i));
    }
    
    // 并发应该比串行快（粗略检查）
    println!("并发读取 10 个文件耗时: {:?}", duration);
    assert!(duration.as_millis() < 1000); // 应该很快
}
```

**验收标准**:
- [ ] 正确并发读取所有层级
- [ ] 性能提升明显（至少 30%）
- [ ] 无数据竞争

---

#### UT-0.3.2: Embedding 缓存测试

```rust
#[tokio::test]
async fn test_embedding_cache_hit() {
    let inner = mock_embedding_client();
    let cached = CachedEmbeddingClient::new(Arc::new(inner), 100);
    
    let text = "测试文本";
    
    // 第一次调用：缓存未命中
    let start = Instant::now();
    let vector1 = cached.embed(text).await.unwrap();
    let first_duration = start.elapsed();
    
    // 第二次调用：缓存命中
    let start = Instant::now();
    let vector2 = cached.embed(text).await.unwrap();
    let second_duration = start.elapsed();
    
    // Verify
    assert_eq!(vector1, vector2);
    assert!(second_duration < first_duration / 10); // 缓存命中应该快 10 倍以上
    println!("第一次: {:?}, 第二次: {:?}", first_duration, second_duration);
}

#[tokio::test]
async fn test_embedding_cache_eviction() {
    let inner = mock_embedding_client();
    let cached = CachedEmbeddingClient::new(Arc::new(inner), 2); // 容量只有 2
    
    // 添加 3 个条目
    cached.embed("text1").await.unwrap();
    cached.embed("text2").await.unwrap();
    cached.embed("text3").await.unwrap(); // 应该驱逐 text1
    
    // 再次访问 text1：缓存未命中
    let start = Instant::now();
    cached.embed("text1").await.unwrap();
    let duration = start.elapsed();
    
    // 应该重新生成，耗时较长
    assert!(duration.as_millis() > 10);
}
```

**验收标准**:
- [ ] 缓存命中显著加速（10x+）
- [ ] LRU 驱逐正确
- [ ] 并发安全

---

#### BT-0.3.1: 批量 Embedding 性能测试

**测试文件**: `cortex-mem-core/benches/embedding_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_embedding_single_vs_batch(c: &mut Criterion) {
    let client = setup_real_embedding_client();
    let texts: Vec<String> = (0..10).map(|i| format!("测试文本 {}", i)).collect();
    
    c.bench_function("embedding_single", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut vectors = vec![];
                for text in &texts {
                    let v = client.embed(text).await.unwrap();
                    vectors.push(v);
                }
                black_box(vectors);
            });
    });
    
    c.bench_function("embedding_batch", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let vectors = client.embed_batch(&texts).await.unwrap();
                black_box(vectors);
            });
    });
}

criterion_group!(benches, bench_embedding_single_vs_batch);
criterion_main!(benches);
```

**验收标准**:
- [ ] 批量比单次快 5x+
- [ ] 基准报告生成

---

## 三、阶段 1: 检索引擎升级测试

### 3.1 目录递归检索测试

#### UT-1.1.1: 全局搜索测试

```rust
#[tokio::test]
async fn test_global_search() {
    let vector_store = mock_vector_store_with_results(vec![
        ("cortex://user/memories", 0.9, false),
        ("cortex://resources/docs", 0.85, false),
        ("cortex://agent/cases", 0.8, false),
        ("cortex://user/memories/preferences/code_style.md", 0.75, true), // 叶子，不应返回
    ]);
    
    let retriever = HierarchicalRetriever::new(
        vector_store,
        mock_embedding_client(),
        mock_filesystem(),
        default_config(),
    );
    
    let query = TypedQuery {
        query: "代码风格偏好".to_string(),
        context_type: ContextType::Memory,
        target_scope: None,
        limit: 10,
    };
    
    let top_dirs = retriever.global_search(&query, 3).await.unwrap();
    
    // Verify
    assert_eq!(top_dirs.len(), 3);
    assert_eq!(top_dirs[0].uri, "cortex://user/memories");
    assert_eq!(top_dirs[0].score, 0.9);
    
    // 不应包含叶子节点
    assert!(!top_dirs.iter().any(|d| d.uri.contains("code_style.md")));
}
```

**验收标准**:
- [ ] 正确过滤叶子节点
- [ ] 按分数排序
- [ ] 限制返回数量

---

#### UT-1.1.2: 递归搜索测试

```rust
#[tokio::test]
async fn test_recursive_search() {
    let fs = setup_test_filesystem().await;
    
    // Setup 目录结构:
    // cortex://user/memories/
    //   ├── preferences/
    //   │   ├── code_style.md (叶子)
    //   │   └── communication.md (叶子)
    //   └── entities/
    //       └── project_x.md (叶子)
    
    fs.mkdir("cortex://user/memories/preferences").await.unwrap();
    fs.write("cortex://user/memories/preferences/code_style.md", "内容").await.unwrap();
    fs.write("cortex://user/memories/preferences/communication.md", "内容").await.unwrap();
    fs.mkdir("cortex://user/memories/entities").await.unwrap();
    fs.write("cortex://user/memories/entities/project_x.md", "内容").await.unwrap();
    
    let vector_store = mock_vector_store_with_hierarchical_results();
    let retriever = HierarchicalRetriever::new(
        vector_store,
        mock_embedding_client(),
        fs,
        default_config(),
    );
    
    let start_dir = DirectoryScore {
        uri: "cortex://user/memories".to_string(),
        score: 0.9,
        depth: 1,
    };
    
    let query = TypedQuery {
        query: "代码风格".to_string(),
        context_type: ContextType::Memory,
        target_scope: None,
        limit: 10,
    };
    
    let candidates = retriever.recursive_search(&start_dir, &query, 3).await.unwrap();
    
    // Verify
    assert!(candidates.len() > 0);
    
    // 应该包含 code_style.md
    assert!(candidates.iter().any(|c| c.uri.contains("code_style.md")));
    
    // 每个候选都应该有 final_score（应用了分数传播）
    for candidate in &candidates {
        assert!(candidate.final_score > 0.0);
        assert!(candidate.final_score <= 1.0);
    }
}
```

**验收标准**:
- [ ] 正确递归探索子目录
- [ ] 应用分数传播
- [ ] 限制最大深度
- [ ] 测试覆盖率 > 85%

---

#### UT-1.1.3: 分数传播测试

```rust
#[test]
fn test_score_propagation() {
    let config = HierarchicalConfig {
        score_propagation_alpha: 0.5,
        ..default_config()
    };
    
    let retriever = HierarchicalRetriever::new(
        mock_vector_store(),
        mock_embedding_client(),
        mock_filesystem(),
        config,
    );
    
    let candidates = vec![
        Candidate {
            uri: "cortex://user/memories/preferences/code_style.md".to_string(),
            score: 0.8,
            final_score: 0.0, // 待计算
            parent_score: 0.9,
            depth: 2,
        },
    ];
    
    let results = retriever.apply_score_propagation_and_sort(candidates, 10);
    
    // Verify: final_score = 0.5 * 0.8 + 0.5 * 0.9 = 0.85
    assert_eq!(results[0].score, 0.85);
}

#[test]
fn test_score_propagation_alpha_0() {
    let config = HierarchicalConfig {
        score_propagation_alpha: 0.0, // 完全依赖父节点
        ..default_config()
    };
    
    let retriever = HierarchicalRetriever::new(
        mock_vector_store(),
        mock_embedding_client(),
        mock_filesystem(),
        config,
    );
    
    let candidates = vec![
        Candidate {
            uri: "test".to_string(),
            score: 0.6,
            final_score: 0.0,
            parent_score: 0.9,
            depth: 2,
        },
    ];
    
    let results = retriever.apply_score_propagation_and_sort(candidates, 10);
    
    // final_score = 0.0 * 0.6 + 1.0 * 0.9 = 0.9
    assert_eq!(results[0].score, 0.9);
}

#[test]
fn test_score_propagation_alpha_1() {
    let config = HierarchicalConfig {
        score_propagation_alpha: 1.0, // 完全依赖当前分数
        ..default_config()
    };
    
    let retriever = HierarchicalRetriever::new(
        mock_vector_store(),
        mock_embedding_client(),
        mock_filesystem(),
        config,
    );
    
    let candidates = vec![
        Candidate {
            uri: "test".to_string(),
            score: 0.6,
            final_score: 0.0,
            parent_score: 0.9,
            depth: 2,
        },
    ];
    
    let results = retriever.apply_score_propagation_and_sort(candidates, 10);
    
    // final_score = 1.0 * 0.6 + 0.0 * 0.9 = 0.6
    assert_eq!(results[0].score, 0.6);
}
```

**验收标准**:
- [ ] 正确应用公式
- [ ] 边界值测试（alpha=0, alpha=1）
- [ ] 测试覆盖率 > 95%

---

### 3.2 意图分析测试

#### UT-1.2.1: 意图分析器测试

```rust
#[tokio::test]
async fn test_analyze_simple_query() {
    let llm = mock_llm_client_with_json_response(r#"[
        {
            "query": "代码风格偏好",
            "context_type": "memory",
            "target_scope": "user/preferences"
        }
    ]"#);
    
    let analyzer = LightweightIntentAnalyzer::new(llm, default_config());
    
    let queries = analyzer.analyze("我的代码风格是什么?", None).await.unwrap();
    
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].query, "代码风格偏好");
    assert_eq!(queries[0].context_type, ContextType::Memory);
    assert_eq!(queries[0].target_scope.as_ref().unwrap(), "user/preferences");
}

#[tokio::test]
async fn test_analyze_with_context() {
    let llm = mock_llm_client_with_json_response(r#"[
        {
            "query": "项目 X 进展",
            "context_type": "memory",
            "target_scope": "user/entities"
        },
        {
            "query": "项目相关讨论",
            "context_type": "session",
            "target_scope": null
        }
    ]"#);
    
    let analyzer = LightweightIntentAnalyzer::new(llm, default_config());
    
    let recent_context = "上次我们讨论了项目 X 的架构设计...";
    let queries = analyzer.analyze("项目现在怎么样了?", Some(recent_context)).await.unwrap();
    
    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].context_type, ContextType::Memory);
    assert_eq!(queries[1].context_type, ContextType::Session);
}

#[tokio::test]
async fn test_analyze_max_queries_limit() {
    let llm = mock_llm_client_with_json_response(r#"[
        {"query": "q1", "context_type": "memory"},
        {"query": "q2", "context_type": "resource"},
        {"query": "q3", "context_type": "agent"},
        {"query": "q4", "context_type": "session"}
    ]"#);
    
    let config = IntentAnalyzerConfig {
        max_queries: 2,
        ..default_config()
    };
    
    let analyzer = LightweightIntentAnalyzer::new(llm, config);
    
    let queries = analyzer.analyze("复杂查询", None).await.unwrap();
    
    // 应该被限制为 2 个
    assert_eq!(queries.len(), 2);
}

#[tokio::test]
async fn test_analyze_disabled() {
    let llm = mock_llm_client();
    let config = IntentAnalyzerConfig {
        enabled: false,
        ..default_config()
    };
    
    let analyzer = LightweightIntentAnalyzer::new(llm, config);
    
    let queries = analyzer.analyze("任意查询", None).await.unwrap();
    
    // 禁用时应返回单一默认查询
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].query, "任意查询");
}
```

**验收标准**:
- [ ] 正确解析 JSON 响应
- [ ] 应用数量限制
- [ ] 支持禁用开关
- [ ] 测试覆盖率 > 85%

---

### 3.3 集成测试

#### IT-1.1: 端到端检索测试

```rust
#[tokio::test]
async fn test_end_to_end_hierarchical_search() {
    // Setup: 真实的 Qdrant + 文件系统
    let (fs, vector_store) = setup_integration_environment().await;
    
    // 准备测试数据
    populate_test_data(&fs, &vector_store).await;
    
    // 创建检索引擎
    let engine = VectorSearchEngine::new(
        vector_store,
        real_embedding_client(),
        fs,
        default_config(),
    );
    
    // Execute
    let results = engine.search_with_intent(
        "Rust 代码风格偏好",
        None,
        &SearchOptions::default(),
    ).await.unwrap();
    
    // Verify
    assert!(results.len() > 0);
    assert!(results[0].uri.contains("preferences") || results[0].uri.contains("code"));
    assert!(results[0].score > 0.5);
}
```

**验收标准**:
- [ ] 真实环境测试通过
- [ ] 正确集成所有组件
- [ ] 性能符合预期

---

## 四、阶段 2: 记忆管理测试

### 4.1 记忆分类扩展测试

#### UT-2.1.1: Profile 提取测试

```rust
#[tokio::test]
async fn test_extract_profile_with_info() {
    let llm = mock_llm_client_with_response(r#"# 用户画像

## 基本信息
- 职业: 软件工程师
- 技术栈: Rust, Python
- 兴趣: AI, 开源

## 工作习惯
- 偏好简洁高效的工具
- 重视代码质量和性能"#);
    
    let extractor = MemoryExtractor::new(llm, mock_filesystem(), default_config());
    
    let messages = vec![
        Message::user("我是一名软件工程师，主要使用 Rust 和 Python"),
        Message::assistant("了解，请问您的工作习惯是怎样的?"),
        Message::user("我偏好简洁高效的工具，重视代码质量和性能"),
    ];
    
    let profile = extractor.extract_profile(&messages).await.unwrap();
    
    assert!(profile.is_some());
    let profile = profile.unwrap();
    assert_eq!(profile.category, MemoryCategory::Profile);
    assert!(profile.content.contains("软件工程师"));
    assert!(profile.content.contains("Rust"));
}

#[tokio::test]
async fn test_extract_profile_no_info() {
    let llm = mock_llm_client_with_response("null");
    
    let extractor = MemoryExtractor::new(llm, mock_filesystem(), default_config());
    
    let messages = vec![
        Message::user("今天天气怎么样?"),
        Message::assistant("今天天气很好"),
    ];
    
    let profile = extractor.extract_profile(&messages).await.unwrap();
    
    assert!(profile.is_none());
}

#[tokio::test]
async fn test_merge_profile() {
    let llm = mock_llm_client_with_response(r#"# 用户画像

## 基本信息
- 职业: 软件工程师
- 技术栈: Rust, Python, Go
- 兴趣: AI, 开源, 区块链

## 工作习惯
- 偏好简洁高效的工具
- 重视代码质量和性能
- 喜欢 TDD 开发模式"#);
    
    let extractor = MemoryExtractor::new(llm, mock_filesystem(), default_config());
    
    let existing = "# 用户画像\n\n## 基本信息\n- 职业: 软件工程师\n- 技术栈: Rust, Python";
    let new_info = "## 基本信息\n- 技术栈: Go\n- 兴趣: 区块链\n\n## 工作习惯\n- 喜欢 TDD 开发模式";
    
    let merged = extractor.merge_profile(existing, new_info).await.unwrap();
    
    assert!(merged.contains("Rust"));
    assert!(merged.contains("Python"));
    assert!(merged.contains("Go"));
    assert!(merged.contains("区块链"));
    assert!(merged.contains("TDD"));
}
```

**验收标准**:
- [ ] 正确提取 Profile
- [ ] 无信息时返回 None
- [ ] 正确合并 Profile
- [ ] 测试覆盖率 > 85%

---

#### UT-2.1.2: Pattern 提取测试

```rust
#[tokio::test]
async fn test_extract_patterns() {
    let llm = mock_llm_client_with_json_response(r#"[
        {
            "name": "调试性能问题的流程",
            "applicability": "应用响应慢、CPU/内存占用高",
            "steps": [
                "使用 perf 分析 CPU 热点",
                "检查 allocator 性能",
                "添加 tracing 日志",
                "对比优化前后基准测试"
            ],
            "examples": [
                "案例1: 优化 Rust 应用 CPU 占用",
                "案例2: 解决内存泄漏问题"
            ]
        }
    ]"#);
    
    let extractor = MemoryExtractor::new(llm, mock_filesystem(), default_config());
    
    let messages = vec![
        Message::user("我的应用很慢，怎么调试?"),
        Message::assistant("可以先用 perf 分析 CPU 热点..."),
        // ... 更多对话
    ];
    
    let patterns = extractor.extract_patterns(&messages).await.unwrap();
    
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].category, MemoryCategory::Pattern);
    assert!(patterns[0].content.contains("调试性能问题"));
    assert!(patterns[0].content.contains("perf"));
}
```

**验收标准**:
- [ ] 正确提取 Pattern
- [ ] Markdown 格式正确
- [ ] 测试覆盖率 > 80%

---

### 4.2 记忆去重测试

#### UT-2.2.1: 去重检测测试

```rust
#[tokio::test]
async fn test_check_duplicate_no_similar() {
    let vector_store = mock_vector_store_with_empty_results();
    let deduplicator = MemoryDeduplicator::new(
        vector_store,
        mock_embedding_client(),
        mock_llm_client(),
        default_config(),
    );
    
    let candidate = CandidateMemory {
        category: MemoryCategory::Preference,
        abstract_text: "代码风格偏好".to_string(),
        overview: "简洁高效".to_string(),
        content: "偏好简洁高效的代码...".to_string(),
    };
    
    let result = deduplicator.check_duplicate(&candidate).await.unwrap();
    
    assert!(matches!(result, DeduplicationResult::NoDuplicate));
}

#[tokio::test]
async fn test_check_duplicate_found() {
    let vector_store = mock_vector_store_with_results(vec![
        ("cortex://user/preferences/code_style.md", 0.9, true),
    ]);
    
    let llm = mock_llm_client_with_json_response(r#"{
        "is_duplicate": true,
        "reason": "内容实质相同"
    }"#);
    
    let deduplicator = MemoryDeduplicator::new(
        vector_store,
        mock_embedding_client(),
        llm,
        default_config(),
    );
    
    let candidate = CandidateMemory {
        category: MemoryCategory::Preference,
        abstract_text: "代码风格偏好".to_string(),
        overview: "简洁高效".to_string(),
        content: "偏好简洁高效的代码...".to_string(),
    };
    
    let result = deduplicator.check_duplicate(&candidate).await.unwrap();
    
    match result {
        DeduplicationResult::Duplicate { existing_uri } => {
            assert_eq!(existing_uri, "cortex://user/preferences/code_style.md");
        }
        _ => panic!("Expected Duplicate"),
    }
}

#[tokio::test]
async fn test_check_duplicate_llm_disabled() {
    let vector_store = mock_vector_store_with_results(vec![
        ("cortex://user/preferences/code_style.md", 0.9, true),
    ]);
    
    let config = DeduplicatorConfig {
        enable_llm_check: false,
        ..default_config()
    };
    
    let deduplicator = MemoryDeduplicator::new(
        vector_store,
        mock_embedding_client(),
        mock_llm_client(),
        config,
    );
    
    let candidate = CandidateMemory {
        category: MemoryCategory::Preference,
        abstract_text: "代码风格偏好".to_string(),
        overview: "简洁高效".to_string(),
        content: "偏好简洁高效的代码...".to_string(),
    };
    
    let result = deduplicator.check_duplicate(&candidate).await.unwrap();
    
    // LLM 禁用时，仅依赖向量相似度（无法确定是否真的重复）
    assert!(matches!(result, DeduplicationResult::NoDuplicate));
}
```

**验收标准**:
- [ ] 正确检测无重复
- [ ] 正确检测有重复
- [ ] LLM 开关生效
- [ ] 测试覆盖率 > 90%

---

#### UT-2.2.2: 记忆合并测试

```rust
#[tokio::test]
async fn test_merge_memory() {
    let fs = setup_test_filesystem().await;
    fs.write("cortex://user/preferences/code_style.md", "现有内容：偏好简洁代码").await.unwrap();
    
    let llm = mock_llm_client_with_json_response(r#"{
        "abstract": "代码风格偏好：简洁、高效、可读",
        "overview": "用户偏好简洁高效的代码，重视可读性",
        "content": "# 代码风格偏好\n\n用户偏好简洁高效的代码，重视可读性和性能。"
    }"#);
    
    let deduplicator = MemoryDeduplicator::new(
        mock_vector_store(),
        mock_embedding_client(),
        llm,
        fs.clone(),
        default_config(),
    );
    
    let new_content = "新增内容：重视可读性";
    
    let merged = deduplicator.merge_memory(
        "cortex://user/preferences/code_style.md",
        new_content,
        &MemoryCategory::Preference,
    ).await.unwrap();
    
    // Verify
    assert!(merged.content.contains("简洁"));
    assert!(merged.content.contains("可读性"));
    
    // 验证文件已更新
    let updated = fs.read("cortex://user/preferences/code_style.md").await.unwrap();
    assert!(updated.contains("简洁"));
    assert!(updated.contains("可读性"));
}
```

**验收标准**:
- [ ] 正确合并内容
- [ ] 文件更新成功
- [ ] 测试覆盖率 > 85%

---

## 五、性能基准测试

### 5.1 LOMOCO 基准测试

**测试文件**: `examples/lomoco-evaluation/run_benchmark.sh`

```bash
#!/bin/bash

# Cortex-Memory 3.0 LOMOCO 基准测试

echo "准备测试环境..."
cargo build --release

echo "运行 LOMOCO 评估..."
cargo run --release --example lomoco_evaluation -- \
    --data-path ./examples/lomoco-evaluation/data \
    --output-path ./benchmark_results/3.0_$(date +%Y%m%d_%H%M%S).json

echo "生成报告..."
python3 ./examples/lomoco-evaluation/generate_report.py \
    --input ./benchmark_results/3.0_*.json \
    --baseline ./benchmark_results/2.x_baseline.json \
    --output ./benchmark_results/3.0_report.md
```

**验收标准**:
- [ ] Recall@1 > 95%
- [ ] MRR > 95%
- [ ] NDCG@5 > 85%
- [ ] 对比 2.x 有提升

---

### 5.2 查询延迟基准测试

**测试文件**: `cortex-mem-core/benches/search_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_search_latency(c: &mut Criterion) {
    let engine = setup_real_search_engine();
    
    let queries = vec![
        "Rust 代码风格",
        "项目 X 进展",
        "性能优化方法",
    ];
    
    let mut group = c.benchmark_group("search_latency");
    
    for query in queries {
        group.bench_with_input(BenchmarkId::new("hierarchical", query), query, |b, q| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let results = engine.search_with_intent(q, None, &SearchOptions::default()).await.unwrap();
                    black_box(results);
                });
        });
    }
    
    group.finish();
}

fn bench_search_throughput(c: &mut Criterion) {
    let engine = setup_real_search_engine();
    
    c.bench_function("search_throughput_100", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let tasks: Vec<_> = (0..100).map(|i| {
                    engine.search_with_intent(&format!("query {}", i), None, &SearchOptions::default())
                }).collect();
                
                let results = futures::future::try_join_all(tasks).await.unwrap();
                black_box(results);
            });
    });
}

criterion_group!(benches, bench_search_latency, bench_search_throughput);
criterion_main!(benches);
```

**验收标准**:
- [ ] P50 < 50ms
- [ ] P95 < 100ms
- [ ] P99 < 200ms
- [ ] 吞吐量 > 100 QPS

---

## 六、测试工具与辅助函数

### 6.1 Mock 工具

```rust
// cortex-mem-core/src/test_utils/mod.rs

pub fn mock_filesystem() -> Arc<CortexFilesystem> {
    // 使用内存文件系统
    Arc::new(CortexFilesystem::new_in_memory())
}

pub fn mock_llm_client() -> Arc<dyn LLMClient> {
    Arc::new(MockLLMClient::new())
}

pub fn mock_llm_client_with_response(response: &str) -> Arc<dyn LLMClient> {
    let mut client = MockLLMClient::new();
    client.expect_generate()
        .returning(move |_| Ok(response.to_string()));
    Arc::new(client)
}

pub fn mock_embedding_client() -> Arc<dyn EmbeddingClient> {
    let mut client = MockEmbeddingClient::new();
    client.expect_embed()
        .returning(|_| Ok(vec![0.1; 1536])); // 固定向量
    Arc::new(client)
}

pub fn mock_vector_store() -> Arc<dyn VectorStore> {
    Arc::new(MockVectorStore::new())
}

pub fn mock_vector_store_with_results(results: Vec<(&str, f32, bool)>) -> Arc<dyn VectorStore> {
    let mut store = MockVectorStore::new();
    store.expect_search()
        .returning(move |_, _| {
            Ok(results.iter().map(|(uri, score, is_leaf)| SearchResult {
                uri: uri.to_string(),
                score: *score,
                is_leaf: *is_leaf,
                // ... 其他字段
            }).collect())
        });
    Arc::new(store)
}

pub fn default_config() -> LayerGenerationConfig {
    LayerGenerationConfig::default()
}
```

---

## 七、持续集成配置

### 7.1 GitHub Actions 配置

**文件**: `.github/workflows/test.yml`

```yaml
name: Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Run unit tests
        run: cargo test --all-features
      
      - name: Run integration tests
        run: cargo test --test '*' --all-features
      
      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --all-features
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  benchmark:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Run benchmarks
        run: cargo bench --bench search_bench
      
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/output.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
```

---

## 八、总结

### 测试覆盖概览

| 模块 | 单元测试 | 集成测试 | 基准测试 |
|------|---------|---------|---------|
| 层级生成 | 15 个 | 2 个 | - |
| 递归检索 | 12 个 | 3 个 | 3 个 |
| 意图分析 | 8 个 | 2 个 | 1 个 |
| 记忆去重 | 10 个 | 2 个 | - |
| 性能优化 | 8 个 | - | 4 个 |
| **总计** | **53 个** | **9 个** | **8 个** |

### 验收标准总览

- [ ] 单元测试覆盖率 > 80%
- [ ] 所有集成测试通过
- [ ] LOMOCO 基准: Recall@1 > 95%
- [ ] 查询延迟: P95 < 100ms
- [ ] 无回归问题

**测试用例设计完成，准备实施！✅**
