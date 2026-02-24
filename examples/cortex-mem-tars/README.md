# Cortex TARS

<p align="center">
  <strong>🎧 基于 Cortex Memory 的交互式 AI 助手 — 终端用户界面版</strong>
</p>

Cortex TARS 是一个基于 **Cortex Memory** 的生产级 TUI（终端用户界面）应用。它不仅是一个聊天机器人，更是一个具备语音输入和持久记忆功能的智能 AI 助手平台。

## ✨ 核心特性

### 🤖 多 Agent 管理
创建和管理多个 AI 机器人，每个机器人具有独特的个性、系统提示词和专业知识领域。无论是编程助手、写作伙伴还是生产力教练，都可以同时运行。

### 🧠 分层记忆系统
- **L0 摘要层**：快速判断记忆相关性（~100 tokens）
- **L1 概览层**：理解核心信息（~2000 tokens）
- **L2 完整层**：获取完整内容
- 基于 Qdrant 向量数据库的语义搜索
- 租户隔离的记忆架构

### 🎤 语音输入
- 实时麦克风音频捕获
- Whisper 语音转录（中英文支持）
- 音频增益和预处理
- 静音检测和过滤

### 🎨 现代 TUI 体验
- 基于 ratatui 的精美终端界面
- 多个预设主题（Default、Dark、Forest、Ocean、Sunset）
- Markdown 完整支持
- AI 响应实时流式输出
- 会话导出到剪贴板

### 💾 记忆持久化
- 退出时自动提取会话记忆
- 基于 Qdrant 的向量搜索
- 用户信息和对话内容自动提取
- Timeline 层（L0/L1）自动生成

## 📋 前置要求

- **Rust** 1.70+
- **Qdrant** 向量数据库（用于语义搜索和记忆存储）
- **OpenAI 兼容**的 LLM API 端点（如 OpenAI、Azure OpenAI、本地 LLM 服务等）
- **OpenAI 兼容**的 Embedding API 端点（用于向量化）
- **麦克风**（可选，用于语音输入功能）

### 系统依赖

**Linux 系统**需要额外安装音频相关依赖：

```bash
# Ubuntu/Debian
sudo apt-get install libasound2-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Fedora/RHEL
sudo dnf install alsa-lib-devel libxcb-devel
```

**macOS** 和 **Windows** 通常无需额外依赖。

## 🚀 安装

### 克隆并构建

```bash
cd examples/cortex-mem-tars
cargo build --release
```

编译后的二进制文件位于 `target/release/cortex-mem-tars`。

## ⚙️ 配置

### 1. 创建配置文件

复制示例配置：

```bash
cp config.example.toml config.toml
```

### 2. 编辑配置

编辑 `config.toml`：

```toml
# Qdrant 向量数据库配置
[qdrant]
url = "http://localhost:6334"
collection_name = "memo-rs"
timeout_secs = 30

# Embedding 配置（用于向量搜索）
[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key-here"
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

# LLM 配置（用于 Agent 对话）
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "your-api-key-here"
model_efficient = "gpt-4o-mini"
temperature = 0.1
max_tokens = 40960

# HTTP 服务器配置（预留）
[server]
host = "0.0.0.0"
port = 3000
cors_origins = ["*"]

# Cortex Memory 数据目录
[cortex]
# 优先级：
#   1. 环境变量 CORTEX_DATA_DIR（最高优先级）
#   2. 本配置文件中的 data_dir
#   3. 应用数据目录（TARS 默认）
#      - macOS: ~/Library/Application Support/com.cortex-mem.tars/cortex
#      - Linux: ~/.local/share/cortex-mem-tars/cortex
#      - Windows: %APPDATA%\cortex-mem\tars\cortex
# 留空或注释此行将使用默认值（应用数据目录）
# data_dir = "/path/to/custom/cortex/data"

# 日志配置
[logging]
enabled = true
log_directory = "logs"
level = "debug"
```

### 3. 启动 Qdrant

```bash
# 使用 Docker
docker run -p 6334:6334 qdrant/qdrant
```

## 🎮 使用方法

### 基本命令

```bash
# 运行应用
cargo run --release
# 或直接运行已编译的二进制文件
./target/release/cortex-mem-tars
```

**重要提示**：在 macOS/Linux 系统上，如果遇到 "Too many open files" 错误，请先增加文件描述符限制：

```bash
# 设置文件描述符限制（临时生效，当前终端会话）
ulimit -n 65536

# 然后启动应用
cargo run --release
```

> 💡 **提示**：永久修改系统限制的方法，请参见下方的 [FAQ: "Too many open files" 错误](#too-many-open-files-错误)。

### 键盘快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Shift+Enter` | 输入换行 |
| `Ctrl+C` | 清空当前会话 |
| `Ctrl+D` | 导出会话到剪贴板 |
| `Ctrl+H` | 显示帮助 |
| `Ctrl+T` | 打开主题选择器 |
| `Ctrl+B` | 打开机器人管理 |
| `Ctrl+A` | 启用语音输入 |
| `q` | 退出应用（在机器人选择界面） |
| `Esc` | 关闭弹窗 / 返回上一状态 |

### 机器人管理

1. **创建新机器人**：按 `Ctrl+B` → 选择"Create Bot"
2. **设置机器人属性**：
   - **Name**：显示名称
   - **System Prompt**：机器人个性和行为指令
   - **Password**：可选的访问密码（用于保护特定机器人）
3. **切换机器人**：按 `Ctrl+B` 选择不同的机器人
4. **编辑机器人**：在机器人列表中选择"Edit Bot"
5. **删除机器人**：在机器人列表中选择"Delete Bot"

每个机器人维护独立记忆（基于 `agent_id` 的租户隔离），确保知识和上下文完全分离。

### 语音输入使用

1. 按 `Ctrl+A` 启用语音输入
2. 界面会显示 "🎙️ 语音输入已启用"
3. 对着麦克风说话，系统会自动将语音转录为文本
4. 转录的文本会自动发送到 AI 助手
5. 再次按 `Ctrl+A` 可以关闭语音输入

**提示**：
- 支持中英文混合语音识别
- 使用 Whisper 模型进行转录
- 转录过程在本地完成（需要 Whisper 模型文件）

### 命令支持

在聊天界面输入以下命令：

```
/help     - 显示帮助信息
/theme    - 切换主题
/clear    - 清空当前会话
/dump     - 导出会话到剪贴板
```

**提示**：命令也可以通过键盘快捷键触发（如 `Ctrl+H` 显示帮助）。

## 🏗️ 项目架构

```
cortex-mem-tars/
├── src/
│   ├── main.rs              # 应用入口
│   ├── app.rs               # 核心应用逻辑和事件处理
│   ├── agent.rs             # AI Agent 与 Cortex Memory 集成
│   ├── config.rs            # 配置管理和机器人配置
│   ├── infrastructure.rs   # 基础设施（LLM、向量存储）
│   ├── ui.rs                # TUI 界面和渲染
│   ├── logger.rs            # 日志系统
│   ├── audio_input.rs       # 麦克风音频捕获
│   └── audio_transcription.rs # Whisper 语音转录
├── config.example.toml      # 配置模板
└── README.md
```

## 🧠 记忆系统工作原理

Cortex TARS 利用 Cortex Memory 的智能记忆系统：

1. **会话记录**：所有对话自动保存到文件系统（按 session_id 组织）
2. **退出时提取**：应用退出时，自动触发记忆提取流程：
   - 生成 Timeline 层（L0 摘要、L1 概览）
   - 提取用户基本信息
   - 提取对话中的关键事实和洞察
3. **向量存储**：提取的记忆通过 Embedding API 向量化后存储到 Qdrant
4. **语义检索**：Agent 在生成响应前，可以使用 `search` 工具检索相关记忆
5. **租户隔离**：每个机器人（agent_id）的记忆完全隔离，互不干扰

### 记忆工具（Agent 可用）

Cortex Memory 为 Agent 提供以下工具：

- **search(query, scope, layer)**：语义搜索记忆
  - `query`: 搜索查询
  - `scope`: 搜索范围（如 `user/`, `agent/`, `session/xxx`）
  - `layer`: 访问层级（`abstract`/L0, `overview`/L1, `full`/L2）

- **abstract(uri)**：获取 L0 摘要（~100 tokens）

- **overview(uri)**：获取 L1 概览（~2000 tokens）

- **read(uri)**：获取 L2 完整内容

- **ls(uri)**：列出目录内容

### 记忆 URI 格式

```
cortex://user/{user_id}/              - 用户记忆目录
cortex://user/{user_id}/profile.json  - 用户档案
cortex://agent/{agent_id}/             - Agent 记忆目录
cortex://session/{session_id}/         - 特定会话
cortex://resources/                    - 知识库
```

### 分层访问示例

```rust
// Agent 在对话中可以这样使用记忆工具：

// 1. 搜索相关记忆（L1 层，平衡速度和信息量）
search("用户的编程语言偏好", scope="user/", layer="overview")

// 2. 快速判断相关性（L0 层，最快）
abstract("cortex://user/alice/profile.json")

// 3. 获取完整信息（L2 层，完整内容）
read("cortex://session/2024-02-20-conversation-01/")
```

## 🔧 开发相关

### 运行测试

```bash
cargo test
```

### 代码检查

```bash
cargo check
```

### 代码格式化

```bash
cargo fmt
```

### 发布构建

```bash
cargo build --release
```

## 🐛 故障排除 / FAQ

### Qdrant 连接问题

验证 Qdrant 是否运行：

```bash
curl http://localhost:6334/health
```

检查 `config.toml` 中的 Qdrant URL 配置。

如果需要启动 Qdrant：

```bash
# 使用 Docker
docker run -p 6334:6334 -v $(pwd)/qdrant_storage:/qdrant/storage qdrant/qdrant
```

### LLM API 错误

- 验证 API 密钥正确
- 检查 API 端点 URL
- 确保 API 配额充足
- 查看日志获取详细错误信息

日志文件位置：
- macOS: `~/Library/Application Support/com.cortex-mem.tars/logs/`
- Linux: `~/.local/share/cortex-mem-tars/logs/`

### 记忆功能不工作

- 确保 Qdrant 正常运行
- 验证 LLM 和 embedding 服务的 API 密钥
- 检查日志中的向量存储相关错误
- 确认 `config.toml` 中的 embedding 配置正确

### 语音输入问题

- 检查麦克风权限
- 确认音频设备可用
- 查看日志中的音频配置信息
- 按 `Ctrl+A` 启用语音输入后，确认界面显示 "🎙️ 语音输入已启用"

### "Too many open files" 错误

这是操作系统对单个进程可打开文件描述符的限制。Qdrant（及其底层的 RocksDB）在处理大量数据时需要同时打开许多文件。

#### macOS 系统

**临时生效（当前终端会话）：**

```bash
# 查看当前限制
ulimit -n

# 设置为更高的值（如 65536）
ulimit -n 65536

# 然后在同一终端启动应用
cargo run --release
```

**永久生效：**

1. 创建配置文件：

```bash
sudo nano /Library/LaunchDaemons/limit.maxfiles.plist
```

2. 添加以下内容：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>limit.maxfiles</string>
    <key>ProgramArguments</key>
    <array>
      <string>launchctl</string>
      <string>limit</string>
      <string>maxfiles</string>
      <string>65536</string>
      <string>200000</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ServiceIPC</key>
    <false/>
  </dict>
</plist>
```

3. 设置权限并加载：

```bash
sudo chown root:wheel /Library/LaunchDaemons/limit.maxfiles.plist
sudo launchctl load -w /Library/LaunchDaemons/limit.maxfiles.plist
```

4. **重启系统**使其生效。

5. 验证：

```bash
launchctl limit maxfiles
# 应该显示: maxfiles    65536          200000
```

#### Linux 系统

**临时生效：**

```bash
ulimit -n 65536
```

**永久生效：**

1. 编辑 `/etc/security/limits.conf`：

```bash
sudo nano /etc/security/limits.conf
```

2. 添加以下行：

```
*    soft nofile 65536
*    hard nofile 200000
```

3. 编辑 `/etc/pam.d/common-session`（Debian/Ubuntu）或 `/etc/pam.d/system-auth`（RHEL/CentOS）：

```bash
sudo nano /etc/pam.d/common-session
```

添加：
```
session required pam_limits.so
```

4. 重新登录或重启系统。

5. 验证：

```bash
ulimit -n
# 应该显示 65536
```

> 💡 **为什么会出现这个错误？**
>
> 当应用退出时，Cortex Memory 会自动提取会话记忆并保存到 Qdrant 向量数据库。这个过程涉及：
> - 创建 timeline 层（L0 摘要、L1 概览）
> - 提取用户信息和记忆
> - 向量化并索引到 Qdrant
>
> 如果会话消息较多，Qdrant 的 RocksDB 存储引擎需要同时打开大量文件（segment 文件、索引文件、WAL 日志等），可能超过系统默认限制（macOS 通常是 256 或 1024）。

## 📚 相关资源

- [Cortex Memory 文档](https://github.com/sopaco/cortex-mem/tree/main/litho.docs)
- [Cortex Memory Core](../../cortex-mem-core)
- [Cortex Memory Tools](../../cortex-mem-tools)
- [RatATUI 框架](https://github.com/ratatui-org/ratatui)
- [Rig Agent 框架](https://github.com/0xPlaygrounds/rig)
- [Qdrant 向量数据库](https://qdrant.tech/)

## 📄 许可证

MIT 许可证 - 详见 [LICENSE](../../LICENSE)。

## 🙏 致谢

- **Cortex Memory**：为 AI 提供持久记忆的智能框架
- **RatATUI**：精美的终端 UI 框架
- **Rig**：用于构建智能系统的 LLM Agent 框架
- **Qdrant**：用于语义搜索的高性能向量数据库
- **whisper-rs**：Rust 实现的 Whisper 语音识别

---

**Cortex TARS** — 让 AI 在终端中拥有持久记忆。🚀