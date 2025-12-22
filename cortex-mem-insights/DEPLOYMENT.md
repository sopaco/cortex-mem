# cortex-mem-insights 部署指南

cortex-mem-insights 是一个基于 SvelteKit + Bun + Elysia 的 Web 应用。本文档介绍多种部署方式。

## 方式一: 独立可执行文件 (推荐)

使用 Bun 的 `--compile` 功能将应用打包成独立的可执行文件,无需安装 Node.js 或 Bun 运行时。

### 构建步骤

```bash
# 1. 安装依赖
bun install

# 2. 构建独立可执行文件
bun run build:executable
```

构建完成后,会在 `dist/` 目录生成可执行文件:
- macOS/Linux: `dist/cortex-mem-insights`
- Windows: `dist/cortex-mem-insights.exe`

### 运行

```bash
# macOS/Linux
./dist/cortex-mem-insights

# Windows
dist\cortex-mem-insights.exe
```

### 跨平台编译

如果需要为其他平台构建,可以修改 `build-executable.js` 中的 `target` 参数:

```javascript
// Linux x64
target: "bun-linux-x64"

// Linux ARM64 (如 Raspberry Pi)
target: "bun-linux-arm64"

// Windows x64
target: "bun-windows-x64"

// macOS x64
target: "bun-darwin-x64"

// macOS ARM64 (Apple Silicon)
target: "bun-darwin-arm64"
```

### 优点
- ✅ 单文件分发,包含所有依赖和运行时
- ✅ 无需安装 Node.js 或 Bun
- ✅ 启动速度快(使用字节码编译)
- ✅ 文件体积相对较小(已启用压缩和优化)

### 缺点
- ❌ 每次代码更新需要重新编译
- ❌ 可执行文件体积较大(约 50-100MB,包含 Bun 运行时)

---

## 方式二: 使用 Bun 运行时

如果目标环境已安装 Bun,可以直接运行源代码。

### 部署步骤

```bash
# 1. 安装依赖
bun install

# 2. 构建前端静态文件
bun run build

# 3. 启动生产服务
bun run start:prod
```

### 环境变量

创建 `.env` 文件配置:

```bash
# cortex-mem-service 地址
CORTEX_MEM_SERVICE_URL=http://localhost:3000

# Web 服务器端口
PORT=15173
HOST=0.0.0.0
```

### 使用 PM2 管理进程

```bash
# 安装 PM2
npm install -g pm2

# 启动服务
pm2 start start-prod.js --name cortex-mem-insights --interpreter bun

# 查看日志
pm2 logs cortex-mem-insights

# 停止服务
pm2 stop cortex-mem-insights

# 开机自启
pm2 startup
pm2 save
```

### 优点
- ✅ 代码更新方便,无需重新编译
- ✅ 可以使用 PM2 等进程管理工具
- ✅ 支持热重载(开发模式)

### 缺点
- ❌ 需要在目标环境安装 Bun
- ❌ 需要部署整个项目目录

---

## 方式三: Docker 容器

使用 Docker 容器化部署,适合云环境和 Kubernetes。

### Dockerfile

创建 `Dockerfile`:

```dockerfile
FROM oven/bun:latest

WORKDIR /app

# 复制依赖文件
COPY package.json bun.lock ./
RUN bun install --production

# 复制源代码
COPY . .

# 构建前端
RUN bun run build

# 暴露端口
EXPOSE 15173

# 启动服务
CMD ["bun", "run", "start:prod"]
```

### 构建和运行

```bash
# 构建镜像
docker build -t cortex-mem-insights .

# 运行容器
docker run -d \
  --name cortex-mem-insights \
  -p 15173:15173 \
  -e CORTEX_MEM_SERVICE_URL=http://cortex-mem-service:3000 \
  cortex-mem-insights
```

### Docker Compose

创建 `docker-compose.yml`:

```yaml
version: '3.8'

services:
  cortex-mem-insights:
    build: .
    ports:
      - "15173:15173"
    environment:
      - CORTEX_MEM_SERVICE_URL=http://cortex-mem-service:3000
      - PORT=15173
      - HOST=0.0.0.0
    depends_on:
      - cortex-mem-service
    restart: unless-stopped

  cortex-mem-service:
    # cortex-mem-service 配置...
```

### 优点
- ✅ 环境一致性
- ✅ 易于扩展和编排
- ✅ 适合云原生部署

### 缺点
- ❌ 需要 Docker 环境
- ❌ 镜像体积较大

---

## 方式四: 静态文件托管 + API 服务分离

将前端静态文件部署到 CDN,API 服务单独部署。

### 步骤

1. **构建前端静态文件**

```bash
bun run build
```

静态文件位于 `build/` 目录。

2. **部署静态文件到 CDN**

将 `build/` 目录上传到:
- Vercel
- Netlify
- Cloudflare Pages
- AWS S3 + CloudFront
- 任何静态文件托管服务

3. **单独部署 API 服务**

```bash
# 只启动 API 服务
bun run start:api
```

4. **配置 CORS**

在 `src/server/index.ts` 中配置 CORS,允许前端域名访问:

```typescript
import { cors } from '@elysiajs/cors';

app.use(cors({
  origin: 'https://your-frontend-domain.com'
}));
```

### 优点
- ✅ 前端可以利用 CDN 加速
- ✅ 前后端独立部署和扩展
- ✅ 静态文件托管成本低

### 缺点
- ❌ 需要配置 CORS
- ❌ 部署流程相对复杂

---

## 性能优化建议

### 1. 启用字节码编译

在 `build-executable.js` 中已启用:

```javascript
bytecode: true
```

这可以将启动时间减少约 50%。

### 2. 启用压缩

已启用代码压缩:

```javascript
minify: true
```

### 3. 使用 Source Map

生产环境建议启用 source map 以便调试:

```javascript
sourcemap: "linked"
```

### 4. 资源缓存

在 `start-prod.js` 中配置静态资源缓存:

```javascript
set.headers['Cache-Control'] = 'public, max-age=31536000';
```

---

## 监控和日志

### 应用日志

```bash
# 查看实时日志
tail -f logs/app.log

# 使用 PM2 查看日志
pm2 logs cortex-mem-insights
```

### 健康检查

访问 `/health` 端点检查服务状态:

```bash
curl http://localhost:15173/health
```

### 性能监控

可以集成:
- Prometheus + Grafana
- New Relic
- Datadog
- 自定义监控方案

---

## 故障排查

### 问题: 可执行文件无法启动

**解决方案:**
1. 检查文件权限: `chmod +x dist/cortex-mem-insights`
2. 查看错误日志
3. 确保 cortex-mem-service 正在运行

### 问题: 静态文件 404

**解决方案:**
1. 确保已运行 `bun run build`
2. 检查 `build/` 目录是否存在
3. 查看 `start-prod.js` 中的路径配置

### 问题: API 请求失败

**解决方案:**
1. 检查 `CORTEX_MEM_SERVICE_URL` 环境变量
2. 确认 cortex-mem-service 可访问
3. 检查网络和防火墙设置

---

## 安全建议

1. **使用 HTTPS**: 生产环境务必使用 HTTPS
2. **环境变量**: 敏感信息使用环境变量,不要硬编码
3. **CORS 配置**: 限制允许的域名
4. **定期更新**: 及时更新依赖包
5. **访问控制**: 根据需要添加认证和授权

---

## 总结

| 部署方式 | 适用场景 | 难度 | 推荐度 |
|---------|---------|------|--------|
| 独立可执行文件 | 快速部署、无运行时环境 | ⭐ | ⭐⭐⭐⭐⭐ |
| Bun 运行时 | 开发环境、频繁更新 | ⭐⭐ | ⭐⭐⭐⭐ |
| Docker 容器 | 云环境、K8s | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| 静态托管分离 | 高流量、CDN 加速 | ⭐⭐⭐⭐ | ⭐⭐⭐ |

**推荐方案**: 对于大多数场景,使用**独立可执行文件**是最简单高效的部署方式。
