# Cortex-Mem CLI 数据目录说明

## 📂 数据目录的确定方式

Cortex-Mem CLI **不需要在记忆目录下执行**，它通过以下优先级自动确定数据目录：

### 优先级顺序（从高到低）

```
1. config.toml 中的 [cortex] data_dir 配置
   ↓ (如果未配置)
2. 环境变量 CORTEX_DATA_DIR
   ↓ (如果未设置)
3. 系统应用数据目录/cortex
   - macOS: ~/Library/Application Support/cortex-mem.tars/cortex
   - Linux: ~/.local/share/cortex-mem.tars/cortex
   - Windows: %APPDATA%\cortex-mem\tars\cortex
   ↓ (如果无法获取)
4. 当前工作目录下的 ./.cortex
```

---

## 🛠️ 指定数据目录的三种方式

### 方式 1️⃣: 通过 `config.toml` 配置（推荐）

编辑 `config.toml`，添加或修改 `[cortex]` 段：

```toml
[cortex]
data_dir = "/path/to/your/cortex-data"
```

**示例**:
```toml
[cortex]
data_dir = "/Users/yourname/Documents/cortex-memory"
```

**优点**: 
- ✅ 配置固定，不受工作目录影响
- ✅ 团队成员可以共享配置模板
- ✅ 支持绝对路径和相对路径

---

### 方式 2️⃣: 通过环境变量

```bash
# 临时设置（仅当前会话）
export CORTEX_DATA_DIR="/path/to/your/cortex-data"

# 永久设置（添加到 ~/.zshrc 或 ~/.bashrc）
echo 'export CORTEX_DATA_DIR="/path/to/your/cortex-data"' >> ~/.zshrc
source ~/.zshrc
```

**优点**: 
- ✅ 不修改配置文件
- ✅ 可以快速切换不同的数据目录
- ✅ 适合脚本和 CI/CD 环境

---

### 方式 3️⃣: 使用默认目录（无需配置）

如果不做任何配置，CLI 会自动使用：
- **TARS 桌面应用**: 系统应用数据目录
- **CLI 工具**: 当前工作目录下的 `./.cortex`

**示例**:
```bash
# 在项目根目录执行
cd /path/to/my-project
cortex-mem-cli layers status
# → 数据目录: /path/to/my-project/.cortex
```

---

## 📋 完整示例

### 示例 1: 使用环境变量指定数据目录

```bash
# 设置数据目录
export CORTEX_DATA_DIR="/Users/jiangmeng/my-cortex-data"

# 在任意目录执行 CLI
cd /tmp
cargo run -p cortex-mem-cli -- layers status
# → 读取目录: /Users/jiangmeng/my-cortex-data/default

# 查看指定会话
cargo run -p cortex-mem-cli -- list -u cortex://session/abc123
# → 访问文件: /Users/jiangmeng/my-cortex-data/default/session/abc123
```

---

### 示例 2: 使用 config.toml 指定数据目录

**config.toml**:
```toml
[cortex]
data_dir = "./my-memories"

[qdrant]
url = "http://localhost:6334"
collection_name = "cortex-mem-v2"
# ... 其他配置
```

**执行**:
```bash
# 在 config.toml 所在目录执行
cargo run -p cortex-mem-cli -- layers ensure-all
# → 数据目录: ./my-memories/default
```

---

### 示例 3: 使用默认目录（当前目录 .cortex）

```bash
# 不设置任何配置
cd /path/to/my-project

# 生成测试数据（会创建 ./.cortex 目录）
bash scripts/create_test_data.sh

# 查看状态
cargo run -p cortex-mem-cli -- layers status
# → 数据目录: /path/to/my-project/.cortex/default
```

---

## 🏢 租户（Tenant）参数

CLI 还支持通过 `--tenant` 参数指定租户 ID，用于多租户隔离：

```bash
# 使用默认租户（default）
cargo run -p cortex-mem-cli -- layers status

# 使用自定义租户
cargo run -p cortex-mem-cli -- --tenant my-team layers status
# → 数据目录: /path/to/data/my-team
```

---

## 📁 最终数据目录结构

假设数据目录为 `/data/cortex`，租户为 `default`：

```
/data/cortex/
└── default/                 ← 租户目录
    ├── session/             ← 会话维度
    │   └── abc123/
    │       ├── .session.json
    │       └── timeline/
    │           └── 2026-02/
    │               └── 25/
    │                   ├── .abstract.md
    │                   ├── .overview.md
    │                   └── 10_30_45_abc.md
    ├── user/                ← 用户维度
    │   └── user-001/
    │       └── preferences/
    │           ├── .abstract.md
    │           ├── .overview.md
    │           └── pref_0.md
    ├── agent/               ← Agent 维度
    │   └── agent-001/
    │       └── cases/
    │           ├── .abstract.md
    │           ├── .overview.md
    │           └── case_0.md
    └── resources/           ← 资源维度
        └── docs/
            ├── .abstract.md
            ├── .overview.md
            └── api_doc.md
```

---

## ✅ 总结

### ❓ 需要在记忆目录下执行 CLI 吗？

**答案**: **不需要！**

CLI 可以在任意目录执行，数据目录由配置决定，不受工作目录影响。

### 🎯 推荐做法

| 场景 | 推荐方式 | 原因 |
|------|----------|------|
| 开发测试 | 环境变量 `CORTEX_DATA_DIR` | 灵活切换，不污染项目 |
| 生产部署 | `config.toml` 配置 | 固定路径，配置统一 |
| 快速试用 | 默认目录 `./.cortex` | 零配置，即开即用 |
| 多租户 | `--tenant` 参数 | 数据隔离，权限清晰 |

### 🚀 快速开始

```bash
# 1. 设置数据目录（可选）
export CORTEX_DATA_DIR="/path/to/your/data"

# 2. 生成测试数据
bash scripts/create_test_data.sh

# 3. 查看层级文件状态
cargo run -p cortex-mem-cli -- layers status

# 4. 生成缺失的 L0/L1 文件
cargo run -p cortex-mem-cli -- layers ensure-all

# 5. 查看会话列表
cargo run -p cortex-mem-cli -- session list
```

---

**完整配置示例**: 参考 `config.toml` 文件
**测试脚本**: 参考 `scripts/create_test_data.sh`
