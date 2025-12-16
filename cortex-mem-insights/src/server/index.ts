import { Elysia } from 'elysia';
import { cors } from '@elysiajs/cors';
import { memoryRoutes } from './api/memory';
import { optimizationRoutes } from './api/optimization';
import { systemRoutes } from './api/system';

// 创建Elysia应用
const app = new Elysia()
  .use(cors({
    origin: ['http://localhost:5173', 'http://localhost:3000'],
    credentials: true,
    methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
    allowedHeaders: ['Content-Type', 'Authorization']
  }))
  .get('/health', () => ({
    status: 'healthy',
    timestamp: new Date().toISOString(),
    service: 'cortex-mem-insights-api'
  }))
  .use(memoryRoutes)
  .use(optimizationRoutes)
  .use(systemRoutes)
  .onError(({ code, error }) => {
    console.error(`API Error [${code}]:`, error);
    return {
      error: error.message,
      code,
      timestamp: new Date().toISOString()
    };
  });

// 导出类型化的Elysia实例
export type App = typeof app;

// 启动服务器（仅在直接运行时）
if (import.meta.url === `file://${process.argv[1]}`) {
  const port = process.env.PORT ? parseInt(process.env.PORT) : 15173;
  app.listen(port, () => {
    console.log(`🚀 cortex-mem-insights API 运行在 http://localhost:${port}`);
  });
}

export { app };