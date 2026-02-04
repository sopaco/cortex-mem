# Cortex-Mem Service V2

**HTTP REST API service for Cortex-Mem V2**

基于Cortex-Mem V2核心库的HTTP REST API服务，提供完整的Web访问接口。

---

## 🎯 特性

- ✅ **RESTful API** - 符合REST规范的HTTP接口
- ✅ **会话管理** - 创建、管理、关闭会话
- ✅ **消息存储** - 保存和检索会话消息
- ✅ **文件系统访问** - 浏览cortex://文件系统
- ✅ **多模式搜索** - 支持文件系统、向量、混合搜索
- ✅ **记忆提取** - 自动提取事实、决策、实体
- ✅ **向量搜索** - 可选的语义相似度搜索（feature-gated）
- ✅ **无需鉴权** - 简化部署，专注功能

---

## 📦 安装

### 基础安装（仅文件系统搜索）

```bash
cargo build --release -p cortex-mem-service
```

### 完整安装（包含向量搜索）

```bash
cargo build --release -p cortex-mem-service --features vector-search
```

---

## 🚀 快速开始

### 基础启动

```bash
# 使用默认配置启动（端口8080，数据目录./cortex-data）
cargo run -p cortex-mem-service

# 或使用已编译的二进制
./target/release/cortex-mem-service
```

### 自定义配置

```bash
# 指定数据目录和端口
cortex-mem-service --data-dir /path/to/data --port 3000

# 启用详细日志
cortex-mem-service --verbose

# 查看所有选项
cortex-mem-service --help
```

### 使用LLM功能（可选）

如果需要使用记忆提取功能，需要设置环境变量：

```bash
export LLM_API_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="your-api-key"
export LLM_MODEL="gpt-4"

cortex-mem-service
```

### 启用向量搜索（可选）

如果编译时启用了`vector-search` feature，可以配置Qdrant：

```bash
export QDRANT_URL="http://localhost:6333"
export QDRANT_COLLECTION="cortex_memories"
export QDRANT_EMBEDDING_DIM="1536"  # 可选，默认自动检测

# 启动服务（需要先启动Qdrant）
cortex-mem-service --features vector-search
```

**注意**:
- 向量搜索需要运行Qdrant服务器
- 如果未配置Qdrant，向量搜索会降级为文件系统搜索
- 可以使用Docker快速启动Qdrant: `docker run -p 6333:6333 qdrant/qdrant`

---

## 📡 API 端点

### 健康检查

```bash
GET /health
```

**响应**:
```json
{
  "status": "healthy",
  "service": "cortex-mem-service",
  "version": "2.0.0",
  "llm_available": true,
  "timestamp": "2026-02-04T15:30:00Z"
}
```

---

### 会话管理

#### 创建会话

```bash
POST /api/v2/sessions
Content-Type: application/json

{
  "thread_id": "my-session-123",  // 可选，不提供则自动生成
  "title": "我的第一个会话"      // 可选
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "thread_id": "my-session-123",
    "status": "Active",
    "message_count": 0,
    "created_at": "2026-02-04T15:30:00Z",
    "updated_at": "2026-02-04T15:30:00Z"
  },
  "timestamp": "2026-02-04T15:30:00Z"
}
```

#### 列出所有会话

```bash
GET /api/v2/sessions
```

#### 添加消息

```bash
POST /api/v2/sessions/{thread_id}/messages
Content-Type: application/json

{
  "role": "user",  // user | assistant | system
  "content": "Hello, this is my first message!"
}
```

#### 关闭会话

```bash
POST /api/v2/sessions/{thread_id}/close
```

---

### 文件系统操作

#### 列出目录内容

```bash
GET /api/v2/filesystem?uri=cortex://threads
```

**响应**:
```json
{
  "success": true,
  "data": [
    {
      "uri": "cortex://threads/my-session-123",
      "name": "my-session-123",
      "is_directory": true,
      "size": 0,
      "modified": "2026-02-04T15:30:00Z"
    }
  ]
}
```

#### 读取文件内容

```bash
GET /api/v2/filesystem/read/threads/my-session-123/.session.json
```

---

### 搜索

#### 多模式搜索

支持3种搜索模式：**文件系统搜索**、**向量搜索**（需要feature）、**混合搜索**（需要feature）

```bash
POST /api/v2/search
Content-Type: application/json

{
  "query": "hello",
  "mode": "filesystem",         // "filesystem" | "vector" | "hybrid", 默认: "filesystem"
  "thread": "my-session-123",  // 可选，限制搜索范围
  "limit": 10,                 // 可选，默认10
  "min_score": 0.5             // 可选，默认0.0
}
```

**响应**:
```json
{
  "success": true,
  "data": [
    {
      "uri": "cortex://threads/my-session-123/timeline/2026-02/04/15_30_00_abc12345.md",
      "score": 1.0,
      "snippet": "...Hello, this is my first message!...",
      "content": "# Message\n\n...",
      "source": "filesystem"  // "filesystem" | "vector" | "hybrid"
    }
  ]
}
```

**搜索模式说明**:

| 模式 | 描述 | 需要Feature | 需要Qdrant |
|------|------|------------|-----------|
| `filesystem` | 全文关键词搜索 | ❌ | ❌ |
| `vector` | 语义相似度搜索 | ✅ vector-search | ✅ |
| `hybrid` | 结合两种搜索 | ✅ vector-search | ✅ |

**注意**:
- `filesystem`模式总是可用，基于文本匹配
- `vector`和`hybrid`模式需要编译时启用`vector-search` feature
- 如果未配置Qdrant，`vector`和`hybrid`会自动降级为`filesystem`模式

---

### 自动化 - 记忆提取

#### 提取会话记忆

```bash
POST /api/v2/automation/extract/{thread_id}
Content-Type: application/json

{
  "auto_save": false  // 是否自动保存到用户/代理记忆
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "thread_id": "my-session-123",
    "message_count": 5,
    "facts_count": 3,
    "decisions_count": 1,
    "entities_count": 2,
    "facts": [
      {
        "content": "User prefers dark mode",
        "confidence": 0.9,
        "source": "user stated preference"
      }
    ],
    "decisions": [...],
    "entities": [...]
  }
}
```

**注意**: 此功能需要配置LLM环境变量。

---

## 🔧 配置选项

### 命令行参数

| 参数 | 简写 | 默认值 | 说明 |
|------|------|--------|------|
| `--data-dir` | `-d` | `./cortex-data` | 数据存储目录 |
| `--host` | - | `127.0.0.1` | 服务器监听地址 |
| `--port` | `-p` | `8080` | 服务器监听端口 |
| `--verbose` | `-v` | - | 启用详细日志 |

### 环境变量（LLM相关）

| 变量 | 说明 | 示例 |
|------|------|------|
| `LLM_API_BASE_URL` | LLM API基础URL | `https://api.openai.com/v1` |
| `LLM_API_KEY` | LLM API密钥 | `sk-...` |
| `LLM_MODEL` | LLM模型名称 | `gpt-4` |

---

## 🌐 CORS支持

服务默认启用permissive CORS策略，允许所有来源访问。适合开发和内部部署。

---

## 📊 监控和日志

### 日志级别

- 默认: `INFO`
- 详细模式 (`--verbose`): `DEBUG`

### 日志示例

```
2026-02-04T15:30:00Z INFO Starting Cortex-Mem Service V2
2026-02-04T15:30:00Z INFO Data directory: ./cortex-data
2026-02-04T15:30:00Z INFO LLM client initialized
2026-02-04T15:30:00Z INFO Server listening on http://127.0.0.1:8080
```

---

## 🧪 测试

### 使用curl测试

```bash
# 健康检查
curl http://localhost:8080/health

# 创建会话
curl -X POST http://localhost:8080/api/v2/sessions \
  -H "Content-Type: application/json" \
  -d '{"thread_id": "test-123", "title": "Test Session"}'

# 添加消息
curl -X POST http://localhost:8080/api/v2/sessions/test-123/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "Hello!"}'

# 搜索
curl -X POST http://localhost:8080/api/v2/search \
  -H "Content-Type: application/json" \
  -d '{"query": "hello", "limit": 5}'
```

### 使用Postman/Insomnia

导入以下基础URL开始测试：
```
http://localhost:8080
```

---

## 📝 API响应格式

所有API使用统一的响应格式：

### 成功响应

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2026-02-04T15:30:00Z"
}
```

### 错误响应

```json
{
  "error": "Error message here",
  "status": 404
}
```

HTTP状态码：
- `200 OK` - 成功
- `400 Bad Request` - 请求参数错误
- `404 Not Found` - 资源不存在
- `500 Internal Server Error` - 服务器内部错误

---

## 🔐 安全注意事项

**⚠️ 重要**: 当前版本**不包含**任何鉴权或安全机制。

**仅适用于**:
- 本地开发环境
- 内部网络部署
- 受信任的环境

**不适用于**:
- 公网直接暴露
- 多租户环境
- 生产环境（除非有额外的安全层）

如需生产部署，建议：
1. 使用反向代理（Nginx/Caddy）添加认证
2. 使用VPN或内网访问
3. 实施IP白名单

---

## 🏗️ 架构

```
┌─────────────────┐
│   HTTP Client   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Axum Router    │
│  (CORS/Trace)   │
└────────┬────────┘
         │
    ┌────┴────┬──────────┬──────────┐
    ▼         ▼          ▼          ▼
┌────────┐ ┌─────┐  ┌────────┐ ┌─────────┐
│Sessions│ │Files│  │Search  │ │Automate │
└───┬────┘ └──┬──┘  └───┬────┘ └────┬────┘
    │         │          │           │
    └─────────┴──────────┴───────────┘
              │
              ▼
    ┌──────────────────┐
    │ Cortex-Mem Core  │
    │  - Filesystem    │
    │  - Sessions      │
    │  - Extraction    │
    │  - Search        │
    └──────────────────┘
```

---

## 🛣️ Roadmap

未来可能添加的功能：

- [ ] WebSocket支持（实时消息推送）
- [ ] 批量操作API
- [ ] 导出/导入功能
- [ ] 统计和分析API
- [ ] GraphQL支持
- [ ] 鉴权和权限管理（可选）

---

## 🐛 故障排除

### 服务无法启动

**问题**: `Error: Address already in use`  
**解决**: 端口被占用，使用`--port`指定其他端口

```bash
cortex-mem-service --port 9090
```

### LLM功能不可用

**问题**: 记忆提取API返回错误  
**解决**: 检查环境变量是否正确设置

```bash
echo $LLM_API_BASE_URL
echo $LLM_MODEL
# API_KEY不应该echo出来，但确保已设置
```

### 数据目录权限错误

**问题**: `Permission denied`  
**解决**: 确保数据目录有读写权限

```bash
chmod 755 ./cortex-data
```

---

## 📄 License

MIT License - 与Cortex-Mem项目相同

---

## 🤝 贡献

欢迎贡献！请参考主项目的贡献指南。

---

## 📞 支持

- GitHub Issues: [cortex-mem/issues](https://github.com/sopaco/cortex-mem/issues)
- 文档: [主项目README](../README.md)

---

**Built with ❤️ using Axum and Cortex-Mem V2**
