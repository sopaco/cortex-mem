# cortex-mem-insights 技术方案文档

## 1. 项目概述

### 1.1 项目背景
cortex-mem-insights 是 cortex-mem 项目的观测后台系统，旨在为 AI 智能体记忆管理系统提供可视化监控、分析和优化功能。通过集成 cortex-mem-service 的 HTTP API，实现对记忆数据的全面观测和管理。

### 1.2 核心目标
- 提供记忆数据的可视化检视界面
- 实现记忆类型、质量分布的统计分析
- 集成 cortex-mem-cli 的 optimize 命令进行优化操作
- 提供系统健康状态监控

### 1.3 技术约束
- 前端：使用 Svelte（不使用 React/Vue）
- 运行时：使用 Bun（不使用 Node.js）
- 集成方式：基于 cortex-mem-service 的 HTTP API
- 部署环境：用户系统已预装 cortex-mem-service

## 2. 系统架构设计

### 2.1 整体架构
```
┌─────────────────────────────────────────────────────────────┐
│                    cortex-mem-insights                       │
├─────────────────────────────────────────────────────────────┤
│  Frontend (SvelteKit)              Backend (Elysia.js)      │
│  ├─ Dashboard                      ├─ API Proxy Layer       │
│  ├─ Memory Browser                 ├─ Optimization Service  │
│  ├─ Analytics Dashboard            ├─ Statistics Service    │
│  ├─ Optimization Panel             ├─ WebSocket Server      │
│  └─ System Monitor                 └─ Cache Manager         │
├─────────────────────────────────────────────────────────────┤
│                    cortex-mem-service                        │
│  ├─ HTTP API (Axum)                                         │
│  ├─ Memory Manager                                          │
│  ├─ Qdrant Integration                                      │
│  └─ LLM Client                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术栈选型

#### 前端技术栈
| 组件 | 技术选型 | 说明 |
|------|----------|------|
| 框架 | SvelteKit | 现代前端框架，编译时优化 |
| UI组件 | Skeleton UI | 基于 Tailwind CSS 的 Svelte 组件库 |
| 图表 | Chart.js + svelte-chartjs | 数据可视化 |
| 状态管理 | Svelte stores | 内置状态管理方案 |
| HTTP客户端 | fetch API + 封装 | 轻量级 HTTP 请求 |
| 打包工具 | Vite | SvelteKit 内置 |

#### 后端技术栈
| 组件 | 技术选型 | 说明 |
|------|----------|------|
| 运行时 | Bun | 高性能 JavaScript 运行时 |
| Web框架 | Elysia.js | Bun 原生高性能框架 |
| 数据存储 | 内存缓存 + 本地文件 | 单用户工具，简化存储 |
| WebSocket | Bun WebSocket API | 实时通信 |

### 2.3 数据流设计

#### 2.3.1 记忆数据流
```
用户请求 → Svelte前端 → Elysia后端 → cortex-mem-service → Qdrant/LLM
      ↑          ↑           ↑               ↑
      └──────────┴───────────┴───────────────┘
        响应渲染    业务处理    代理转发    数据源
```

#### 2.3.2 优化命令流
```
用户触发优化 → 前端 → Elysia后端 → 启动优化进程 → 实时进度推送
      ↑          ↑         ↑              ↑
      └──────────┴─────────┴──────────────┘
        状态更新   WebSocket   子进程管理
```

## 3. 功能模块设计

### 3.1 前端模块

#### 3.1.1 仪表盘 (Dashboard)
- 系统概览卡片（记忆总数、今日新增、优化次数）
- 实时监控图表（Qdrant 状态、LLM 服务状态）
- 快速操作面板（一键优化、数据刷新）

#### 3.1.2 记忆浏览器 (Memory Browser)
- 记忆列表（支持分页、虚拟滚动）
- 高级过滤（用户ID、Agent ID、记忆类型、时间范围）
- 搜索功能（全文搜索、语义搜索）
- 记忆详情面板（内容预览、元数据展示）

#### 3.1.3 分析面板 (Analytics Dashboard)
- 记忆类型分布图（饼图/柱状图）
- 质量评分分布（直方图）
- 时间趋势分析（折线图）
- 用户/Agent 维度统计（表格）

#### 3.1.4 优化面板 (Optimization Panel)
- 优化计划预览
- 一键执行优化
- 实时进度展示
- 优化历史记录
- 优化报告查看

#### 3.1.5 系统监控 (System Monitor)
- 服务健康状态
- 存储空间使用情况
- 性能指标监控
- 日志查看器

### 3.2 后端模块

#### 3.2.1 API 代理层 (API Proxy Layer)
```typescript
// 示例：记忆列表代理
app.get('/api/memories', async (context) => {
  const { limit, offset, filters } = context.query;
  const response = await fetch(`${CORTEX_MEM_SERVICE}/memories?${params}`, {
    headers: { 'Authorization': `Bearer ${API_KEY}` }
  });
  return response.json();
});
```

#### 3.2.2 优化服务 (Optimization Service)
- 优化命令执行器
- 进度跟踪管理器
- 结果报告生成器
- 历史记录存储

#### 3.2.3 统计服务 (Statistics Service)
- 数据聚合计算
- 缓存管理（SQLite）
- 定时更新任务
- 报表生成

#### 3.2.4 WebSocket 服务
- 实时进度推送
- 系统状态更新
- 日志流传输
- 事件通知

## 4. API 集成设计

### 4.1 cortex-mem-service API 映射

#### 4.1.1 现有 API 端点
| 端点 | 方法 | 功能 | insights 用途 |
|------|------|------|---------------|
| `/health` | GET | 健康检查 | 系统监控 |
| `/memories` | POST | 创建记忆 | 测试功能 |
| `/memories` | GET | 列出记忆 | 记忆浏览器 |
| `/memories/search` | POST | 搜索记忆 | 搜索功能 |
| `/memories/:id` | GET | 获取记忆 | 详情查看 |

#### 4.1.2 需要扩展的 API
```rust
// 建议在 cortex-mem-service 中添加以下端点：
// 1. 统计端点
GET /stats/summary          // 系统概览统计
GET /stats/distribution     // 类型分布统计
GET /stats/trends           // 时间趋势统计

// 2. 优化端点
POST /optimize/start        // 启动优化
GET  /optimize/:id/status   // 获取优化状态
GET  /optimize/:id/report   // 获取优化报告
GET  /optimize/history      // 优化历史记录

// 3. 监控端点
GET /monitor/status         // 详细监控状态
GET /monitor/metrics        // 性能指标
```

### 4.2 insights 后端 API 设计

#### 4.2.1 记忆相关 API
```typescript
// 记忆列表（带分页和过滤）
GET /api/v1/memories
Query: {
  page?: number
  limit?: number
  userId?: string
  agentId?: string
  memoryType?: string
  startDate?: string
  endDate?: string
  search?: string
}

// 记忆详情
GET /api/v1/memories/:id

// 批量操作
POST /api/v1/memories/batch
Body: {
  action: 'delete' | 'export' | 'tag'
  ids: string[]
  // ... 其他参数
}
```

#### 4.2.2 统计相关 API
```typescript
// 系统概览
GET /api/v1/stats/overview

// 类型分布
GET /api/v1/stats/distribution
Query: {
  dimension: 'type' | 'quality' | 'user' | 'agent'
  groupBy?: 'day' | 'week' | 'month'
}

// 时间趋势
GET /api/v1/stats/trends
Query: {
  metric: 'count' | 'quality' | 'size'
  period: '7d' | '30d' | '90d'
}
```

#### 4.2.3 优化相关 API
```typescript
// 启动优化
POST /api/v1/optimize
Body: {
  strategy: 'full' | 'deduplication' | 'quality'
  filters?: OptimizationFilters
  options?: {
    preview?: boolean
    aggressive?: boolean
    timeout?: number
  }
}

// 获取优化状态
GET /api/v1/optimize/:id/status

// 优化历史
GET /api/v1/optimize/history
Query: {
  page?: number
  limit?: number
  status?: 'completed' | 'failed' | 'running'
}
```

#### 4.2.4 WebSocket 端点
```typescript
// 优化进度
WS /ws/optimize/:id

// 系统状态
WS /ws/system-status

// 实时日志
WS /ws/logs
```

## 5. 数据存储设计

### 5.1 内存缓存
```typescript
// 简单的内存缓存实现
class MemoryCache {
  private cache = new Map<string, { value: any; expires: number }>();
  
  set(key: string, value: any, ttl: number = 300000) {
    this.cache.set(key, {
      value,
      expires: Date.now() + ttl
    });
  }
  
  get(key: string): any | null {
    const item = this.cache.get(key);
    if (!item) return null;
    
    if (Date.now() > item.expires) {
      this.cache.delete(key);
      return null;
    }
    
    return item.value;
  }
  
  delete(key: string) {
    this.cache.delete(key);
  }
  
  clear() {
    this.cache.clear();
  }
}
```

### 5.2 本地文件存储（可选）
```typescript
// 用于存储用户配置和优化历史
import { writeFile, readFile } from 'fs/promises';
import { join } from 'path';

class LocalStorage {
  private configPath: string;
  
  constructor(appDataPath: string) {
    this.configPath = join(appDataPath, 'cortex-mem-insights');
  }
  
  async saveConfig(key: string, data: any) {
    const filePath = join(this.configPath, `${key}.json`);
    await writeFile(filePath, JSON.stringify(data, null, 2));
  }
  
  async loadConfig(key: string): Promise<any> {
    try {
      const filePath = join(this.configPath, `${key}.json`);
      const content = await readFile(filePath, 'utf-8');
      return JSON.parse(content);
    } catch {
      return null;
    }
  }
}
```

## 6. 部署架构

### 6.1 开发环境部署
```
cortex-mem-insights (Bun + SvelteKit)
    ↓ HTTP/WebSocket
cortex-mem-service (cargo run)
    ↓
Qdrant + LLM Service
```

### 6.2 生产环境部署
```
用户浏览器
    ↓ HTTP/WebSocket
cortex-mem-insights (Bun 服务)
    ↓ HTTP
cortex-mem-service (预编译二进制)
    ↓
Qdrant + LLM Service
```

### 6.3 配置管理
```typescript
// config/default.ts
export default {
  cortexMemService: {
    url: process.env.CORTEX_MEM_SERVICE_URL || 'http://localhost:3000',
    timeout: 30000
  },
  
  server: {
    port: process.env.PORT || 5173,
    host: process.env.HOST || 'localhost'
  },
  
  cache: {
    ttl: 300000, // 5分钟
    maxSize: 1000
  }
};
```

## 7. 开发计划

### Phase 1: 基础框架搭建 (2天)
- [ ] 初始化 SvelteKit + Elysia 项目
- [ ] 配置开发环境 (Bun, TypeScript, ESLint)
- [ ] 基础布局和路由设置
- [ ] cortex-mem-service 集成测试

### Phase 2: 核心功能实现 (5天)
- [ ] 记忆列表和详情页面
- [ ] 基础统计图表
- [ ] optimize 命令集成
- [ ] WebSocket 实时更新
- [ ] 过滤和搜索功能

### Phase 3: 高级功能开发 (3天)
- [ ] 高级统计分析
- [ ] 批量操作功能
- [ ] 导出功能 (CSV, JSON)
- [ ] 系统监控面板
- [ ] 用户配置管理

### Phase 4: 优化和部署 (2天)
- [ ] 性能优化 (缓存、懒加载)
- [ ] 错误处理和日志
- [ ] 生产环境配置
- [ ] 文档编写
- [ ] 测试用例

## 8. 测试策略

### 8.1 单元测试
- 前端组件测试 (Vitest)
- 后端服务测试 (Bun test)
- API 集成测试

### 8.2 集成测试
- cortex-mem-service 集成测试
- 端到端功能测试
- 性能压力测试

### 8.3 测试数据
```typescript
// 测试数据生成器
const generateTestMemories = (count: number) => {
  const memories = [];
  for (let i = 0; i < count; i++) {
    memories.push({
      id: `mem_${i}`,
      content: `Test memory content ${i}`,
      metadata: {
        memoryType: ['conversational', 'factual', 'personal'][i % 3],
        importanceScore: Math.random(),
        userId: `user_${i % 5}`,
        agentId: `agent_${i % 3}`
      }
    });
  }
  return memories;
};
```

## 9. 性能考虑

### 9.1 前端性能优化
- 虚拟滚动处理大量数据
- 图片和资源懒加载
- 组件代码分割
- 服务端渲染 (SSR) 优化

### 9.2 后端性能优化
- API 响应缓存
- 数据库查询优化
- 批量操作支持
- 连接池管理

### 9.3 内存管理
- 大文件分片处理
- 流式响应支持
- 内存泄漏检测
- 垃圾回收优化

## 10. 简化安全考虑

### 10.1 基础安全
- 跨域资源共享 (CORS) 配置
- 输入验证和清理
- 错误信息脱敏

### 10.2 单用户工具特性
- 本地运行，无需网络认证
- 数据不离开本地设备
- 简单日志记录用于调试

## 11. 扩展性设计

### 11.1 插件系统
```typescript
// 插件接口定义
interface InsightsPlugin {
  name: string;
  version: string;
  initialize: (context: PluginContext) => Promise<void>;
  registerRoutes?: (app: Elysia) => void;
  registerComponents?: () => Component[];
}

// 插件管理器
class PluginManager {
  private plugins: Map<string, InsightsPlugin> = new Map();
  
  async loadPlugin(plugin: InsightsPlugin) {
    await plugin.initialize(this.context);
    this.plugins.set(plugin.name, plugin);
  }
}
```

### 11.2 主题系统
- 多主题支持 (light/dark)
- 自定义 CSS 变量
- 主题切换器
- 主题导出/导入

### 11.3 国际化
- 多语言支持
- 动态语言切换
- 翻译文件管理
- RTL 布局支持

## 12. 监控和运维

### 12.1 健康检查
```typescript
// 健康检查端点
app.get('/health', async () => {
  const checks = {
    cortex_mem_service: await checkCortexMemService(),
    database: await checkDatabase(),
    cache: await checkCache(),
    memory: process.memoryUsage()
  };
  
  return {
    status: checks.cortex_mem_service.healthy ? 'healthy' : 'unhealthy',
    checks,
    timestamp: new Date().toISOString()
  };
});
```

### 12.2 指标收集
- API 响应时间
- 内存使用情况
- CPU 使用率
- 错误率统计
- 用户行为分析

### 12.3 日志管理
- 结构化日志输出
- 日志级别控制
- 日志轮转策略
- 日志聚合分析

## 13. 风险评估和应对

### 13.1 技术风险
| 风险 | 概率 | 影响 | 应对措施 |
|------|------|------|----------|
| cortex-mem-service API 变更 | 中 | 高 | 1. API 版本管理 2. 兼容性层 3. 自动测试 |
| Bun 运行时兼容性问题 | 低 | 中 | 1. 定期更新 2. 降级方案 3. 社区支持 |
| 大数据量性能问题 | 中 | 高 | 1. 分页优化 2. 缓存策略 3. 异步处理 |

### 13.2 业务风险
| 风险 | 概率 | 影响 | 应对措施 |
|------|------|------|----------|
| 优化操作误删数据 | 中 | 高 | 1. 操作确认 2. 预览模式 3. 详细日志 |
| cortex-mem-service 连接失败 | 中 | 高 | 1. 自动重试 2. 清晰错误提示 3. 服务状态检查 |

## 14. 成功标准

### 14.1 功能完成度
- [ ] 记忆浏览功能完整实现
- [ ] 统计分析功能准确可靠
- [ ] 优化集成功能稳定运行
- [ ] 系统监控功能全面覆盖

### 14.2 性能指标
- 页面加载时间 < 2秒
- API 响应时间 < 500ms
- 内存使用 < 500MB
- 支持并发用户 > 50

### 14.3 用户体验
- 界面响应流畅
- 操作简单直观
- 错误提示清晰
- 帮助文档完整

## 15. 后续规划

### 15.1 短期规划 (1-3个月)
- 移动端适配
- 高级分析功能
- 团队协作功能
- 通知系统

### 15.2 中期规划 (3-6个月)
- AI 辅助分析
- 预测性优化
- 第三方集成
- 插件市场

### 15.3 长期规划 (6-12个月)
- 分布式部署
- 多租户支持
- 机器学习集成
- 开放平台

---

**文档版本**: 1.0  
**最后更新**: 2025-12-13  
**负责人**: iFlow CLI  
**状态**: 草案 (待评审)