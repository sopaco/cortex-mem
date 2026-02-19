# Cortex Memory CLI

一个基于文件系统的AI Agent记忆管理命令行工具。

## 快速开始

### 1. 构建

```bash
cd cortex-mem
cargo build --release --bin cortex-mem
```

### 2. 运行

```bash
# 开发模式（推荐用于测试）
cargo run --bin cortex-mem -- --help

# 或使用构建的二进制
./target/release/cortex-mem --help
```

### 3. 快速测试

```bash
# 运行自动化测试脚本
./cortex-mem-cli/quick-test.sh

# 或手动测试
cargo run --bin cortex-mem -- session create my-session
cargo run --bin cortex-mem -- add --thread my-session "Hello!"
cargo run --bin cortex-mem -- stats
```

## 核心命令

### 📝 会话管理
```bash
# 创建会话
cortex-mem session create <thread-id> [--title <title>]

# 关闭会话
cortex-mem session close <thread-id>

# 提取记忆
cortex-mem session extract <thread-id>

# 列出所有会话
cortex-mem session list
```

### ✉️ 消息操作
```bash
# 添加消息
cortex-mem add --thread <thread-id> [--role user|assistant|system] <content>

# 搜索消息
cortex-mem search <query> [--thread <thread-id>] [-n <limit>] [-s <min-score>]

# 列出消息
cortex-mem list [--thread <thread-id>] [--dimension <dimension>]

# 获取消息
cortex-mem get <uri>

# 删除消息
cortex-mem delete <uri>
```

### 📊 统计信息
```bash
# 查看系统统计
cortex-mem stats
```

## 配置

### 数据目录

默认数据目录为 `./cortex-data`，可通过参数自定义：

```bash
cortex-mem --data-dir /path/to/data stats
```

### 详细日志

```bash
cortex-mem --verbose add --thread test "Hello"
```

## 示例

### 完整工作流

```bash
# 1. 创建会话
cortex-mem session create tech-discussion --title "技术讨论"

# 2. 添加对话
cortex-mem add --thread tech-discussion "如何实现OAuth?"
cortex-mem add --thread tech-discussion --role assistant "建议使用OAuth 2.0"

# 3. 搜索
cortex-mem search "OAuth" --thread tech-discussion

# 4. 提取记忆
cortex-mem session extract tech-discussion

# 5. 关闭会话
cortex-mem session close tech-discussion
```

## 文档

- [详细测试指南](./TESTING_GUIDE.md)
- [Phase 4 实现报告](../cortex-mem-2-planning/phase4-part1-cli-report.md)

## 特性

- ✅ 会话生命周期管理
- ✅ 时间轴消息存储
- ✅ 智能检索（基于L0/L1层）
- ✅ 记忆提取
- ✅ 彩色友好输出
- ✅ 完整错误处理

## 技术栈

- **Rust 2021**
- **clap 4.5** - CLI框架
- **colored 2.1** - 彩色输出
- **tokio** - 异步运行时
- **cortex-mem-core** - 核心库

## License

MIT
