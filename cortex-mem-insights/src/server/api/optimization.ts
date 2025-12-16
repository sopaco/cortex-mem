import { Elysia, t } from 'elysia';
import { cortexMemService } from '../integrations/cortex-mem';

// 类型定义
interface OptimizationRequest {
  memory_type?: string;
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  similarity_threshold?: number;
  dry_run?: boolean;
  verbose?: boolean;
  strategy?: string;
  aggressive?: boolean;
  timeout_minutes?: number;
}



// 优化API路由
export const optimizationRoutes = new Elysia({ prefix: '/api/optimization' })
  // 启动优化
  .post('/', async ({ body }) => {
    try {
      // 直接调用cortex-mem-service的API
      const result = await cortexMemService.optimize(body);
      return result;
    } catch (error) {
      console.error('启动优化失败:', error);
      return {
        success: false,
        error: {
          code: 'OPTIMIZE_FAILED',
          message: error instanceof Error ? error.message : '启动优化失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }, {
    body: t.Object({
      memory_type: t.Optional(t.String()),
      user_id: t.Optional(t.String()),
      agent_id: t.Optional(t.String()),
      run_id: t.Optional(t.String()),
      actor_id: t.Optional(t.String()),
      similarity_threshold: t.Optional(t.Number({ default: 0.7 })),
      dry_run: t.Optional(t.Boolean({ default: false })),
      verbose: t.Optional(t.Boolean({ default: false })),
      strategy: t.Optional(t.String()),
      aggressive: t.Optional(t.Boolean({ default: false })),
      timeout_minutes: t.Optional(t.Number()),
    })
  })
  
  // 获取优化状态
  .get('/:jobId', async ({ params }) => {
    try {
      const result = await cortexMemService.getOptimizationStatus(params.jobId);
      return result;
    } catch (error) {
      console.error('获取优化状态失败:', error);
      return {
        success: false,
        error: {
          code: 'GET_STATUS_FAILED',
          message: error instanceof Error ? error.message : '获取状态失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }, {
    params: t.Object({
      jobId: t.String()
    })
  })
  
  // 取消优化
  .post('/:jobId/cancel', async ({ params }) => {
    try {
      const result = await cortexMemService.cancelOptimization(params.jobId);
      return result;
    } catch (error) {
      console.error('取消优化失败:', error);
      return {
        success: false,
        error: {
          code: 'CANCEL_FAILED',
          message: error instanceof Error ? error.message : '取消优化失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }, {
    params: t.Object({
      jobId: t.String()
    })
  })
  
  // 获取优化历史
  .get('/history', async ({ query }) => {
    try {
      const result = await cortexMemService.getOptimizationHistory({
        limit: query.limit ? parseInt(query.limit) : 20,
        offset: query.offset ? parseInt(query.offset) : 0,
        status: query.status,
        start_date: query.start_date,
        end_date: query.end_date,
      });
      return result;
    } catch (error) {
      console.error('获取优化历史失败:', error);
      return {
        success: false,
        error: {
          code: 'GET_HISTORY_FAILED',
          message: error instanceof Error ? error.message : '获取历史失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }, {
    query: t.Object({
      limit: t.Optional(t.String()),
      offset: t.Optional(t.String()),
      status: t.Optional(t.String()),
      start_date: t.Optional(t.String()),
      end_date: t.Optional(t.String()),
    })
  })
  
  // 分析优化问题（预览）
  .post('/analyze', async ({ body }) => {
    try {
      const result = await cortexMemService.analyzeOptimization(body);
      return result;
    } catch (error) {
      console.error('分析优化问题失败:', error);
      return {
        success: false,
        error: {
          code: 'ANALYZE_FAILED',
          message: error instanceof Error ? error.message : '分析失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }, {
    body: t.Object({
      memory_type: t.Optional(t.String()),
      user_id: t.Optional(t.String()),
      agent_id: t.Optional(t.String()),
      run_id: t.Optional(t.String()),
      actor_id: t.Optional(t.String()),
      similarity_threshold: t.Optional(t.Number({ default: 0.7 })),
    })
  })
  
  // 获取优化统计
  .get('/statistics', async () => {
    try {
      const result = await cortexMemService.getOptimizationStatistics();
      return result;
    } catch (error) {
      console.error('获取优化统计失败:', error);
      return {
        success: false,
        error: {
          code: 'GET_STATISTICS_FAILED',
          message: error instanceof Error ? error.message : '获取统计失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  })
  
  // 清理旧的历史记录
  .post('/cleanup', async ({ body }) => {
    try {
      const result = await cortexMemService.cleanupOptimizationHistory(body.max_age_days);
      return result;
    } catch (error) {
      console.error('清理历史记录失败:', error);
      return {
        success: false,
        error: {
          code: 'CLEANUP_FAILED',
          message: error instanceof Error ? error.message : '清理失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }, {
    body: t.Object({
      max_age_days: t.Number({ default: 7 })
    })
  });