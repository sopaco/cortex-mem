import { Elysia, t } from 'elysia';
import { cors } from '@elysiajs/cors';
import { cortexMemService } from '../integrations/cortex-mem';

// 系统状态接口
interface SystemStatus {
  status: 'healthy' | 'unhealthy';
  vector_store: boolean;
  llm_service: boolean;
  timestamp: string;
}

// 性能指标接口
interface PerformanceMetrics {
  cpu_usage: number;
  memory_usage: number;
  disk_usage: number;
  active_connections: number;
  request_count: number;
  error_rate: number;
  response_time_avg: number;
  timestamp: string;
}

// 系统信息接口
interface SystemInfo {
  version: string;
  uptime: string;
  platform: string;
  arch: string;
  node_version: string;
  memory_total: number;
  memory_used: number;
  cpu_count: number;
  hostname: string;
}

// 日志条目接口
interface LogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  source: string;
  metadata?: Record<string, any>;
}

// 模拟数据
const mockSystemStatus: SystemStatus = {
  status: 'healthy',
  vector_store: true,
  llm_service: true,
  timestamp: new Date().toISOString(),
};

const mockPerformanceMetrics: PerformanceMetrics = {
  cpu_usage: 45.2,
  memory_usage: 68.7,
  disk_usage: 32.1,
  active_connections: 12,
  request_count: 1250,
  error_rate: 0.5,
  response_time_avg: 125.3,
  timestamp: new Date().toISOString(),
};

const mockSystemInfo: SystemInfo = {
  version: '0.1.0',
  uptime: '2 days, 3 hours, 45 minutes',
  platform: 'win32',
  arch: 'x64',
  node_version: '22.12.0',
  memory_total: 16384,
  memory_used: 11264,
  cpu_count: 8,
  hostname: 'cortex-mem-insights',
};

const mockLogs: LogEntry[] = [
  {
    timestamp: new Date(Date.now() - 60000).toISOString(),
    level: 'info',
    message: 'System health check completed',
    source: 'health-check',
  },
  {
    timestamp: new Date(Date.now() - 120000).toISOString(),
    level: 'info',
    message: 'Memory search request processed',
    source: 'memory-api',
    metadata: { query: 'test', results: 5 },
  },
  {
    timestamp: new Date(Date.now() - 180000).toISOString(),
    level: 'warn',
    message: 'High memory usage detected',
    source: 'monitor',
    metadata: { usage: 85.2 },
  },
  {
    timestamp: new Date(Date.now() - 240000).toISOString(),
    level: 'info',
    message: 'Optimization job started',
    source: 'optimization-api',
    metadata: { job_id: 'opt-123' },
  },
  {
    timestamp: new Date(Date.now() - 300000).toISOString(),
    level: 'error',
    message: 'Failed to connect to vector store',
    source: 'vector-store',
    metadata: { error: 'Connection timeout' },
  },
];

// 创建系统API路由
export const systemRoutes = new Elysia({ prefix: '/api/system' })
  .use(cors())
  
  // 获取系统状态
  .get('/status', async () => {
    try {
      // 获取真实的cortex-mem-service状态
      const healthCheck = await cortexMemService.healthCheck();
      const llmStatus = await cortexMemService.getLLMStatus();
      
      // 检查Qdrant状态（通过cortex-mem-service的健康检查）
      const vectorStoreStatus = healthCheck.vector_store;
      const llmServiceStatus = healthCheck.llm_service;

      const systemStatus = {
        status: vectorStoreStatus && llmServiceStatus ? 'healthy' : 'unhealthy',
        cortex_mem_service: true, // cortex-mem-service可用
        vector_store: vectorStoreStatus,
        llm_service: llmServiceStatus,
        llm_details: {
          completion_model: llmStatus.completion_model,
          embedding_model: llmStatus.embedding_model,
          overall_status: llmStatus.overall_status,
        },
        timestamp: new Date().toISOString(),
      };

      return {
        success: true,
        data: systemStatus,
        timestamp: new Date().toISOString(),
      };
    } catch (error) {
      console.error('获取系统状态失败 - cortex-mem-service不可用:', error);
      // 当cortex-mem-service不可用时，返回错误状态
      return {
        success: true, // 仍然返回success: true，但数据中标记服务不可用
        data: {
          status: 'unhealthy',
          cortex_mem_service: false,
          vector_store: false,
          llm_service: false,
          llm_details: {
            completion_model: {
              available: false,
              provider: 'unknown',
              model_name: 'unknown',
              error_message: error instanceof Error ? error.message : 'Cortex Memory Service不可用',
              last_check: new Date().toISOString(),
            },
            embedding_model: {
              available: false,
              provider: 'unknown',
              model_name: 'unknown',
              error_message: error instanceof Error ? error.message : 'Cortex Memory Service不可用',
              last_check: new Date().toISOString(),
            },
            overall_status: 'error',
          },
          timestamp: new Date().toISOString(),
        },
        timestamp: new Date().toISOString(),
      };
    }
  })
  
  // 获取向量存储状态
  .get('/vector-store/status', async () => {
    try {
      const healthCheck = await cortexMemService.healthCheck();
      
      return {
        success: true,
        data: {
          status: healthCheck.vector_store ? 'connected' : 'disconnected',
          available: healthCheck.vector_store,
          type: 'qdrant',
          last_check: healthCheck.timestamp,
        },
        timestamp: new Date().toISOString(),
      };
    } catch (error) {
      console.error('获取向量存储状态失败 - cortex-mem-service不可用:', error);
      // 当cortex-mem-service不可用时，向量存储也不可用
      return {
        success: false,
        error: {
          code: 'CORTEX_MEM_SERVICE_UNAVAILABLE',
          message: error instanceof Error ? error.message : 'Cortex Memory Service不可用',
        },
        timestamp: new Date().toISOString(),
      };
    }
  })
  
  // 获取LLM服务详细状态
  .get('/llm/status', async () => {
    try {
      const llmStatus = await cortexMemService.getLLMStatus();
      
      return {
        success: true,
        data: llmStatus,
        timestamp: new Date().toISOString(),
      };
    } catch (error) {
      console.error('获取LLM服务状态失败 - cortex-mem-service不可用:', error);
      // 当cortex-mem-service不可用时，LLM服务也不可用
      return {
        success: false,
        error: {
          code: 'CORTEX_MEM_SERVICE_UNAVAILABLE',
          message: error instanceof Error ? error.message : 'Cortex Memory Service不可用',
        },
        timestamp: new Date().toISOString(),
      };
    }
  })
  
  // 获取性能指标
  .get('/metrics', () => {
    return {
      success: true,
      data: mockPerformanceMetrics,
      timestamp: new Date().toISOString(),
    };
  })
  
  // 获取系统信息
  .get('/info', () => {
    return {
      success: true,
      data: mockSystemInfo,
      timestamp: new Date().toISOString(),
    };
  })
  
  // 获取实时日志
  .get('/logs', ({ query }) => {
    const { limit = 50, level, source } = query as {
      limit?: number;
      level?: string;
      source?: string;
    };
    
    let filteredLogs = [...mockLogs];
    
    if (level) {
      filteredLogs = filteredLogs.filter(log => log.level === level);
    }
    
    if (source) {
      filteredLogs = filteredLogs.filter(log => log.source === source);
    }
    
    filteredLogs = filteredLogs.slice(0, limit);
    
    return {
      success: true,
      data: filteredLogs,
      total: filteredLogs.length,
      timestamp: new Date().toISOString(),
    };
  })
  
  // 健康检查 - 返回insights server自身的健康状态
  .get('/health', async () => {
    try {
      // 检查cortex-mem-service的健康状态
      const healthCheck = await cortexMemService.healthCheck();
      
      return {
        success: true,
        status: healthCheck.status === 'healthy' ? 'healthy' : 'unhealthy',
        timestamp: new Date().toISOString(),
        services: {
          cortex_mem_service: true,
          vector_store: healthCheck.vector_store,
          llm_service: healthCheck.llm_service,
        },
      };
    } catch (error) {
      console.error('健康检查失败 - cortex-mem-service不可用:', error);
      return {
        success: false,
        status: 'unhealthy',
        timestamp: new Date().toISOString(),
        services: {
          cortex_mem_service: false,
          vector_store: false,
          llm_service: false,
        },
        error: {
          code: 'CORTEX_MEM_SERVICE_UNAVAILABLE',
          message: error instanceof Error ? error.message : 'Cortex Memory Service不可用',
        },
      };
    }
  })
  
  // 获取资源使用情况
  .get('/resources', () => {
    return {
      success: true,
      data: {
        memory: {
          total: mockSystemInfo.memory_total,
          used: mockSystemInfo.memory_used,
          free: mockSystemInfo.memory_total - mockSystemInfo.memory_used,
          percentage: (mockSystemInfo.memory_used / mockSystemInfo.memory_total) * 100,
        },
        cpu: {
          usage: mockPerformanceMetrics.cpu_usage,
          cores: mockSystemInfo.cpu_count,
        },
        disk: {
          usage: mockPerformanceMetrics.disk_usage,
        },
        network: {
          active_connections: mockPerformanceMetrics.active_connections,
        },
      },
      timestamp: new Date().toISOString(),
    };
  })
  
  // 清理系统缓存
  .post('/clear-cache', () => {
    return {
      success: true,
      message: 'System cache cleared successfully',
      timestamp: new Date().toISOString(),
    };
  })
  
  // 重启服务
  .post('/restart', () => {
    return {
      success: true,
      message: 'Service restart initiated',
      timestamp: new Date().toISOString(),
      restart_time: new Date(Date.now() + 5000).toISOString(),
    };
  })
  
  // 错误处理
  .onError(({ code, error }) => {
    console.error('System API error:', error);
    
    return {
      success: false,
      error: {
        code: code || 'INTERNAL_ERROR',
        message: error.message || 'An unexpected error occurred',
      },
      timestamp: new Date().toISOString(),
    };
  });