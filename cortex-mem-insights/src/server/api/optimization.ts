import { Elysia, t } from 'elysia';
import { cortexMemCli } from '../integrations/cortex-mem-cli';

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
}

interface OptimizationProgress {
  job_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  current_stage?: string;
  logs: string[];
  result?: any;
  start_time: string;
  end_time?: string;
  duration?: number;
}

// 优化状态存储（内存中）
const optimizationState = new Map<string, OptimizationProgress>();

// 优化API路由
export const optimizationRoutes = new Elysia({ prefix: '/api/optimization' })
  // 启动优化
  .post('/', async ({ body }) => {
    const jobId = `opt_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    // 初始化状态
    optimizationState.set(jobId, {
      job_id: jobId,
      status: 'pending',
      progress: 0,
      logs: [`优化任务 ${jobId} 已创建`],
      start_time: new Date().toISOString(),
    });
    
    // 异步执行优化
    executeOptimization(jobId, body).catch(error => {
      const state = optimizationState.get(jobId);
      if (state) {
        state.status = 'failed';
        state.logs.push(`优化失败: ${error.message}`);
        state.end_time = new Date().toISOString();
        state.duration = new Date(state.end_time).getTime() - new Date(state.start_time).getTime();
        optimizationState.set(jobId, state);
      }
    });
    
    return {
      success: true,
      data: {
        job_id: jobId,
        message: '优化任务已启动',
        status: 'pending',
        start_time: optimizationState.get(jobId)?.start_time,
      },
      timestamp: new Date().toISOString(),
    };
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
    })
  })
  
  // 获取优化状态
  .get('/:jobId', async ({ params }) => {
    const state = optimizationState.get(params.jobId);
    if (!state) {
      return {
        success: false,
        error: {
          code: 'JOB_NOT_FOUND',
          message: `优化任务 ${params.jobId} 不存在`,
        },
        timestamp: new Date().toISOString(),
      };
    }
    
    return {
      success: true,
      data: state,
      timestamp: new Date().toISOString(),
    };
  }, {
    params: t.Object({
      jobId: t.String()
    })
  })
  
  // 取消优化
  .post('/:jobId/cancel', async ({ params }) => {
    const state = optimizationState.get(params.jobId);
    if (!state) {
      return {
        success: false,
        error: {
          code: 'JOB_NOT_FOUND',
          message: `优化任务 ${params.jobId} 不存在`,
        },
        timestamp: new Date().toISOString(),
      };
    }
    
    if (state.status === 'completed' || state.status === 'failed' || state.status === 'cancelled') {
      return {
        success: false,
        error: {
          code: 'JOB_COMPLETED',
          message: `优化任务 ${params.jobId} 已结束，无法取消`,
        },
        timestamp: new Date().toISOString(),
      };
    }
    
    state.status = 'cancelled';
    state.logs.push('优化任务已被用户取消');
    state.end_time = new Date().toISOString();
    state.duration = new Date(state.end_time).getTime() - new Date(state.start_time).getTime();
    optimizationState.set(params.jobId, state);
    
    return {
      success: true,
      data: {
        job_id: params.jobId,
        message: '优化任务已取消',
        status: 'cancelled',
        cancelled_at: state.end_time,
      },
      timestamp: new Date().toISOString(),
    };
  }, {
    params: t.Object({
      jobId: t.String()
    })
  })
  
  // 获取优化历史
  .get('/history', async ({ query }) => {
    const limit = query.limit ? parseInt(query.limit) : 20;
    const offset = query.offset ? parseInt(query.offset) : 0;
    const status = query.status;
    const startDate = query.start_date;
    const endDate = query.end_date;
    
    let history = Array.from(optimizationState.values());
    
    // 应用过滤器
    if (status) {
      history = history.filter(job => job.status === status);
    }
    
    if (startDate) {
      const start = new Date(startDate);
      history = history.filter(job => new Date(job.start_time) >= start);
    }
    
    if (endDate) {
      const end = new Date(endDate);
      history = history.filter(job => new Date(job.start_time) <= end);
    }
    
    // 按开始时间倒序排序
    history.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime());
    
    // 分页
    const total = history.length;
    const paginatedHistory = history.slice(offset, offset + limit);
    
    // 简化历史记录
    const simplifiedHistory = paginatedHistory.map(job => ({
      job_id: job.job_id,
      status: job.status,
      start_time: job.start_time,
      end_time: job.end_time,
      duration: job.duration,
      logs_count: job.logs.length,
      has_result: !!job.result,
    }));
    
    return {
      success: true,
      data: {
        total,
        history: simplifiedHistory,
        pagination: {
          limit,
          offset,
          total,
        },
      },
      timestamp: new Date().toISOString(),
    };
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
      // 使用cortex-mem-cli进行实际分析
      const result = await cortexMemCli.optimize({
        ...body,
        dry_run: true,
        verbose: true,
      });
      
      if (!result.success) {
        return {
          success: false,
          error: {
            code: 'ANALYSIS_FAILED',
            message: result.error || '分析失败',
          },
          timestamp: new Date().toISOString(),
        };
      }
      
      // 解析CLI输出
      const analysisResult = parseOptimizationAnalysis(result.data);
      
      return {
        success: true,
        data: analysisResult,
        timestamp: new Date().toISOString(),
      };
    } catch (error) {
      console.error('分析优化问题失败:', error);
      return {
        success: false,
        error: {
          code: 'INTERNAL_ERROR',
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
      const history = Array.from(optimizationState.values());
      
      const statistics = {
        total_jobs: history.length,
        successful_jobs: history.filter(job => job.status === 'completed').length,
        failed_jobs: history.filter(job => job.status === 'failed').length,
        cancelled_jobs: history.filter(job => job.status === 'cancelled').length,
        total_memories_processed: history.reduce((sum, job) => 
          sum + (job.result?.memories_affected || 0), 0),
        total_memories_deduplicated: history.reduce((sum, job) => 
          sum + (job.result?.deduplicated || 0), 0),
        total_memories_merged: history.reduce((sum, job) => 
          sum + (job.result?.merged || 0), 0),
        total_memories_enhanced: history.reduce((sum, job) => 
          sum + (job.result?.enhanced || 0), 0),
        avg_duration: history.length > 0 
          ? history.reduce((sum, job) => sum + (job.duration || 0), 0) / history.length
          : 0,
        last_run: history.length > 0 
          ? history.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime())[0].start_time
          : null,
      };
      
      return {
        success: true,
        data: statistics,
        timestamp: new Date().toISOString(),
      };
    } catch (error) {
      console.error('获取优化统计失败:', error);
      return {
        success: false,
        error: {
          code: 'INTERNAL_ERROR',
          message: error instanceof Error ? error.message : '获取统计失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  })
  
  // 清理旧的历史记录
  .post('/cleanup', async ({ body }) => {
    try {
      const cutoffTime = Date.now() - (body.max_age_days * 24 * 60 * 60 * 1000);
      
      let deleted = 0;
      for (const [id, state] of optimizationState.entries()) {
        const timestamp = parseInt(id.split('_')[1]);
        if (!isNaN(timestamp) && timestamp < cutoffTime) {
          optimizationState.delete(id);
          deleted++;
        }
      }
      
      return {
        success: true,
        data: {
          deleted,
          remaining: optimizationState.size,
          message: `已清理 ${deleted} 条旧记录`,
        },
        timestamp: new Date().toISOString(),
      };
    } catch (error) {
      console.error('清理历史记录失败:', error);
      return {
        success: false,
        error: {
          code: 'INTERNAL_ERROR',
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

// 异步执行优化任务
async function executeOptimization(jobId: string, request: OptimizationRequest) {
  const state = optimizationState.get(jobId);
  if (!state) return;
  
  try {
    // 更新状态为运行中
    state.status = 'running';
    state.current_stage = '准备优化';
    state.progress = 10;
    state.logs.push('开始准备优化任务...');
    optimizationState.set(jobId, state);
    
    // 调用cortex-mem-cli进行优化
    state.current_stage = '执行优化命令';
    state.progress = 30;
    state.logs.push('正在执行cortex-mem-cli optimize命令...');
    optimizationState.set(jobId, state);
    
    const result = await cortexMemCli.optimize(request);
    
    if (!result.success) {
      throw new Error(result.error || '优化命令执行失败');
    }
    
    // 解析结果
    state.current_stage = '处理优化结果';
    state.progress = 80;
    state.logs.push('优化命令执行成功，正在处理结果...');
    optimizationState.set(jobId, state);
    
    const optimizationResult = parseOptimizationResult(result.data);
    
    // 完成
    state.status = 'completed';
    state.progress = 100;
    state.current_stage = '完成';
    state.result = optimizationResult;
    state.end_time = new Date().toISOString();
    state.duration = new Date(state.end_time).getTime() - new Date(state.start_time).getTime();
    state.logs.push('优化任务完成');
    optimizationState.set(jobId, state);
    
  } catch (error) {
    const state = optimizationState.get(jobId);
    if (state) {
      state.status = 'failed';
      state.logs.push(`执行失败: ${error instanceof Error ? error.message : '未知错误'}`);
      state.end_time = new Date().toISOString();
      state.duration = new Date(state.end_time).getTime() - new Date(state.start_time).getTime();
      optimizationState.set(jobId, state);
    }
    console.error('优化任务执行失败:', error);
  }
}

// 解析优化分析结果
function parseOptimizationAnalysis(data: any) {
  if (!data) {
    return {
      issues: [],
      summary: {
        total_issues: 0,
        total_affected_memories: 0,
        estimated_savings_mb: 0,
        estimated_duration_minutes: 0,
      },
      recommendations: [],
    };
  }
  
  // 尝试从CLI输出中解析信息
  const output = data.output || JSON.stringify(data);
  
  // 简单的解析逻辑（实际项目中需要根据CLI的实际输出格式进行调整）
  const issues = [];
  
  // 检查重复记忆
  const duplicateMatch = output.match(/duplicate.*?(\d+)/i);
  if (duplicateMatch) {
    issues.push({
      type: '重复记忆',
      count: parseInt(duplicateMatch[1]),
      severity: 'high' as const,
      description: '语义相似度超过阈值的记忆',
    });
  }
  
  // 检查低质量记忆
  const lowQualityMatch = output.match(/low.*?quality.*?(\d+)/i);
  if (lowQualityMatch) {
    issues.push({
      type: '低质量记忆',
      count: parseInt(lowQualityMatch[1]),
      severity: 'medium' as const,
      description: '重要性评分较低的记忆',
    });
  }
  
  // 检查过时记忆
  const outdatedMatch = output.match(/outdated.*?(\d+)/i);
  if (outdatedMatch) {
    issues.push({
      type: '过时记忆',
      count: parseInt(outdatedMatch[1]),
      severity: 'low' as const,
      description: '长时间未更新的记忆',
    });
  }
  
  const totalAffected = issues.reduce((sum, issue) => sum + issue.count, 0);
  
  return {
    issues,
    summary: {
      total_issues: issues.length,
      total_affected_memories: totalAffected,
      estimated_savings_mb: parseFloat((totalAffected * 0.15).toFixed(2)),
      estimated_duration_minutes: Math.ceil(totalAffected / 10),
    },
    recommendations: issues.map(issue => ({
      type: issue.type,
      action: issue.severity === 'high' ? '立即处理' : 
              issue.severity === 'medium' ? '建议处理' : '可选处理',
      priority: issue.severity,
    })),
  };
}

// 解析优化结果
function parseOptimizationResult(data: any) {
  if (!data) {
    return {
      memories_affected: 0,
      deduplicated: 0,
      merged: 0,
      enhanced: 0,
      errors: 0,
      space_saved_mb: 0,
      duration_seconds: 0,
    };
  }
  
  const output = data.output || JSON.stringify(data);
  
  // 简单的解析逻辑
  return {
    memories_affected: extractNumber(output, /memories.*?affected.*?(\d+)/i) || 0,
    deduplicated: extractNumber(output, /deduplicated.*?(\d+)/i) || 0,
    merged: extractNumber(output, /merged.*?(\d+)/i) || 0,
    enhanced: extractNumber(output, /enhanced.*?(\d+)/i) || 0,
    errors: extractNumber(output, /errors.*?(\d+)/i) || 0,
    space_saved_mb: extractNumber(output, /space.*?saved.*?([\d.]+)/i) || 0,
    duration_seconds: extractNumber(output, /duration.*?([\d.]+)/i) || 0,
  };
}

// 从文本中提取数字
function extractNumber(text: string, pattern: RegExp): number {
  const match = text.match(pattern);
  return match ? parseFloat(match[1]) : 0;
}