# Cortex Memory TARS

这是一个基于 Cortex Memory 的 TUI（终端用户界面）聊天应用，具有记忆功能。它能够记住用户的对话历史和个人信息，提供更智能的对话体验。

## 功能特性

- 🧠 **记忆功能**：自动记忆用户的对话历史和个人信息
- 🤖 **智能 AI 助手**：支持多个机器人配置，每个机器人可以有不同的系统提示词
- 📝 **Markdown 渲染**：支持 Markdown 格式的消息显示
- 💾 **对话导出**：可以将对话导出到剪贴板
- 🔧 **灵活配置**：支持自定义 LLM API、向量存储等配置
- 🎨 **现代化 TUI**：基于 ratatui 的美观终端界面

## 安装

### 前置要求

- Rust 1.70 或更高版本
- Qdrant 向量数据库（可选，用于记忆功能）
- OpenAI API 密钥或其他兼容的 LLM API

### 构建项目

```bash
cd examples/cortex-mem-tars-new
cargo build --release
```

## 配置

### 1. 创建配置文件

将 `config.example.toml` 复制为 `config.toml` 并修改相应的配置：

```bash
cp config.example.toml config.toml
```

### 2. 修改配置

编辑 `config.toml` 文件，至少需要配置以下内容：

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "your-actual-api-key"
model_efficient = "gpt-4o-mini"

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "your-actual-api-key"
model_name = "text-embedding-3-small"

[qdrant]
url = "http://localhost:6334"
```

### 3. 启动 Qdrant（可选，用于记忆功能）

如果你使用记忆功能，需要启动 Qdrant 向量数据库：

```bash
# 使用 Docker
docker run -p 6334:6334 qdrant/qdrant

# 或使用本地安装
qdrant
```

## 使用方法

### 运行应用

```bash
cargo run --release
```

### 基本操作

- **Enter**：发送消息
- **Shift+Enter**：换行
- **Ctrl+L**：打开/关闭日志面板
- **Esc**：关闭日志面板
- **Ctrl+H**：显示帮助信息
- **Ctrl+C**：清空会话
- **Ctrl+D**：导出对话到剪贴板
- **q**：退出程序

### 命令

在输入框中输入以下命令：

- `/quit`：退出程序
- `/clear`：清空会话
- `/help`：显示帮助信息
- `/dump`：导出对话到剪贴板

## 项目结构

```
cortex-mem-tars-new/
├── src/
│   ├── main.rs           # 主程序入口
│   ├── app.rs            # 应用程序主逻辑
│   ├── agent.rs          # Agent 实现（包括记忆功能）
│   ├── config.rs         # 配置管理
│   ├── infrastructure.rs # 基础设施（LLM、向量存储、记忆管理器）
│   ├── logger.rs         # 日志系统
│   └── ui.rs             # TUI 界面
├── config.example.toml   # 配置文件示例
└── README.md            # 本文件
```

## 核心功能

### 1. 记忆功能

应用会自动：

- 在启动时加载用户的基本信息（个人特征、事实信息等）
- 在对话过程中使用记忆工具检索相关信息
- 在退出时将对话历史保存到记忆系统

### 2. 多机器人支持

可以在配置目录中创建多个机器人配置，每个机器人可以有：

- 不同的名称
- 不同的系统提示词
- 不同的访问密码

### 3. 流式响应

支持实时的流式 AI 响应，提供更流畅的对话体验。

## 故障排除

### 1. 无法连接到 Qdrant

确保 Qdrant 正在运行并且 URL 配置正确：

```bash
curl http://localhost:6334/health
```

### 2. API 密钥错误

检查 `config.toml` 中的 API 密钥是否正确。

### 3. 记忆功能不工作

- 确保 Qdrant 正在运行
- 检查 API 密钥是否正确
- 查看日志面板获取详细错误信息

## 开发

### 运行测试

```bash
cargo test
```

### 检查代码

```bash
cargo check
```

### 格式化代码

```bash
cargo fmt
```

## 许可证

MIT

## 致谢

- [Cortex Memory](https://github.com/sopaco/cortex-mem) - 记忆管理系统
- [RatATUI](https://github.com/ratatui-org/ratatui) - TUI 框架
- [Rig](https://github.com/0xPlaygrounds/rig) - LLM Agent 框架
