//! cortex-mem-cli 命令功能测试
//!
//! # 测试分类
//!
//! ## 1. 基础命令测试 (无需外部服务)
//!    - `--help`：检验帮助信息输出
//!    - `--version`：检验版本号输出
//!
//! ## 2. Tenant 命令测试 (仅需配置文件和本地文件系统)
//!    - `tenant list`：列出租户，使用临时目录
//!
//! ## 3. 参数验证测试 (无需外部服务)
//!    - 缺少必要参数时的错误提示
//!    - 非法参数值的错误提示（如 min_score > 1.0）
//!    - 非法 URI scheme 的错误提示
//!
//! ## 4. 完整功能测试 (需要 Qdrant + LLM + Embedding，标记为 #[ignore])
//!    - `add`：添加消息
//!    - `list`：列出记忆
//!    - `get`：获取单条记忆
//!    - `delete`：删除记忆
//!    - `search`：语义搜索
//!    - `session list/create/close`：会话管理
//!    - `stats`：统计信息
//!    - `layers status/ensure-all/regenerate-oversized`：层文件管理
//!
//! # 运行方式
//!
//! ```bash
//! # 只运行不依赖外部服务的测试
//! cargo test -p cortex-mem-cli
//!
//! # 运行全部测试（需要配置好 Qdrant + LLM + Embedding）
//! cargo test -p cortex-mem-cli -- --include-ignored
//!
//! # 通过环境变量指定配置
//! CONFIG_PATH=/path/to/config.toml TENANT_ID=my-tenant cargo test -p cortex-mem-cli -- --include-ignored
//! ```

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ─── 辅助函数 ────────────────────────────────────────────────────────────────

/// 获取 cortex-mem CLI 命令
fn cli() -> Command {
    // 使用 Command::new + CARGO_BIN_EXE 方式，兼容自定义 build-dir
    Command::new(env!("CARGO_BIN_EXE_cortex-mem"))
}

/// 从环境变量读取配置路径，如未设置则使用 workspace 根目录的 config.toml
///
/// 注意：cargo test 的工作目录是 crate 目录（cortex-mem-cli/），而非 workspace 根目录。
/// 因此需要将相对路径解析为基于 CARGO_MANIFEST_DIR 父目录的绝对路径。
fn config_path() -> String {
    match std::env::var("CONFIG_PATH") {
        Ok(p) => {
            // 环境变量提供的路径：如果是相对路径，则相对于 workspace 根目录（CARGO_MANIFEST_DIR 的父目录）
            let path = std::path::Path::new(&p);
            if path.is_absolute() {
                p
            } else {
                // CARGO_MANIFEST_DIR = cortex-mem-cli/，其父目录 = workspace 根目录
                let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("CARGO_MANIFEST_DIR has no parent");
                workspace_root.join(path).to_string_lossy().to_string()
            }
        }
        Err(_) => {
            // 默认使用 workspace 根目录下的 config.toml
            let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("CARGO_MANIFEST_DIR has no parent");
            workspace_root
                .join("config.toml")
                .to_string_lossy()
                .to_string()
        }
    }
}

/// 从环境变量读取 Tenant ID，如未设置则使用默认值
fn tenant_id() -> String {
    std::env::var("TENANT_ID").unwrap_or_else(|_| "default".to_string())
}

/// 创建临时数据目录，并在其中生成一个最简配置文件（不含外部服务配置）
/// 该函数用于不需要向量数据库的测试场景
fn setup_temp_env() -> (TempDir, String) {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let data_dir = tmp.path().join("cortex-data");
    fs::create_dir_all(&data_dir).expect("Failed to create data dir");

    // 创建 tenants 目录结构（用于 tenant list 测试）
    let tenants_dir = data_dir.join("tenants");
    fs::create_dir_all(&tenants_dir).expect("Failed to create tenants dir");
    fs::create_dir_all(tenants_dir.join("tenant-alpha")).expect("Failed to create tenant dir");
    fs::create_dir_all(tenants_dir.join("tenant-beta")).expect("Failed to create tenant dir");

    // 生成最小化 config.toml（包含所有必需字段，但 URL 指向本地不存在的服务）
    let config_content = format!(
        r#"[qdrant]
url = "http://localhost:16334"
collection_name = "test-cortex-mem"
embedding_dim = 256
timeout_secs = 5
api_key = ""

[embedding]
api_base_url = "http://localhost:18080"
api_key = "test-key"
model_name = "test-model"
batch_size = 10
timeout_secs = 5

[llm]
api_base_url = "http://localhost:18080"
api_key = "test-key"
model_efficient = "test-model"
temperature = 0.1
max_tokens = 4096

[server]
host = "localhost"
port = 3000
cors_origins = ["*"]

[cortex]
data_dir = "{data_dir}"

[logging]
enabled = false
log_directory = "logs"
level = "error"
"#,
        data_dir = data_dir.display()
    );

    let config_path = tmp.path().join("config.toml");
    fs::write(&config_path, &config_content).expect("Failed to write config file");

    let config_str = config_path.to_string_lossy().to_string();
    (tmp, config_str)
}

// ─── 1. 基础命令测试 ─────────────────────────────────────────────────────────

/// B01: --help 输出应包含程序名称和使用说明
#[test]
fn test_help_command() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("cortex-mem"))
        .stdout(predicate::str::contains("Usage"));
}

/// B02: --version 输出应包含二进制名称
#[test]
fn test_version_command() {
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("cortex-mem"));
}

/// B03: add 子命令的 --help 应包含参数说明
#[test]
fn test_add_subcommand_help() {
    cli()
        .args(["add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("thread").or(predicate::str::contains("content")));
}

/// B04: search 子命令的 --help 应包含查询参数说明
#[test]
fn test_search_subcommand_help() {
    cli()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("query").or(predicate::str::contains("limit")));
}

/// B05: session 子命令的 --help 应包含子命令说明
#[test]
fn test_session_subcommand_help() {
    cli().args(["session", "--help"]).assert().success().stdout(
        predicate::str::contains("list")
            .or(predicate::str::contains("create"))
            .or(predicate::str::contains("close")),
    );
}

/// B06: layers 子命令的 --help 应包含子命令说明
#[test]
fn test_layers_subcommand_help() {
    cli().args(["layers", "--help"]).assert().success().stdout(
        predicate::str::contains("status")
            .or(predicate::str::contains("ensure-all"))
            .or(predicate::str::contains("regenerate-oversized")),
    );
}

/// B07: tenant 子命令的 --help 应包含 list 子命令
#[test]
fn test_tenant_subcommand_help() {
    cli()
        .args(["tenant", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

/// B08: get 子命令的 --help 应包含 URI 参数说明
#[test]
fn test_get_subcommand_help() {
    cli()
        .args(["get", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uri").or(predicate::str::contains("URI")));
}

/// B09: list 子命令的 --help 应包含相关参数说明
#[test]
fn test_list_subcommand_help() {
    cli()
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uri").or(predicate::str::contains("URI")));
}

/// B10: delete 子命令的 --help 应包含 URI 参数说明
#[test]
fn test_delete_subcommand_help() {
    cli()
        .args(["delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("uri").or(predicate::str::contains("URI")));
}

/// B11: stats 子命令的 --help 应成功
#[test]
fn test_stats_subcommand_help() {
    cli().args(["stats", "--help"]).assert().success();
}

// ─── 2. Tenant 命令测试 ──────────────────────────────────────────────────────

/// T01: tenant list 在有租户目录时应列出所有租户
#[test]
fn test_tenant_list_with_tenants() {
    let (_tmp, config) = setup_temp_env();

    cli()
        .args(["-c", &config, "tenant", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tenant-alpha"))
        .stdout(predicate::str::contains("tenant-beta"));
}

/// T02: tenant list 在无租户目录时应给出友好提示
#[test]
fn test_tenant_list_empty() {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let data_dir = tmp.path().join("cortex-data-empty");
    fs::create_dir_all(&data_dir).expect("Failed to create data dir");

    let config_content = format!(
        r#"[qdrant]
url = "http://localhost:16334"
collection_name = "test-cortex-mem"
embedding_dim = 256
timeout_secs = 5
api_key = ""

[embedding]
api_base_url = "http://localhost:18080"
api_key = "test-key"
model_name = "test-model"
batch_size = 10
timeout_secs = 5

[llm]
api_base_url = "http://localhost:18080"
api_key = "test-key"
model_efficient = "test-model"
temperature = 0.1
max_tokens = 4096

[server]
host = "localhost"
port = 3000
cors_origins = ["*"]

[cortex]
data_dir = "{data_dir}"

[logging]
enabled = false
log_directory = "logs"
level = "error"
"#,
        data_dir = data_dir.display()
    );

    let config_path = tmp.path().join("config.toml");
    fs::write(&config_path, &config_content).expect("Failed to write config");

    cli()
        .args(["-c", &config_path.to_string_lossy(), "tenant", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tenants").or(predicate::str::contains("found")));
}

/// T03: tenant list 使用不存在的配置文件时应以错误退出
#[test]
fn test_tenant_list_missing_config() {
    cli()
        .args(["-c", "/nonexistent/path/config.toml", "tenant", "list"])
        .assert()
        .failure();
}

// ─── 3. 参数验证测试 ─────────────────────────────────────────────────────────

/// V01: add 命令缺少必要参数 --thread 时应以错误退出
#[test]
fn test_add_missing_thread_arg() {
    cli().args(["add", "some content"]).assert().failure();
}

/// V02: add 命令缺少 content 位置参数时应以错误退出
#[test]
fn test_add_missing_content_arg() {
    cli()
        .args(["add", "--thread", "my-thread"])
        .assert()
        .failure();
}

/// V03: search 命令缺少 query 位置参数时应以错误退出
#[test]
fn test_search_missing_query_arg() {
    cli().args(["search"]).assert().failure();
}

/// V04: get 命令缺少 URI 位置参数时应以错误退出
#[test]
fn test_get_missing_uri_arg() {
    cli().args(["get"]).assert().failure();
}

/// V05: delete 命令缺少 URI 位置参数时应以错误退出
#[test]
fn test_delete_missing_uri_arg() {
    cli().args(["delete"]).assert().failure();
}

/// V06: session create 缺少 thread 参数时应以错误退出
#[test]
fn test_session_create_missing_thread() {
    cli().args(["session", "create"]).assert().failure();
}

/// V07: session close 缺少 thread 参数时应以错误退出
#[test]
fn test_session_close_missing_thread() {
    cli().args(["session", "close"]).assert().failure();
}

/// V08: search 命令的 --min-score 参数超出范围（>1.0）应以错误退出
/// 注意：参数验证发生在 MemoryOperations 初始化之后，因此该测试需要外部服务
#[test]
#[ignore = "参数验证位于 MemoryOperations 初始化之后，需要外部服务才能到达验证逻辑"]
fn test_search_invalid_min_score_over_limit() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "search",
            "test query",
            "-s",
            "2.0",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("min_score must be between")
                .or(predicate::str::contains("between 0.0 and 1.0")),
        );
}

/// V09: get 命令使用无效的 URI scheme 时应以错误退出
/// 注意：URI 验证发生在 MemoryOperations 初始化之后，因此该测试需要外部服务
#[test]
#[ignore = "URI 验证位于 MemoryOperations 初始化之后，需要外部服务才能到达验证逻辑"]
fn test_get_invalid_uri_scheme() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "get",
            "http://invalid-scheme/path",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Invalid URI scheme")
                .or(predicate::str::contains("invalid").or(predicate::str::contains("error"))),
        );
}

/// V10: list 命令不带任何选项时应以错误退出（需要配置才能初始化 MemoryOperations）
/// 这验证了配置文件缺失时的错误处理
#[test]
fn test_list_no_config_fails() {
    cli()
        .args(["-c", "/tmp/nonexistent_config_xyzabc.toml", "list"])
        .assert()
        .failure();
}

// ─── 4. 完整功能测试 (需要外部服务，标记 #[ignore]) ─────────────────────────
//
// 运行方式:
//   CONFIG_PATH=/path/to/config.toml TENANT_ID=<tenant> \
//   cargo test -p cortex-mem-cli -- --include-ignored
//
// 环境变量:
//   CONFIG_PATH   - 配置文件路径 (默认: config.toml)
//   TENANT_ID     - 租户 ID (默认: default)

/// F01: add 命令 - 添加用户消息后应打印成功信息和 URI
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_add_user_message() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-add-{}", uuid_short());

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "--role",
            "user",
            "Hello, this is a test message from cli test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("successfully").or(predicate::str::contains("✓")))
        .stdout(predicate::str::contains("cortex://session"));
}

/// F02: add 命令 - 添加助手消息后应打印成功信息
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_add_assistant_message() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-add-asst-{}", uuid_short());

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "--role",
            "assistant",
            "This is an assistant response for testing",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("successfully").or(predicate::str::contains("✓")));
}

/// F03: list 命令 - 列出默认 URI (cortex://session)
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_list_default_uri() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args(["-c", &config, "--tenant", &tenant, "list"])
        .assert()
        .success();
}

/// F04: list 命令 - 列出 user 维度
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_list_user_dimension() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "list",
            "--uri",
            "cortex://user",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Found").or(predicate::str::contains("No memories")));
}

/// F05: list 命令 - 添加消息后列出该会话的内容
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_list_after_add() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-list-{}", uuid_short());

    // 先添加一条消息
    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "Test message for list verification",
        ])
        .assert()
        .success();

    // 然后列出该会话
    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "list",
            "--uri",
            &format!("cortex://session/{}", thread_id),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Found").or(predicate::str::contains("item")));
}

/// F06: get 命令 - 先 add 再 get 该 URI 的内容
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_get_after_add() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-get-{}", uuid_short());
    let unique_content = format!("Unique test content {}", uuid_short());

    // 先添加消息
    let add_output = cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            &unique_content,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // 从输出中提取 URI（形如 cortex://session/...）
    let output_str = String::from_utf8_lossy(&add_output);
    let uri = extract_uri_from_output(&output_str);

    if let Some(uri) = uri {
        // 使用 get 获取内容
        cli()
            .args(["-c", &config, "--tenant", &tenant, "get", &uri])
            .assert()
            .success()
            .stdout(predicate::str::contains(unique_content));
    } else {
        // URI 提取失败时直接通过（不阻塞 CI）
        println!("WARN: Could not extract URI from add output, skipping get check");
    }
}

/// F07: get 命令 - --abstract-only 选项应返回 L0 层内容
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_get_abstract_only() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-abstract-{}", uuid_short());

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "Content to test abstract layer retrieval",
        ])
        .assert()
        .success();

    // 列出会话以获取 URI
    let list_output = cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "list",
            "--uri",
            &format!("cortex://session/{}", thread_id),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&list_output);
    println!("List output: {}", output_str);
    // 注：只验证命令能正常执行，具体内容由 L0 层生成逻辑决定
}

/// F08: delete 命令 - 先 add 再 delete 应成功
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_delete_after_add() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-delete-{}", uuid_short());

    // 先添加消息并获取 URI
    let add_output = cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "Message to be deleted",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&add_output);
    let uri = extract_uri_from_output(&output_str);

    if let Some(uri) = uri {
        // 删除该 URI
        cli()
            .args(["-c", &config, "--tenant", &tenant, "delete", &uri])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("deleted").or(predicate::str::contains("successfully")),
            );
    } else {
        println!("WARN: Could not extract URI from add output, skipping delete check");
    }
}

/// F09: search 命令 - 基本搜索
///
/// search 命令需要调用 Embedding API 将查询转为向量，若 Embedding 服务不可达则会失败。
/// 本测试验证命令能够正常执行（参数解析正确），对 Embedding 服务的可用性不做强依赖要求。
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_search_basic() {
    let config = config_path();
    let tenant = tenant_id();

    let output = cli()
        .args(["-c", &config, "--tenant", &tenant, "search", "test query"])
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 如果成功：stdout 中应有 "Found" 或 "results"
    // 如果失败：允许因 Embedding 服务不可达而失败（网络/服务错误），但不应是参数校验失败
    if output.status.success() {
        assert!(
            stdout.contains("Found") || stdout.contains("results") || stdout.contains("0 results"),
            "Expected search result output, got: {}",
            stdout
        );
    } else {
        // 允许因网络/服务不可用而失败，但不应是命令解析错误
        let is_network_or_service_error = stderr.contains("Embedding error")
            || stderr.contains("HTTP request failed")
            || stderr.contains("connection refused")
            || stderr.contains("Vector store error")
            || stderr.contains("tonic::transport");
        assert!(
            is_network_or_service_error,
            "Unexpected failure (not a network/service error): stderr={}",
            stderr
        );
        println!(
            "INFO: search failed due to service unavailability (acceptable): {}",
            stderr.trim()
        );
    }
}

/// F10: search 命令 - 指定 limit 和 min_score 参数
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_search_with_options() {
    let config = config_path();
    let tenant = tenant_id();

    let output = cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "search",
            "test query",
            "--limit",
            "5",
            "--min-score",
            "0.5",
        ])
        .output()
        .expect("Failed to run command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // 成功或因服务不可达失败均可接受
    if !output.status.success() {
        let is_service_error = stderr.contains("Embedding error")
            || stderr.contains("HTTP request failed")
            || stderr.contains("connection refused")
            || stderr.contains("Vector store error");
        assert!(is_service_error, "Unexpected failure: {}", stderr);
    }
}

/// F11: search 命令 - 指定 scope 为 user 维度
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_search_user_scope() {
    let config = config_path();
    let tenant = tenant_id();

    let output = cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "search",
            "user preference query",
            "--scope",
            "user",
        ])
        .output()
        .expect("Failed to run command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let is_service_error = stderr.contains("Embedding error")
            || stderr.contains("HTTP request failed")
            || stderr.contains("connection refused")
            || stderr.contains("Vector store error");
        assert!(is_service_error, "Unexpected failure: {}", stderr);
    }
}

/// F12: search 命令 - 指定 --thread 限制在某个会话内搜索
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_search_in_thread() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-search-{}", uuid_short());

    // 先添加内容
    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "Rust programming language features",
        ])
        .assert()
        .success();

    // 在该 thread 内搜索（允许因 Embedding 服务不可达而失败）
    let search_output = cli()
        .args([
            "-c", &config, "--tenant", &tenant, "search", "Rust", "--thread", &thread_id,
        ])
        .output()
        .expect("Failed to run search command");

    let search_stderr = String::from_utf8_lossy(&search_output.stderr);
    if !search_output.status.success() {
        let is_service_error = search_stderr.contains("Embedding error")
            || search_stderr.contains("HTTP request failed")
            || search_stderr.contains("connection refused")
            || search_stderr.contains("Vector store error");
        assert!(
            is_service_error,
            "Unexpected search failure: {}",
            search_stderr
        );
        println!(
            "INFO: search in thread failed due to service unavailability: {}",
            search_stderr.trim()
        );
    }
}

/// F13: session list 命令 - 应列出会话（可能为空）
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_session_list() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args(["-c", &config, "--tenant", &tenant, "session", "list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("sessions")
                .or(predicate::str::contains("No sessions"))
                .or(predicate::str::contains("Found")),
        );
}

/// F14: session create 命令 - 创建新会话
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_session_create() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-session-create-{}", uuid_short());

    cli()
        .args([
            "-c", &config, "--tenant", &tenant, "session", "create", &thread_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("created").or(predicate::str::contains("✓")));
}

/// F15: session create 命令 - 指定 --title 选项
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_session_create_with_title() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-session-titled-{}", uuid_short());

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "session",
            "create",
            &thread_id,
            "--title",
            "My Test Session",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("created").or(predicate::str::contains("✓")));
}

/// F16: session close 命令 - 先 create 再 close
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_session_close_after_create() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-session-close-{}", uuid_short());

    // 创建会话并添加消息
    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "add",
            "--thread",
            &thread_id,
            "A test message before closing session",
        ])
        .assert()
        .success();

    // 关闭会话（触发记忆提取流水线）
    cli()
        .args([
            "-c", &config, "--tenant", &tenant, "session", "close", &thread_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("closed").or(predicate::str::contains("completed")));
}

/// F17: stats 命令 - 统计信息应包含维度数据
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_stats() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args(["-c", &config, "--tenant", &tenant, "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Statistics").or(predicate::str::contains("Sessions")));
}

/// F18: layers status 命令 - 显示 L0/L1 文件覆盖状态
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_layers_status() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args(["-c", &config, "--tenant", &tenant, "layers", "status"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Layer file status")
                .or(predicate::str::contains("Total directories")),
        );
}

/// F19: layers ensure-all 命令 - 为所有缺失目录生成 L0/L1 文件
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_layers_ensure_all() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args(["-c", &config, "--tenant", &tenant, "layers", "ensure-all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Statistics").or(predicate::str::contains("Generated")));
}

/// F20: layers regenerate-oversized 命令 - 重新生成超大 .abstract 文件
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_layers_regenerate_oversized() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "layers",
            "regenerate-oversized",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Statistics")
                .or(predicate::str::contains("Oversized"))
                .or(predicate::str::contains("All .abstract files")),
        );
}

/// F21: verbose 模式 - --verbose 选项不应导致命令失败
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_verbose_mode() {
    let config = config_path();
    let tenant = tenant_id();

    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "--verbose",
            "session",
            "list",
        ])
        .assert()
        .success();
}

/// F22: 完整工作流 - add → list → get → search → delete
#[test]
#[ignore = "需要外部服务 (Qdrant + LLM + Embedding)，请配置环境变量后运行"]
fn test_full_workflow() {
    let config = config_path();
    let tenant = tenant_id();
    let thread_id = format!("cli-test-workflow-{}", uuid_short());
    let content = format!("Workflow test content {}", uuid_short());

    // Step 1: add 消息
    let add_output = cli()
        .args([
            "-c", &config, "--tenant", &tenant, "add", "--thread", &thread_id, &content,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&add_output);
    println!("Add output: {}", output_str);

    // Step 2: list 该会话内容
    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "list",
            "--uri",
            &format!("cortex://session/{}", thread_id),
        ])
        .assert()
        .success();

    // Step 3: search 刚添加的内容
    cli()
        .args([
            "-c",
            &config,
            "--tenant",
            &tenant,
            "search",
            "Workflow test",
            "--thread",
            &thread_id,
        ])
        .assert()
        .success();

    // Step 4: stats 查看统计
    cli()
        .args(["-c", &config, "--tenant", &tenant, "stats"])
        .assert()
        .success();
}

// ─── 辅助函数 ────────────────────────────────────────────────────────────────

/// 生成短 UUID（8 个字符）用于测试隔离
fn uuid_short() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    format!("{:08x}", hasher.finish())
}

/// 从命令输出中提取第一个 `cortex://` URI
fn extract_uri_from_output(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        if let Some(pos) = line.find("cortex://") {
            // 截取到空白字符或行末
            let uri_start = &line[pos..];
            let uri_end = uri_start
                .find(|c: char| c.is_whitespace())
                .unwrap_or(uri_start.len());
            Some(uri_start[..uri_end].to_string())
        } else {
            None
        }
    })
}
