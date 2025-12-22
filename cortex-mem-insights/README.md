# cortex-mem-insights

Web-based insights and management tool for cortex-mem.

## 项目结构

```
cortex-mem-insights/
├── src/
│   ├── routes/              # SvelteKit 路由（Web 页面）
│   │   ├── +page.svelte    # 首页
│   │   ├── memories/       # 记忆管理页面
│   │   ├── optimization/   # 优化面板页面
│   │   └── analytics/      # 分析页面
│   ├── lib/
│   │   └── api/            # 前端 API 客户端
│   └── server/
│       ├── api/            # Bun service API 路由
│       └── integrations/   # 与 cortex-mem-service 的集成
├── start.js                # 开发环境启动脚本
├── start-prod.js           # 生产环境启动脚本
├── start-api.js            # API 服务器启动脚本
└── start-dev.js            # Web 开发服务器启动脚本
```

## 架构说明

cortex-mem-insights 是一个独立的 Web 应用，包含：

1. **Web 前端** (SvelteKit)
   - 提供用户界面
   - 记忆管理、优化、分析等功能

2. **API 服务** (Bun + Elysia)
   - 作为 API 聚合层
   - 提供业务逻辑、缓存、权限控制等
   - 调用 cortex-mem-service 的 HTTP API

3. **依赖服务**
   - cortex-mem-service (端口 3000) - 核心服务
   - Qdrant (端口 6334) - 向量数据库
   - LLM 服务 - 大语言模型

## 快速开始

### 开发环境

```bash
# 安装依赖
bun install

# 启动完整服务（API + Web）
bun start

# 或分别启动
bun start:api   # 启动 API 服务器 (localhost:3001)
bun start:web   # 启动 Web 开发服务器 (localhost:5173)
```

访问 http://localhost:5173 查看 Web 界面

### 生产环境

```bash
# 1. 构建项目
bun run build

# 2. 启动生产服务
bun start:prod
```

访问 http://localhost:3000 查看 Web 界面

## 环境变量

创建 `.env` 文件：

```bash
# cortex-mem-service 地址
CORTEX_MEM_SERVICE_URL=http://localhost:3000

# Web 服务器端口（生产环境）
PORT=3000
HOST=0.0.0.0

# API 服务器端口
API_PORT=3001
```

## 部署

cortex-mem-insights 支持多种部署方式,详见 [DEPLOYMENT.md](./DEPLOYMENT.md)。

### 快速部署 (推荐)

使用独立可执行文件部署:

```bash
# 1. 构建独立可执行文件
bun run build:executable

# 2. 运行
./dist/cortex-mem-insights
```

优点:
- ✅ 单文件分发,包含所有依赖
- ✅ 无需安装 Node.js 或 Bun
- ✅ 启动速度快
- ✅ 跨平台支持

### 其他部署方式

- **Bun 运行时**: 适合开发环境和频繁更新
- **Docker 容器**: 适合云环境和 Kubernetes
- **静态托管**: 前后端分离,CDN 加速

详细说明请参考 [DEPLOYMENT.md](./DEPLOYMENT.md)。

## 开发指南

### 添加新页面

1. 在 `src/routes/` 下创建新目录
2. 添加 `+page.svelte` 文件
3. 使用 `src/lib/api/client.ts` 中的 API 客户端

### 添加新 API

1. 在 `src/server/api/` 下创建新路由文件
2. 在 `src/server/index.ts` 中注册路由
3. 在 `src/lib/api/client.ts` 中添加客户端方法

## 技术栈

- **前端**: SvelteKit, TailwindCSS, Chart.js
- **后端**: Bun, Elysia
- **构建**: Vite
- **部署**: Node.js adapter

## 许可证

MIT
