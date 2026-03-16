# Cortex Memory HTTP Service

`cortex-mem-service` 提供基于 Axum 框架的 HTTP REST API，是 Cortex Memory 系统与外部世界交互的主要桥梁。

## ✨ 功能特性

- 🌐 **完整 API**: 覆盖 Cortex Memory 核心功能的完整 REST API
- 🔄 **异步架构**: 基于 Tokio 异步运行时，支持高并发请求
- 🔍 **多模式搜索**: 文件系统搜索、向量搜索、混合搜索
- 📁 **文件系统访问**: 浏览和操作虚拟文件系统
- 🧠 **记忆提取**: 通过 LLM 自动提取和结构化记忆
- 🚀 **灵活部署**: 支持单节点和多租户配置
- 📡 **OpenAPI**: 完整的 API 文档支持
- 📊 **可观测性**: 集成日志、指标和健康检查

## 🚀 快速开始

### 安装与启动

```bash
# 构建服务
cd cortex-mem
cargo build --release -p cortex-mem-service

# 使用默认配置启动（默认端口 8085）
./target/release/cortex-mem-service

# 指定端口和数据目录
./cortex-mem-service --port 3000 --data-dir /var/lib/cortex-data
```

### Docker 部署

```bash
# 构建镜像
docker build -t cortex-mem-service -f docker/Dockerfile .

# 运行容器
docker run -d \
  --name cortex-mem \
  -p 8085:8085 \
  -v $(pwd)/cortex-data:/app/cortex-data \
  cortex-mem-service
```

## 📖 API 文档

### 健康检查

```http
GET /health
```

响应示例：
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T14:30:00Z",
  "version": "2.0.0",
  "dependencies": {
    "llm": "connected",
    "qdrant": "connected"
  }
}
```

### 会话管理

#### 创建会话

```http
POST /api/v2/sessions
Content-Type: application/json

{
  "thread_id": "customer-support-123",
  "title": "客户支持会话",
  "participants": [
    {
      "id": "user-123",
      "name": "张三",
      "role": "user"
    },
    {
      "id": "support-bot",
      "name": "支持助手",
      "role": "assistant"
    }
  ]
}
```

#### 获取会话详情

```http
GET /api/v2/sessions/{thread_id}
```

#### 关闭会话

```http
POST /api/v2/sessions/{thread_id}/close
```

### 消息操作

#### 添加消息

```http
POST /api/v2/sessions/{thread_id}/messages
Content-Type: application/json

{
  "role": "user",
  "content": "忘记密码了怎么办？",
  "metadata": {
    "tags": ["password", "help"]
  }
}
```

#### 获取消息时间轴

```http
GET /api/v2/sessions/{thread_id}/timeline?start=2024-01-01&end=2024-01-31
```

### 文件系统操作

#### 列出目录

```http
GET /api/v2/filesystem?uri=cortex://session&recursive=false
```

#### 读取文件内容

```http
GET /api/v2/filesystem/read?uri=cortex://session/support-123/.session.json
```

#### 写入文件

```http
POST /api/v2/filesystem/write
Content-Type: application/json

{
  "uri": "cortex://user/user-123/preferences.md",
  "content": "# 用户偏好\n\n- 主题：深色\n- 语言：中文"
}
```

### 搜索

#### 文件系统搜索

```http
POST /api/v2/search
Content-Type: application/json

{
  "query": "密码重置",
  "mode": "filesystem",
  "filters": {
    "dimensions": ["session"],
    "tenants": ["customer-support"],
    "date_range": {
      "start": "2024-01-01T00:00:00Z",
      "end": "2024-01-31T23:59:59Z"
    }
  },
  "limit": 10,
  "offset": 0
}
```

#### 向量搜索（需要 vector-search 功能）

```http
POST /api/v2/search
Content-Type: application/json

{
  "query": "如何更改密码",
  "mode": "vector",
  "filters": {
    "dimensions": ["user", "session"]
  },
  "limit": 5,
  "min_score": 0.7
}
```

#### 混合搜索

```http
POST /api/v2/search
Content-Type: application/json

{
  "query": "账户设置",
  "mode": "hybrid",
  "filters": {
    "dimensions": ["user", "resources"]
  },
  "limit": 15
}
```

### 记忆提取

#### 触发记忆提取

```http
POST /api/v2/automation/extract/{thread_id}
Content-Type: application/json

{
  "auto_save": true,
  "dimensions": ["user"],
  "extraction_types": ["facts", "preferences", "decisions"]
}
```

响应示例：
```json
{
  "success": true,
  "data": {
    "thread_id": "customer-support-123",
    "facts_count": 5,
    "preferences_count": 3,
    "decisions_count": 2,
    "entities_count": 7,
    "extracted": {
      "facts": [
        {
          "content": "用户忘记了登录密码",
          "confidence": 0.95,
          "category": "auth",
          "source_uri": "cortex://session/customer-support-123/timeline/2024/01/15/14_30_00_abc123.md"
        }
      ],
      "preferences": [
        {
          "content": "希望通过电子邮件接收通知",
          "confidence": 0.9,
          "source": "user stated in conversation"
        }
      ],
      "decisions": [
        {
          "content": "用户决定重置密码而不是联系管理员",
          "confidence": 0.8,
          "source": "user choice"
        }
      ]
    },
    "timestamp": "2024-01-15T14:30:00Z"
  }
}
```

## ⚙️ 配置

### 命令行参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--port` / `-p` | `8085` | 监听端口 |
| `--host` | `127.0.0.1` | 绑定地址 |
| `--data-dir` / `-d` | `./cortex-data` | 数据目录 |
| `--verbose` / `-v` | `false` | 启用详细日志 |

### 环境变量

```bash
# 数据存储
export CORTEX_DATA_DIR="/var/lib/cortex-data"

# 外部服务
export QDRANT_URL="http://localhost:6333"
export LLM_API_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="your-api-key"

# 服务配置
export CORTEX_SERVICE_PORT=8085
export CORTEX_SERVICE_HOST="0.0.0.0"
export RUST_LOG="cortex_service=debug"
```

### 配置文件

可以创建 `config.toml` 文件进行详细配置：

```toml
[server]
host = "127.0.0.1"
port = 8085
workers = 4
max_connections = 1024

[cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["*"]

[limits]
max_body_size = "10MB"
max_message_length = 10000
search_limit = 100
```

## 🔧 运行模式

### 开发模式

```bash
# 使用开发配置启动
cargo run -p cortex-mem-service -- --data-dir ./dev-data --verbose

# 启用自动重载（需要 watch）
install-watch
cargo watch -x 'run -p cortex-mem-service'
```

### 生产模式

```bash
# 优化构建
cargo build --release -p cortex-mem-service

# 使用 systemd 服务
sudo systemctl start cortex-mem-service

# 使用 PM2 管理
pm2 start /path/to/cortex-mem-service --name cortex-mem
```

### 多租户模式

```bash
# 设置租户配置
export CORTEX_MULTITENANT=true
export CORTEX_DEFAULT_TENANT="default"

# 启动服务
./cortex-mem-service --multitenant --tenant-isolation
```

## 🧪 测试

### 单元测试

```bash
# 运行所有测试
cargo test -p cortex-mem-service

# 运行集成测试
cargo test -p cortex-mem-service --test '*_test'
```

### API 测试

```bash
# 使用提供的测试脚本
./cortex-mem-service/test.sh

# 使用 curl 测试
curl -X GET http://localhost:8080/health

# 使用 pytest 进行 API 测试
cd tests/pytest
python -m pytest api_tests.py
```

### 性能测试

```bash
# 使用 wrk 进行基准测试
wrk -t12 -c400 -d30s http://localhost:8080/api/v2/search \
  -s tests/search-post.lua

# 使用 hey 进行简单负载测试
hey -n 10000 -c 50 -m POST -d @tests/search-query.json \
  http://localhost:8080/api/v2/search
```

## 📊 监控与可观测性

### 日志记录

服务使用 `tracing` 框架进行结构化日志：

```json
{
  "timestamp": "2024-01-15T14:30:00Z",
  "level": "INFO",
  "target": "cortex_service::handlers::search",
  "message": "Search completed",
  "fields": {
    "query": "password reset",
    "results_count": 42,
    "duration_ms": 234,
    "tenant_id": "customer-support"
  }
}
```

### 健康检查

```bash
# 基础健康检查
GET /health

# 详细健康检查（包含依赖）
GET /health/detailed
```

### 指标收集

```bash
# Prometheus 格式指标
GET /metrics

# 指标示例
# TYPE cortex_duration_seconds histogram
cortex_duration_seconds_bucket{le="1.0",endpoint="/api/v2/search"} 125
cortex_duration_seconds_bucket{le="5.0",endpoint="/api/v2/search"} 98
```

## 🔐 安全注意事项

### 当前限制

⚠️ **重要**: 当前版本**不包含**认证和授权机制。

适用于：
- 本地开发环境
- 受信任的内网环境
- 前置代理已处理认证的部署

### 安全最佳实践

1. **使用反向代理**:
```nginx
location /api/ {
  auth_basic "Cortex API";
  auth_basic_user_file .htpasswd;
  proxy_pass http://localhost:8080/api/;
}
```

2. **网络安全**:
```bash
# 使用限制性防火墙规则
ufw allow from 10.0.0.0/8 to any port 8080
```

3. **数据加密**:
```bash
# 使用加密文件系统
fscrypt encrypt directory
```

## 🚨 常见问题

### 服务无法启动

**问题**: `Address already in use`
**解决**: 更换端口或终止占用进程
```bash
# 查找进程占用
lsof -i :8080

# 终止进程
kill -9 <PID>

# 或使用其他端口
./cortex-mem-service --port 9090
```

### 搜索无结果

**问题**: Search returns empty results
**解决**: 检查以下配置
```bash
# 确保数据目录有效
ls -la $CORTEX_DATA_DIR

# 检查向量搜索配置（如果启用）
curl -X GET http://localhost:6333/collections

# 启用详细日志进行调试
RUST_LOG=debug ./cortex-mem-service --verbose
```

### CORS 错误

**问题**: CORS policy error in browser
**解决**: 配置允许的源
```bash
# 在生产环境指定具体源
./cortex-mem-service --cors "https://app.example.com"

# 或使用配置文件
[server]
cors_origins = ["https://app.example.com"]
```

## 📚 更多资源

- [Cortex Memory 主文档](../README.md)
- [核心库 API](../cortex-mem-core/README.md)
- [架构文档](../../litho.docs/en)
- [API 完整参考](docs/openapi.yaml)
- [部署指南](docs/deployment.md)

## 🤝 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支
3. 编写测试
4. 提交 PR

## 📄 许可证

MIT 许可证 - 详见 [LICENSE](../../LICENSE) 文件
