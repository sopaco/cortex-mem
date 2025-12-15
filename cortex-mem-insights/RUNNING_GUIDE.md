# cortex-mem-insights 运行指南

## 快速启动

### 1. 启动 API 服务

```bash
cd cortex-mem-insights
bun run start-api.js
```

**预期输出**：
```
🚀 cortex-mem-insights API 运行在 http://localhost:3001
```

### 2. 启动前端开发服务器

```bash
cd cortex-mem-insights
bun run start-dev.js
```

**预期输出**：
```
  VITE v5.4.21  ready in 754 ms

  ➜  Local:   http://localhost:5173/
  ➜  Network: use --host to expose
```

### 3. 访问应用

打开浏览器访问：[http://localhost:5173](http://localhost:5173)

## 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    浏览器 (Frontend)                         │
│                    http://localhost:5173                      │
└─────────────────────────────────────────────────────────────┘
                                    ▲
                                    │ HTTP 请求
                                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    Vite 开发服务器                           │
│                    (代理 API 请求)                           │
└─────────────────────────────────────────────────────────────┘
                                    ▲
                                    │ 代理到 /api →
┌─────────────────────────────────────────────────────────────┐
│                    cortex-mem-insights API                    │
│                    http://localhost:3001                      │
│                    (Elysia 服务器)                            │
└─────────────────────────────────────────────────────────────┘
                                    ▲
                                    │ 尝试连接 →
┌─────────────────────────────────────────────────────────────┐
│                    cortex-mem-service                         │
│                    http://localhost:3000                      │
│                    (可选，如不可用则使用 Mock 数据)            │
└─────────────────────────────────────────────────────────────┘
```

## 运行模式

### 模式 1：纯 Mock 模式（开发推荐）

**特点**：
- 不需要实际的 cortex-mem-service
- 使用内置的 Mock 数据
- 快速启动，适合开发和测试

**启动命令**：
```bash
# 默认行为（自动使用 Mock 数据）
bun run start-api.js
```

**环境变量**：
```bash
export MOCK_CORTEX_MEM=true  # 强制使用 Mock 数据
export CORTEX_MEM_SERVICE_URL=http://localhost:3000  # 设置服务地址（如需要）
```

### 模式 2：混合模式（生产推荐）

**特点**：
- 尝试连接实际的 cortex-mem-service
- 如连接失败，自动回退到 Mock 数据
- 确保应用始终可用

**启动命令**：
```bash
export MOCK_CORTEX_MEM=false  # 尝试使用实际服务
export CORTEX_MEM_SERVICE_URL=http://production-service:3000  # 实际服务地址
bun run start-api.js
```

## API 接口

### 1. 健康检查

```bash
curl http://localhost:3001/health
```

**响应**：
```json
{
  "status": "healthy",
  "vector_store": true,
  "llm_service": true,
  "timestamp": "2025-12-15T03:03:52.966Z",
  "service": "cortex-mem-insights-api",
  "mock_mode": true  // 仅在 Mock 模式下出现
}
```

### 2. 记忆列表

```bash
curl http://localhost:3001/api/memories
curl http://localhost:3001/api/memories?limit=10
curl http://localhost:3001/api/memories?user_id=SkyronJ
```

**响应**：
```json
{
  "total": 42,
  "memories": [
    {
      "id": "023f3938-9d1f-42e8-a70d-d9ddf9e27bf0",
      "content": "用户SkyronJ确认其在2026年1月将有一笔收入...",
      "metadata": {
        "user_id": "SkyronJ",
        "agent_id": null,
        "run_id": null,
        "actor_id": null,
        "role": null,
        "memory_type": "Personal",
        "hash": "fd8777390ba83d10ad1a621094829fcae26f5f8aac3b05b2c2b07e44177093ae",
        "custom": {}
      },
      "created_at": "2025-12-12T08:36:49.038512+00:00",
      "updated_at": "2025-12-12T08:36:49.038512+00:00"
    }
  ]
}
```

### 3. 记忆搜索

```bash
curl -X POST http://localhost:3001/api/memories/search \
  -H "Content-Type: application/json" \
  -d '{"query":"SkyronJ","limit":2}'
```

**响应**：
```json
{
  "total": 2,
  "results": [
    {
      "memory": {
        "id": "7f27afaa-6789-4f65-9014-8781aeeb6cc0",
        "content": "SkyronJ, born in 1988.",
        "metadata": {
          "user_id": "demo_user",
          "agent_id": null,
          "run_id": null,
          "actor_id": null,
          "role": null,
          "memory_type": "Factual",
          "hash": "4f4103c9325230c4752effc7f752816776faf4f32098e9c211e6bee0a15242f4",
          "custom": {
            "keywords": "[\"SkyronJ\",\"born\",\"1988\"]"
          }
        },
        "created_at": "2025-12-09T11:46:02.161812+00:00",
        "updated_at": "2025-12-09T11:46:02.161812+00:00"
      },
      "score": 0.7279024
    }
  ]
}
```

## 前端路由

### 1. 仪表盘（主页）

**URL**：[http://localhost:5173/](http://localhost:5173/)

**功能**：
- 显示系统状态
- 显示记忆统计
- 显示最近记忆
- 实时数据加载

### 2. 记忆浏览器

**URL**：[http://localhost:5173/memories](http://localhost:5173/memories)

**功能**：
- 浏览所有记忆
- 搜索和过滤记忆
- 排序和分页
- 详细记忆信息

### 3. 分析页面

**URL**：[http://localhost:5173/analytics](http://localhost:5173/analytics)

**功能**：
- 记忆统计分析
- 用户分布
- 类型分布
- 时间趋势

### 4. 优化页面

**URL**：[http://localhost:5173/optimization](http://localhost:5173/optimization)

**功能**：
- 记忆优化操作
- 优化历史
- 优化统计
- 批量操作

### 5. 监控页面

**URL**：[http://localhost:5173/monitor](http://localhost:5173/monitor)

**功能**：
- 系统健康监控
- 服务状态
- 日志查看
- 资源使用

## 常见问题排查

### 问题 1：端口被占用

**症状**：服务无法启动，报端口已被占用错误

**解决方案**：
```bash
# 查找占用端口的进程
lsof -i :3001
lsof -i :5173

# 杀死占用端口的进程
kill -9 <PID>

# 或者使用不同的端口
PORT=3002 bun run start-api.js
```

### 问题 2：依赖缺失

**症状**：报找不到模块或依赖错误

**解决方案**：
```bash
# 安装依赖
bun install

# 或者
npm install
```

### 问题 3：前端编译错误

**症状**：Vite 报编译错误

**解决方案**：
```bash
# 检查具体错误信息
bun run dev

# 清除缓存
rm -rf node_modules/.vite
rm -rf .svelte-kit

# 重新安装依赖
bun install
```

### 问题 4：API 连接失败

**症状**：前端无法加载数据

**解决方案**：
```bash
# 检查 API 服务是否运行
curl http://localhost:3001/health

# 检查 Vite 代理配置
# 在 vite.config.ts 中确认以下配置：
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:3001',
      changeOrigin: true,
      secure: false
    }
  }
}

# 重启 API 服务
bun run start-api.js
```

### 问题 5：CORS 错误

**症状**：浏览器报 CORS 错误

**解决方案**：
```bash
# 检查 API 服务的 CORS 配置
# 在 src/server/index.ts 中确认以下配置：
app.use(cors({
  origin: ['http://localhost:5173', 'http://localhost:3000'],
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Authorization']
}))

# 确保前端地址在允许的 origin 列表中
```

## 高级配置

### 1. 自定义端口

```bash
# API 服务
PORT=3002 bun run start-api.js

# 前端服务
PORT=5174 bun run start-dev.js
```

### 2. 生产构建

```bash
# 构建生产版本
bun run build

# 预览生产版本
bun run preview
```

### 3. Docker 部署

```dockerfile
# 示例 Dockerfile
FROM oven/bun:1

WORKDIR /app
COPY . .

RUN bun install

EXPOSE 3001
EXPOSE 5173

CMD ["bun", "run", "start-api.js"]
```

## 测试指南

### 1. 运行测试

```bash
# 运行 Mock 数据测试
bun run test-mock-data.js

# 打开前端测试页面
open test-frontend.html

# 打开 API 连接测试页面
open test-api-connection.html
```

### 2. 手动测试

1. **打开仪表盘**：[http://localhost:5173/](http://localhost:5173/)
2. **检查系统状态**：确保所有服务显示"已连接"
3. **浏览记忆**：点击"记忆浏览器"查看记忆列表
4. **搜索功能**：尝试搜索"SkyronJ"或"Rust"
5. **过滤功能**：尝试不同的记忆类型过滤

### 3. API 测试

```bash
# 测试健康检查
curl http://localhost:3001/health

# 测试记忆列表
curl http://localhost:3001/api/memories | jq '.total'

# 测试搜索
curl -X POST http://localhost:3001/api/memories/search \
  -H "Content-Type: application/json" \
  -d '{"query":"Rust"}' | jq '.total'
```

## 性能优化

### 1. 启用缓存

```javascript
// 在 API 路由中添加缓存
app.use(memoryRoutes)
  .use(cache({ 
    maxAge: 60, // 60 秒缓存
    exclude: [
      '/api/memories/search', // 不缓存搜索结果
      '/api/system/*' // 不缓存系统接口
    ]
  }))
```

### 2. 分页加载

```javascript
// 在前端实现分页
const pageSize = 20;
const currentPage = 1;

// 加载指定页的数据
const response = await api.memory.list({
  limit: pageSize,
  page: currentPage
});
```

### 3. 懒加载

```svelte
<!-- 在 Svelte 组件中实现懒加载 -->
{#if shouldLoadMore}
  <button on:click={loadMore}>加载更多</button>
{/if}
```

## 日志和监控

### 1. 日志级别

```bash
# 设置日志级别
LOG_LEVEL=debug bun run start-api.js
```

### 2. 请求日志

```javascript
// 添加请求日志中间件
app.use(logger({
  level: 'info',
  format: ':method :path :status :response-time ms'
}))
```

### 3. 错误监控

```javascript
// 添加错误监控
app.onError(({ code, error, request }) => {
  console.error(`[${new Date().toISOString()}] [${code}] ${request.method} ${request.path}:`, error.message);
  // 可以集成 Sentry 或其他错误监控服务
  // Sentry.captureException(error);
});
```

## 更新和维护

### 1. 更新依赖

```bash
# 更新所有依赖
bun upgrade

# 更新指定依赖
bun add elysia@latest
```

### 2. 添加新功能

```bash
# 添加新的 API 路由
# 1. 创建新的 API 文件：src/server/api/new-feature.ts
# 2. 在 src/server/index.ts 中注册路由：
import { newFeatureRoutes } from './api/new-feature';
app.use(newFeatureRoutes)

# 添加新的前端页面
# 1. 创建新的 Svelte 页面：src/routes/new-feature/+page.svelte
# 2. 添加导航链接
```

### 3. 代码风格

```bash
# 运行代码格式化
bun run format

# 运行代码检查
bun run lint
```

## 安全最佳实践

### 1. 环境变量

```bash
# 使用环境变量管理敏感信息
# 创建 .env 文件
echo "API_KEY=your-secret-key" > .env
echo "DATABASE_URL=your-db-url" >> .env

# 在代码中使用
const apiKey = process.env.API_KEY;
```

### 2. HTTPS

```bash
# 使用 HTTPS
# 在生产环境中使用反向代理（Nginx, Caddy 等）
# 或者直接配置 HTTPS
import { app } from './src/server/index.js';
import fs from 'fs';

const options = {
  key: fs.readFileSync('ssl/key.pem'),
  cert: fs.readFileSync('ssl/cert.pem')
};

app.listen(443, options, () => {
  console.log('🚀 HTTPS server running on port 443');
});
```

### 3. 速率限制

```javascript
// 添加速率限制
import { rateLimit } from 'elysia-rate-limit';

app.use(rateLimit({
  max: 100, // 每分钟最大请求数
  windowMs: 60 * 1000,
  message: 'Too many requests, please try again later.'
}))
```

## 社区和支持

### 1. 获取帮助

- 检查文档和示例
- 查看错误日志
- 尝试最小可复现示例
- 搜索相关问题

### 2. 贡献代码

- Fork 项目
- 创建功能分支
- 提交拉取请求
- 遵循代码风格
- 添加测试用例

### 3. 报告问题

- 提供详细的错误信息
- 描述复现步骤
- 包含环境信息
- 添加日志和截图

## 版本历史

### v1.0.0（当前版本）

- ✅ 解决了"未找到记忆记录"问题
- ✅ 添加了 Mock 数据支持
- ✅ 实现了自动回退机制
- ✅ 增强了错误处理
- ✅ 添加了完整的测试套件
- ✅ 更新了文档和示例

### 未来版本计划

- **v1.1.0**：添加实际数据集成（重要性分数、优化统计等）
- **v1.2.0**：添加缓存和性能优化
- **v1.3.0**：添加用户认证和授权
- **v2.0.0**：添加多语言支持和国际化

## 许可证

本项目采用 MIT 许可证。详情请参阅 LICENSE 文件。

## 联系方式

如有任何问题或建议，请通过以下方式联系：

- GitHub Issues: [https://github.com/sopaco/cortex-mem/issues](https://github.com/sopaco/cortex-mem/issues)
- 电子邮件: [support@cortex-mem.example.com](mailto:support@cortex-mem.example.com)

---

> "cortex-mem-insights 是一个强大的 AI 记忆管理可视化工具，旨在帮助开发者和用户轻松管理和分析 AI 记忆数据。"

**状态**：准备就绪 🚀