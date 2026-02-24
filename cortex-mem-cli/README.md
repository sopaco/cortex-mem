# Cortex Memory CLI

`cortex-mem-cli` 是 Cortex Memory 系统的命令行界面，提供完整的终端访问功能。作为与系统交互的主要方式之一，它支持会话管理、消息操作、搜索和记忆提取等核心功能。

## ✨ 功能特性

- 🗣️ **会话管理**: 创建、列出、关闭会话
- 💬 **消息操作**: 添加、搜索、删除消息
- 🔍 **智能搜索**: 支持时间范围和维度过滤
- 🧠 **记忆提取**: 自动提取事实、决策和实体
- 📊 **统计信息**: 查看系统状态和使用统计
- 🎨 **友好输出**: 彩色终端输出，可配置详细级别

## 🚀 快速开始

### 安装

```bash
# 从源码构建
cd cortex-mem
cargo build --release --bin cortex-mem

# 或直接运行
cargo run --bin cortex-mem -- --help
```

### 基本使用

```bash
# 创建新会话
./cortex-mem session create tech-discussion --title "技术讨论"

# 添加消息
./cortex-mem add --thread tech-discussion "如何实现OAuth认证？"

# 搜索相关内容
./cortex-mem search "OAuth" --thread tech-discussion

# 提取记忆
./cortex-mem session extract tech-discussion

# 查看统计
./cortex-mem stats
```

## 📖 详细命令参考

### 会话管理命令

#### 创建会话

```bash
cortex-mem session create <thread-id> [--title <title>]

# 示例
cortex-mem session create project-planning --title "项目规划讨论"
cortex-mem session create 2024-01-15-review  # 无标题
```

#### 关闭会话

```bash
cortex-mem session close <thread-id>

# 示例
cortex-mem session close tech-discussion
```

#### 提取记忆

```bash
cortex-mem session extract <thread-id>

# 示例
cortex-mem session extract project-planning
```

#### 列出所有会话

```bash
cortex-mem session list
```

### 消息操作命令

#### 添加消息

```bash
cortex-mem add --thread <thread-id> [--role <role>] <content>

# 角色选项: user, assistant, system (默认: user)
cortex-mem add --thread tech-support --role user "忘记密码了怎么办？"
cortex-mem add --thread tech-support --role assistant "请访问重置密码页面..."
```

#### 搜索消息

```bash
cortex-mem search <query> [--thread <thread-id>] [-n <limit>] [-s <min-score>]

# 示例
cortex-mem search "密码"
cortex-mem search "OAUTH" -n 5 -s 0.7
cortex-mem search "API" --thread tech-support
```

#### 列出消息

```bash
cortex-mem list [--thread <thread-id>] [--dimension <dimension>]

# 示例
cortex-mem list
cortex-mem list --thread tech-support
cortex-mem list --dimension agent
```

#### 获取特定消息

```bash
cortex-mem get <uri>

# 示例
cortex-mem get cortex://session/tech-support/timeline/2024/01/15/14_30_00_abc123.md
```

#### 删除消息

```bash
cortex-mem delete <uri>
```

### 系统命令

#### 查看统计信息

```bash
cortex-mem stats
```

## ⚙️ 配置选项

### 数据目录

默认数据目录为 `./cortex-data`，可通过 `--data-dir` 参数自定义：

```bash
cortex-mem --data-dir /path/to/data session list
```

### 详细输出

使用 `--verbose` 或 `-v` 参数启用详细日志：

```bash
cortex-mem --verbose add --thread test "Hello"
```

### 配置文件

CLI遵循以下配置优先级：
1. 命令行参数
2. 环境变量
3. 配置文件 (config.toml)
4. 默认值

## 🌐 环境变量

可以通过环境变量覆盖配置：

```bash
export CORTEX_DATA_DIR="/custom/path"
export LLM_API_KEY="your-api-key"
export QDRANT_URL="http://localhost:6333"

cortex-mem session create test
```

## 📝 完整工作流示例

```bash
# 1. 创建会话
cortex-mem session create customer-support --title "客户支持会话"

# 2. 添加对话
cortex-mem add --thread customer-support "我的订单状态是什么？"
cortex-mem add --thread customer-support --role assistant "让我帮您查询订单状态..."

# 3. 搜索相关信息
cortex-mem search "订单" --thread customer-support

# 4. 提取记忆到用户档案
cortex-mem session extract customer-support

# 5. 查看提取的记忆
cortex-mem list --dimension user

# 6. 关闭会话
cortex-mem session close customer-support

# 7. 查看系统统计
cortex-mem stats
```

## 🎨 输出格式

CLI使用颜色编码以提高可读性：

- 🔵 **蓝色**: 会话ID和文件URI
- 🟢 **绿色**: 成功操作
- 🟡 **黄色**: 警告信息
- 🔴 **红色**: 错误信息
- ⚪ **白色**: 一般信息

## 🧪 脚本测试

项目包含测试脚本用于快速验证功能：

```bash
# 快速测试
./cortex-mem-cli/quick-test.sh

# 完整演示
./cortex-mem-cli/demo.sh
```

## 🔍 故障排除

### 常见问题

**数据目录权限错误**
```bash
chmod 755 ./cortex-data
```

**LLM服务不可用**
```bash
export LLM_API_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="your-key"
export LLM_MODEL="gpt-4"
```

**向量搜索失败**
```bash
# 启动Qdrant
docker run -p 6333:6333 qdrant/qdrant

# 配置连接
export QDRANT_URL="http://localhost:6333"
```

### 调试模式

```bash
# 启用详细日志查看调试信息
cortex-mem --verbose --log-level debug session create debug-test

# 查看完整错误堆栈
RUST_BACKTRACE=1 cortex-mem search "test"
```

## 🛣️ 路线图

计划中的功能：

- [ ] 批量操作命令
- [ ] 交互式模式
- [ ] 配置管理命令
- [ ] 导入/导出工具
- [ ] 自动补全支持
- [ ] 插件系统

## 📚 更多资源

- [Cortex Memory 主项目](../README.md)
- [核心库文档](../cortex-mem-core/README.md)
- [HTTP API服务](../cortex-mem-service/README.md)
- [架构概述](../../litho.docs/en/2.Architecture.md)

## 🤝 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 项目仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

MIT 许可证 - 详见 [LICENSE](../../LICENSE) 文件

---

**Built with ❤️ using Rust and the Cortex Memory Core**
```

接下来，我将继续为其他子crate创建或更新README文件。
