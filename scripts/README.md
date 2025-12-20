# Cargo.toml 版本更新工具

这是一个用于统一更新 `cortex-mem` 项目中所有 `Cargo.toml` 文件版本的 JavaScript 工具。

## 功能

- 扫描项目中所有的 `Cargo.toml` 文件
- 更新每个 crate 的版本号为指定值（默认为 1.0.0）
- 自动更新内部依赖引用的版本号
- 排除 `target`、`node_modules` 和 `.git` 目录

## 使用方法

1. 打开终端或命令提示符
2. 导航到项目的scripts目录:
   ```bash
   cd scripts
   ```

3. 安装依赖:
   ```bash
   npm install
   ```

4. 运行脚本:
   ```bash
   npm run update-versions
   ```
   或
   ```bash
   node update-versions.js
   ```

## 自定义版本

要更新为不同的版本号，编辑 `update-versions.js` 文件顶部的 `VERSION` 常量:

```javascript
const VERSION = '2.0.0'; // 更改为你想要的版本号
```

## 示例输出

```
==================================================
Cargo.toml Version Updater
Updating all versions to 1.0.0
==================================================
Scanning for Cargo.toml files...
Found 11 Cargo.toml files

Updating package versions...
  Updated version in Cargo.toml
  Updated version in cortex-mem-cli/Cargo.toml
  ...

Updating internal dependencies...
  Updated internal dependencies in cortex-mem-config/Cargo.toml
  ...

==================================================
Update Summary:
  11 package versions updated
  3 dependency references updated
==================================================

Version update completed successfully!
You may want to run "cargo check" to verify all changes.
```

## 注意事项

1. 脚本会在更新前自动备份原始文件的内容，但建议在运行前手动版本控制或备份
2. 更新完成后，建议运行 `cargo check` 或 `cargo build` 验证所有更改