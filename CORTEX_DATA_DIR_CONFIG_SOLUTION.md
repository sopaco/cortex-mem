# 📁 Cortex Memory 数据目录配置方案

## 🎯 目标

为不同的应用场景提供合理的默认数据目录，同时保持灵活的配置能力：
1. **TARS 应用**：使用系统应用数据目录（符合操作系统最佳实践）
2. **其他工具**：支持灵活配置
3. **环境变量**：统一的覆盖机制

---

## 📊 配置优先级

数据目录按以下优先级确定（从高到低）：

```
1. 环境变量 CORTEX_DATA_DIR         ← 最高优先级（运行时覆盖）
2. 配置文件 config.toml 中的 data_dir  ← 用户显式配置
3. 应用默认路径                      ← 应用特定默认值
4. 回退路径 ./.cortex                ← 最低优先级
```

---

## 🏢 TARS 应用的默认路径

### 各操作系统的默认路径

| 操作系统 | 默认数据目录 |
|---------|------------|
| **macOS** | `~/Library/Application Support/com.cortex-mem.tars/cortex` |
| **Linux** | `~/.local/share/cortex-mem-tars/cortex` |
| **Windows** | `%APPDATA%\cortex-mem\tars\cortex` |

### 示例路径

```bash
# macOS
/Users/jiangmeng/Library/Application Support/com.cortex-mem.tars/cortex/
└── tenants/
    ├── bot-alice/
    │   └── cortex/
    │       ├── resources/
    │       ├── user/
    │       ├── agent/
    │       └── session/
    └── bot-bob/
        └── cortex/
            ├── resources/
            ├── user/
            ├── agent/
            └── session/

# Linux
/home/jiangmeng/.local/share/cortex-mem-tars/cortex/
└── tenants/
    └── ...

# Windows
C:\Users\jiangmeng\AppData\Roaming\cortex-mem\tars\cortex\
└── tenants/
    └── ...
```

### 为什么这样设计？

✅ **符合操作系统规范**：
- macOS: 遵循 Apple 文件系统规范
- Linux: 遵循 XDG Base Directory 规范
- Windows: 遵循 Windows 应用数据存储规范

✅ **不污染工作目录**：
- 应用数据与用户文件分离
- 避免在项目目录下创建隐藏文件夹

✅ **易于管理和备份**：
- 系统级的应用数据目录
- 用户清楚数据存储位置
- 便于备份和迁移

✅ **支持多用户**：
- 每个操作系统用户有独立的应用数据
- 不同用户的 TARS 数据互不影响

---

## 🔧 实现细节

### 1. cortex-mem-config 的默认值实现

**文件**: `cortex-mem-config/src/lib.rs`

```rust
impl Default for CortexConfig {
    fn default() -> Self {
        // 优先级：
        // 1. 环境变量 CORTEX_DATA_DIR
        // 2. 应用数据目录/cortex (如果是 TARS 应用)
        // 3. 当前目录 ./.cortex
        let data_dir = std::env::var("CORTEX_DATA_DIR")
            .ok()
            .or_else(|| {
                // 尝试使用应用数据目录（TARS 默认路径）
                directories::ProjectDirs::from("com", "cortex-mem", "tars")
                    .map(|dirs| {
                        let cortex_dir = dirs.data_dir().join("cortex");
                        cortex_dir.to_string_lossy().to_string()
                    })
            })
            .unwrap_or_else(|| "./.cortex".to_string());
        
        CortexConfig { data_dir }
    }
}
```

**说明**：
- 使用 `directories` crate 获取系统应用数据目录
- 在应用数据目录下创建 `cortex` 子目录
- 支持环境变量 `CORTEX_DATA_DIR` 覆盖
- 回退到 `./.cortex`（当无法获取应用数据目录时）

### 2. TARS 配置文件

**文件**: `examples/cortex-mem-tars/config.example.toml`

```toml
[cortex]
# Cortex Memory 数据目录
# 优先级：
#   1. 环境变量 CORTEX_DATA_DIR（最高优先级）
#   2. 本配置文件中的 data_dir
#   3. 应用数据目录/cortex（TARS 默认）
#      - macOS: ~/Library/Application Support/com.cortex-mem.tars/cortex
#      - Linux: ~/.local/share/cortex-mem-tars/cortex
#      - Windows: %APPDATA%\cortex-mem\tars\cortex
#   4. 当前目录 ./.cortex（最低优先级）
# 
# 留空或注释此行将使用默认值（应用数据目录）
# data_dir = "/path/to/custom/cortex/data"
```

**说明**：
- 配置文件中可以不指定 `data_dir`，将使用默认值
- 如果指定了，将覆盖默认值
- 清晰的注释说明优先级

---

## 📝 使用场景

### 场景1：TARS 用户（默认使用）

**无需任何配置**，直接使用：

```bash
# 启动 TARS
cargo run -p cortex-mem-tars

# 数据自动存储到：
# macOS: ~/Library/Application Support/com.cortex-mem.tars/cortex/
# Linux: ~/.local/share/cortex-mem-tars/cortex/
# Windows: %APPDATA%\cortex-mem\tars\cortex/
```

### 场景2：自定义数据目录（通过配置文件）

**编辑 config.toml**：

```toml
[cortex]
data_dir = "/Users/jiangmeng/Documents/my-cortex-data"
```

**启动应用**：

```bash
cargo run -p cortex-mem-tars

# 数据存储到：
# /Users/jiangmeng/Documents/my-cortex-data/
```

### 场景3：临时覆盖（通过环境变量）

**适用于测试或临时使用**：

```bash
# 使用环境变量覆盖
CORTEX_DATA_DIR=/tmp/cortex-test cargo run -p cortex-mem-tars

# 数据存储到：
# /tmp/cortex-test/
```

### 场景4：多环境切换

```bash
# 开发环境
CORTEX_DATA_DIR=./dev-data cargo run -p cortex-mem-tars

# 生产环境
CORTEX_DATA_DIR=/var/lib/cortex cargo run -p cortex-mem-tars --release

# 测试环境
CORTEX_DATA_DIR=/tmp/cortex-test cargo test
```

---

## 🔍 其他工具的配置

### cortex-mem-cli

**默认行为**：
- 使用当前目录 `./.cortex`（因为 CLI 工具通常在项目目录下运行）

**自定义**：
```bash
# 使用环境变量
CORTEX_DATA_DIR=/path/to/data cortex-mem-cli search "query"

# 或通过命令行参数（如果实现）
cortex-mem-cli --data-dir /path/to/data search "query"
```

### cortex-mem-service

**默认行为**：
- 使用配置文件中的 `cortex.data_dir`
- 如果未配置，使用 `./.cortex`

**推荐配置**：
```toml
[cortex]
# 服务器应该使用固定的数据目录
data_dir = "/var/lib/cortex-mem"
```

### cortex-mem-mcp

**默认行为**：
- 使用环境变量或配置文件
- 回退到 `./.cortex`

**推荐**：
```bash
# 通过环境变量指定
CORTEX_DATA_DIR=/path/to/mcp-data cortex-mem-mcp
```

---

## ✅ 优势总结

### 1. 符合最佳实践

| 方面 | 之前 | 现在 |
|------|------|------|
| **默认路径** | `./.cortex` | 系统应用数据目录 |
| **污染工作目录** | ❌ 是 | ✅ 否 |
| **符合OS规范** | ❌ 否 | ✅ 是 |
| **多用户支持** | ⚠️ 有限 | ✅ 完整 |

### 2. 用户体验

✅ **开箱即用**：
- TARS 用户无需任何配置
- 数据自动存储到合适的位置

✅ **清晰可见**：
- 用户知道数据存储在哪里
- 便于查找、备份和管理

✅ **灵活配置**：
- 支持配置文件自定义
- 支持环境变量覆盖
- 支持多环境切换

### 3. 开发体验

✅ **测试友好**：
```bash
# 测试时使用临时目录
CORTEX_DATA_DIR=/tmp/test cargo test
```

✅ **开发友好**：
```bash
# 开发时使用本地目录
CORTEX_DATA_DIR=./dev-data cargo run
```

✅ **部署友好**：
```toml
# 生产环境配置
[cortex]
data_dir = "/var/lib/cortex-mem"
```

---

## 🔄 迁移指南

### 从旧版本迁移

如果用户已经有数据在 `./.cortex`，可以选择：

**选项1：继续使用旧路径**（配置文件）

```toml
[cortex]
data_dir = "./.cortex"
```

**选项2：迁移到新路径**

```bash
# macOS 示例
cp -r ./.cortex ~/Library/Application\ Support/com.cortex-mem.tars/cortex

# 然后删除旧数据（可选）
rm -rf ./.cortex
```

**选项3：使用环境变量**

```bash
# 临时使用旧路径
CORTEX_DATA_DIR=./.cortex cargo run -p cortex-mem-tars
```

---

## 📋 检查清单

实施此方案后，确认以下几点：

- ✅ `cortex-mem-config` 添加 `directories` 依赖
- ✅ `CortexConfig::default()` 实现新的优先级逻辑
- ✅ `config.example.toml` 更新注释说明
- ✅ 编译成功（所有包）
- ✅ 文档更新（README, 快速开始等）
- ✅ 测试通过

---

## 🎊 总结

### 核心改进

1. **默认路径优化**：从 `./.cortex` 改为系统应用数据目录
2. **配置优先级**：环境变量 > 配置文件 > 应用默认 > 回退路径
3. **符合规范**：遵循各操作系统的最佳实践
4. **保持灵活**：支持多种配置方式

### 各平台默认路径

| 平台 | 默认路径 |
|------|---------|
| macOS | `~/Library/Application Support/com.cortex-mem.tars/cortex` |
| Linux | `~/.local/share/cortex-mem-tars/cortex` |
| Windows | `%APPDATA%\cortex-mem\tars\cortex` |

### 配置方式

```
优先级：环境变量 > 配置文件 > 默认值

1. CORTEX_DATA_DIR=...           ← 最灵活
2. config.toml: data_dir = "..."  ← 持久化配置
3. 应用数据目录/cortex            ← 推荐默认值
4. ./.cortex                      ← 回退值
```

---

**方案创建时间**: 2026-02-09 16:45  
**状态**: ✅ 已实施并编译成功  
**推荐度**: ⭐⭐⭐⭐⭐  
**适用**: TARS 及所有 Cortex Memory 应用
