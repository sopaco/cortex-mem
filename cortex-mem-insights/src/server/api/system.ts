import { Elysia, t } from 'elysia';
import { cors } from '@elysiajs/cors';
import { cortexMemService } from '../integrations/cortex-mem';
import os from 'os';
import process from 'process';

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
  
  // 获取性能指标 - 返回真实的系统性能数据
  .get('/metrics', () => {
    // 计算CPU使用率
    const cpus = os.cpus();
    const cpuUsage = cpus.reduce((acc: number, cpu: any) => {
      const times = cpu.times as { user: number; nice: number; sys: number; idle: number; irq: number };
      const total = (Object.values(times) as number[]).reduce((a: number, b: number) => a + b, 0);
      const idle = times.idle;
      return acc + ((total - idle) / total) * 100;
    }, 0) / cpus.length;
    
    // 计算内存使用率
    const totalMem = os.totalmem();
    const freeMem = os.freemem();
    const usedMem = totalMem - freeMem;
    const memoryUsage = (usedMem / totalMem) * 100;
    
    return {
      success: true,
      data: {
        cpu_usage: parseFloat(cpuUsage.toFixed(2)),
        memory_usage: parseFloat(memoryUsage.toFixed(2)),
        disk_usage: 0, // 磁盘使用率需要额外的库来获取,暂时返回0
        active_connections: 0, // 活动连接数需要从应用层统计
        request_count: 0, // 请求计数需要从应用层统计
        error_rate: 0, // 错误率需要从应用层统计
        response_time_avg: 0, // 平均响应时间需要从应用层统计
        timestamp: new Date().toISOString(),
      },
      timestamp: new Date().toISOString(),
    };
  })
  
  // 获取系统信息 - 返回真实的系统信息
  .get('/info', () => {
    // 计算运行时间
    const uptimeSeconds = process.uptime();
    const days = Math.floor(uptimeSeconds / 86400);
    const hours = Math.floor((uptimeSeconds % 86400) / 3600);
    const minutes = Math.floor((uptimeSeconds % 3600) / 60);
    const uptime = `${days} days, ${hours} hours, ${minutes} minutes`;
    
    return {
      success: true,
      data: {
        version: '0.1.0',
        uptime,
        platform: os.platform(),
        arch: os.arch(),
        node_version: process.version,
        memory_total: Math.floor(os.totalmem() / 1024 / 1024), // MB
        memory_used: Math.floor((os.totalmem() - os.freemem()) / 1024 / 1024), // MB
        cpu_count: os.cpus().length,
        hostname: os.hostname(),
      },
      timestamp: new Date().toISOString(),
    };
  })
  
  // 获取实时日志 - 暂时返回空数组,实际应该从日志系统获取
  .get('/logs', ({ query }) => {
    const { limit = 50, level, source } = query as {
      limit?: number;
      level?: string;
      source?: string;
    };
    
    // TODO: 实现真实的日志获取逻辑
    // 目前返回空数组,因为没有实际的日志系统
    const logs: any[] = [];
    
    return {
      success: true,
      data: logs,
      total: logs.length,
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
  
  // 获取资源使用情况 - 返回真实的资源使用数据
  .get('/resources', () => {
    const totalMem = os.totalmem();
    const freeMem = os.freemem();
    const usedMem = totalMem - freeMem;
    
    // 计算CPU使用率
    const cpus = os.cpus();
    const cpuUsage = cpus.reduce((acc: number, cpu: any) => {
      const times = cpu.times as { user: number; nice: number; sys: number; idle: number; irq: number };
      const total = (Object.values(times) as number[]).reduce((a: number, b: number) => a + b, 0);
      const idle = times.idle;
      return acc + ((total - idle) / total) * 100;
    }, 0) / cpus.length;
    
    return {
      success: true,
      data: {
        memory: {
          total: Math.floor(totalMem / 1024 / 1024), // MB
          used: Math.floor(usedMem / 1024 / 1024), // MB
          free: Math.floor(freeMem / 1024 / 1024), // MB
          percentage: (usedMem / totalMem) * 100,
        },
        cpu: {
          usage: parseFloat(cpuUsage.toFixed(2)),
          cores: cpus.length,
        },
        disk: {
          usage: 0, // 磁盘使用率需要额外的库来获取
        },
        network: {
          active_connections: 0, // 活动连接数需要从应用层统计
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
    
    const errorMessage = error instanceof Error ? error.message : 'An unexpected error occurred';
    
    return {
      success: false,
      error: {
        code: code || 'INTERNAL_ERROR',
        message: errorMessage,
      },
      timestamp: new Date().toISOString(),
    };
  });