# Cortex-Mem V2 快速开始指南

**5分钟快速上手Cortex-Mem**

本指南将帮助你快速安装、配置并开始使用Cortex-Mem V2。

---

## 📋 前置要求

### 必需
- ✅ Rust 1.92+ 
- ✅ Cargo（Rust包管理器）
- ✅ Git

### 可选
- OpenAI兼容的LLM API（用于记忆提取功能）
- Claude Desktop（用于MCP集成）
- curl + jq（用于测试HTTP服务）

### 检查环境

```bash
# 检查Rust版本
rustc --version  # 应该 >= 1.92

# 检查Cargo
cargo --version

# 检查Git
git --version
```

---

## 🚀 安装步骤

### 步骤1: 克隆仓库

```bash
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem
```

### 步骤2: 构建项目

```bash
# 构建所有工具（推荐）
cargo build --release --workspace

# 这将构建:
# - cortex-mem (CLI工具)
# - cortex-mem-mcp (MCP服务器)
# - cortex-mem-service (HTTP服务)
```

**预计时间**: 首次构建约3-5分钟（取决于网络和硬件）

### 步骤3: 验证安装

```bash
# 检查CLI工具
./target/release/cortex-mem --version

# 检查MCP服务器
./target/release/cortex-mem-mcp --version

# 检查HTTP服务
./target/release/cortex-mem-service --version
```

---

## ⚙️ 配置

### 基础配置（可选）

如果你需要使用LLM功能（记忆提取），创建配置文件:

```bash
# 在项目根目录创建config.toml
cat > config.toml << 'EOF'
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key-here"
model_efficient = "gpt-4"
temperature = 0.1
max_tokens = 4096
EOF
```

**注意**: 配置文件是可选的，不使用LLM功能可以跳过。

### 支持的LLM服务

- ✅ OpenAI官方API
- ✅ Azure OpenAI
- ✅ 自部署服务（Ollama, LocalAI等）
- ✅ 任何OpenAI兼容的第三方服务

---

## 🎯 第一个示例：使用CLI

### 1. 创建会话

```bash
./target/release/cortex-mem session create my-first-session --title "我的第一个会话"
```

**输出示例**:
```
✓ Session created: my-first-session
  Title: 我的第一个会话
  Status: Active
  Created: 2026-02-04 16:00:00 UTC
```

### 2. 添加消息

```bash
# 添加用户消息
./target/release/cortex-mem add --thread my-first-session \
  "Hello! This is my first message in Cortex-Mem."

# 添加助手回复
./target/release/cortex-mem add --thread my-first-session \
  --role assistant \
  "Hi! Welcome to Cortex-Mem. I can help you manage your memories."
```

### 3. 搜索消息

```bash
./target/release/cortex-mem search "first message" --thread my-first-session
```

**输出示例**:
```
Found 1 result(s)

[1] cortex://threads/my-first-session/timeline/2026-02/04/16_00_00_abc12345.md
    Score: 1.0
    Hello! This is my first message in Cortex-Mem.
```

### 4. 查看会话列表

```bash
./target/release/cortex-mem session list
```

### 5. 关闭会话

```bash
./target/release/cortex-mem session close my-first-session
```

---

## 🌐 启动HTTP服务

### 基础启动

```bash
./target/release/cortex-mem-service
```

**默认配置**:
- Host: `127.0.0.1`
- Port: `8080`
- Data目录: `./cortex-data`

### 自定义启动

```bash
./target/release/cortex-mem-service \
  --data-dir /path/to/data \
  --port 3000 \
  --verbose
```

### 测试HTTP服务

```bash
# 健康检查
curl http://localhost:8080/health | jq

# 创建会话
curl -X POST http://localhost:8080/api/v2/sessions \
  -H "Content-Type: application/json" \
  -d '{"thread_id": "api-test", "title": "API测试会话"}' | jq

# 添加消息
curl -X POST http://localhost:8080/api/v2/sessions/api-test/messages \
  -H "Content-Type: application/json" \
  -d '{"role": "user", "content": "Hello from API!"}' | jq

# 搜索
curl -X POST http://localhost:8080/api/v2/search \
  -H "Content-Type: application/json" \
  -d '{"query": "hello", "limit": 5}' | jq
```

或使用测试脚本:

```bash
cd cortex-mem-service
./test.sh
```

---

## 🔌 配置Claude Desktop（MCP集成）

### 步骤1: 编辑Claude配置

```bash
# macOS
nano ~/Library/Application\ Support/Claude/claude_desktop_config.json

# Linux
nano ~/.config/Claude/claude_desktop_config.json
```

### 步骤2: 添加MCP服务器

```json
{
  "mcpServers": {
    "cortex-mem": {
      "command": "/path/to/cortex-mem/target/release/cortex-mem-mcp",
      "args": ["--config", "/path/to/config.toml"],
      "env": {
        "CORTEX_DATA_DIR": "/path/to/cortex-data"
      }
    }
  }
}
```

**注意**: 将路径替换为你的实际路径。

### 步骤3: 重启Claude Desktop

关闭并重新打开Claude Desktop。

### 步骤4: 验证集成

在Claude中输入：

```
请使用cortex-mem工具存储一条记忆："我喜欢使用Rust编程"
```

如果成功，Claude会调用`store_memory`工具。

---

## 📂 数据存储结构

Cortex-Mem将所有数据存储在`cortex-data`目录（可配置）:

```
cortex-data/
├── threads/                    # 会话数据
│   └── my-first-session/      # 会话目录
│       ├── .session.json      # 会话元数据
│       └── timeline/          # 时间线
│           └── 2026-02/       # 按月组织
│               └── 04/        # 按日组织
│                   └── 16_00_00_abc12345.md  # 消息文件
├── users/                     # 用户记忆（未来）
├── agents/                    # Agent记忆（未来）
└── index/                     # 索引数据
```

所有文件都是**纯Markdown**，可以：
- ✅ 用任何文本编辑器查看
- ✅ 纳入Git版本控制
- ✅ 手动编辑和备份
- ✅ 迁移到其他系统

---

## 🧪 运行测试

```bash
# 运行所有测试
cargo test --workspace

# 查看测试详情
cargo test --workspace -- --nocapture

# 只测试核心库
cargo test -p cortex-mem-core
```

**预期结果**: 56/57测试通过（1个测试需要LLM API配置）

---

## 🐛 故障排除

### 问题1: 编译失败

**症状**: `error: failed to compile cortex-mem`

**解决方案**:
```bash
# 更新Rust
rustup update

# 清理并重新构建
cargo clean
cargo build --release --workspace
```

### 问题2: LLM功能不可用

**症状**: 记忆提取失败，提示"LLM client not configured"

**解决方案**:
1. 确保`config.toml`存在且配置正确
2. 检查API密钥是否有效
3. 测试API连接：
```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://api.openai.com/v1/models
```

### 问题3: MCP集成不工作

**症状**: Claude无法识别cortex-mem工具

**解决方案**:
1. 检查配置文件路径是否正确
2. 确保二进制文件有执行权限：
```bash
chmod +x ./target/release/cortex-mem-mcp
```
3. 查看Claude的日志（Help > Debug Info）

### 问题4: 端口被占用

**症状**: HTTP服务启动失败，"Address already in use"

**解决方案**:
```bash
# 使用其他端口
./target/release/cortex-mem-service --port 9090
```

---

## 📚 下一步

### 深入学习

1. **架构文档**: 阅读 [docs/ARCHITECTURE.md](ARCHITECTURE.md) 了解系统设计
2. **模块文档**: 查看 `docs/modules/` 了解各模块细节
3. **工具文档**: 
   - [CLI工具](../cortex-mem-cli/README.md)
   - [MCP服务器](../cortex-mem-mcp/README.md)
   - [HTTP服务](../cortex-mem-service/README.md)

### 实践项目

1. **个人知识库**: 使用CLI记录学习笔记
2. **AI助手记忆**: 集成到你的AI工作流
3. **团队知识共享**: 使用HTTP API构建团队工具

### 贡献

- 提交Issue反馈问题
- 参与Discussions讨论
- 贡献代码改进

---

## 💡 技巧和最佳实践

### 1. 数据备份

```bash
# 定期备份数据目录
tar -czf cortex-data-backup-$(date +%Y%m%d).tar.gz cortex-data/

# 或使用Git
cd cortex-data
git init
git add .
git commit -m "Backup $(date)"
```

### 2. 性能优化

```bash
# 使用Release构建（比Debug快10-100倍）
cargo build --release

# 定期清理旧会话
./target/release/cortex-mem session list --status closed | \
  xargs -I {} ./target/release/cortex-mem session delete {}
```

### 3. 开发调试

```bash
# 启用详细日志
RUST_LOG=debug ./target/release/cortex-mem-service --verbose

# 查看文件系统结构
tree cortex-data/

# 手动查看会话数据
cat cortex-data/threads/my-session/.session.json | jq
```

---

## 🎉 完成！

恭喜！你已经成功安装并运行了Cortex-Mem V2。

**快速回顾**:
- ✅ 安装并构建了所有工具
- ✅ 创建了第一个会话
- ✅ 测试了CLI、HTTP服务和MCP集成
- ✅ 了解了数据存储结构

**接下来**:
- 探索更多CLI命令
- 尝试HTTP API
- 集成到你的AI工作流
- 查看高级功能文档

如有问题，欢迎提交Issue或参与Discussions！

---

**Happy Hacking! 🚀**
