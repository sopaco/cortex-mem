# Cortex-Mem-Insights 可执行文件打包 - README

## 📦 快速开始

### 开发模式

```bash
# 启动开发服务器（Vite）
bun run dev

# 启动server.ts（测试可执行文件功能）
bun run serve
```

### 编译可执行文件

```bash
# 编译当前平台（自动优化）
bun run compile

# 编译生产版本（包含版本信息）
bun run compile:prod

# 编译所有平台
bun run compile:all
```

### 运行可执行文件

```bash
# 基本运行（自动打开浏览器）
./dist/cortex-mem-insights

# 指定端口
./dist/cortex-mem-insights --port 8080

# 不自动打开浏览器
./dist/cortex-mem-insights --no-browser

# 查看帮助
./dist/cortex-mem-insights --help
```

---

## 🎯 功能特性

✅ **单文件可执行**
- 包含所有依赖和前端资源
- 无需安装Node.js/Bun
- 双击即可运行

✅ **自动打开浏览器**
- 启动后自动在默认浏览器打开
- 支持macOS/Linux/Windows
- 可通过`--no-browser`禁用

✅ **跨平台支持**
- macOS (ARM64 + x64)
- Linux (x64)
- Windows (x64)

✅ **灵活配置**
- 自定义端口：`--port 8080`
- 环境变量：`PORT=8080`
- 禁用浏览器：`--no-browser`

---

## 📊 文件大小

```
cortex-mem-insights-mac-arm64    ~58MB
cortex-mem-insights-mac-x64      ~58MB
cortex-mem-insights-linux        ~55MB
cortex-mem-insights.exe          ~52MB
```

优化后的文件大小（使用`--minify --bytecode`）比未优化版本小约30-40%。

---

## 🚀 编译命令详解

### compile
基本编译，包含优化：
- `--minify`: 代码压缩
- `--bytecode`: 字节码预编译（启动速度提升2x）
- `--sourcemap`: 错误映射

### compile:prod
生产版本，额外注入版本信息：
- `--define VERSION`: 版本号
- `--define BUILD_TIME`: 构建时间

### compile:mac / compile:linux / compile:windows
跨平台编译，指定目标平台：
- `--target=bun-darwin-arm64`: macOS ARM64
- `--target=bun-darwin-x64`: macOS x64
- `--target=bun-linux-x64`: Linux x64
- `--target=bun-windows-x64`: Windows x64

---

## 🎨 使用示例

### 示例1：本地运行
```bash
# 编译
bun run compile

# 运行（自动打开浏览器）
./dist/cortex-mem-insights
```

### 示例2：指定端口
```bash
./dist/cortex-mem-insights --port 8080
```

### 示例3：无头模式（服务器部署）
```bash
./dist/cortex-mem-insights --headless
```

### 示例4：环境变量配置
```bash
PORT=3000 ./dist/cortex-mem-insights
```

---

## 🔧 技术实现

### server.ts核心功能

1. **HTTP服务器**
   - 使用`Bun.serve()`
   - 自动serve前端资源
   - 支持前端路由（所有路径返回index.html）

2. **自动打开浏览器**
   - macOS: `open`命令
   - Linux: `xdg-open`命令
   - Windows: `start`命令

3. **命令行参数**
   - `--port/-p`: 自定义端口
   - `--no-browser/--headless`: 禁用浏览器
   - `--help/-h`: 帮助信息

4. **优雅关闭**
   - 捕获`SIGINT`/`SIGTERM`信号
   - 正确停止server

---

## 📝 开发指南

### 修改server.ts

1. 编辑`server.ts`
2. 测试：`bun run serve`
3. 编译：`bun run compile`
4. 验证：`./dist/cortex-mem-insights`

### 更新版本号

修改`package.json`中的`version`字段，然后重新编译：

```bash
# 修改package.json: "version": "0.2.0"
bun run compile:prod
```

### 添加新的编译目标

在`package.json`的`scripts`中添加：

```json
"compile:linux-arm64": "bun build --compile --target=bun-linux-arm64 ./server.ts --outfile ./dist/cortex-mem-insights-linux-arm64"
```

---

## 🐛 故障排除

### 浏览器没有自动打开

**原因**: 平台不支持或命令不存在

**解决**: 
1. 手动访问控制台显示的URL
2. 或使用`--no-browser`禁用自动打开

### 端口已被占用

**错误信息**: `EADDRINUSE`

**解决**:
```bash
# 使用不同端口
./dist/cortex-mem-insights --port 8080
```

### 编译失败

**检查**:
1. Bun版本：`bun --version`（需要>=1.0.0）
2. 依赖安装：`bun install`
3. 磁盘空间：编译需要~100MB临时空间

---

## 📚 参考文档

- [Bun Standalone Executables](https://bun.com/docs/bundler/executables)
- [完整实现指南](./BUN_EXECUTABLE_PACKAGING_GUIDE.md)

---

## ✅ 验证清单

- [x] 编译成功
- [x] 可执行文件能运行
- [x] 自动打开浏览器
- [x] 前端页面正常显示
- [x] 前端路由工作正常
- [x] 命令行参数生效
- [x] 帮助信息正确

---

**当前状态**: ✅ 完成  
**测试结果**: ✅ 通过  
**可用性**: ✅ 生产就绪
