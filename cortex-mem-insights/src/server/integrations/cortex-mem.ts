import { MemoryResponse, SearchResponse, ListResponse, HealthResponse } from '../api/types';

// Cortex-mem-service API 客户端
export class CortexMemServiceClient {
  private baseUrl: string;
  
  constructor(baseUrl: string = 'http://localhost:3000') {
    this.baseUrl = baseUrl;
  }
  
  // 健康检查
  async healthCheck(): Promise<HealthResponse> {
    const response = await fetch(`${this.baseUrl}/health`);
    if (!response.ok) {
      throw new Error(`Health check failed: ${response.statusText}`);
    }
    return await response.json();
  }
  
  // 获取记忆列表
  async listMemories(params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
  }): Promise<ListResponse> {
    try {
      const queryParams = new URLSearchParams();
      if (params?.user_id) queryParams.append('user_id', params.user_id);
      if (params?.agent_id) queryParams.append('agent_id', params.agent_id);
      if (params?.run_id) queryParams.append('run_id', params.run_id);
      if (params?.actor_id) queryParams.append('actor_id', params.actor_id);
      if (params?.memory_type) queryParams.append('memory_type', params.memory_type);
      if (params?.limit) queryParams.append('limit', params.limit.toString());
      
      const url = `${this.baseUrl}/memories${queryParams.toString() ? `?${queryParams}` : ''}`;
      
      const response = await fetch(url);
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('获取记忆列表失败 - 错误响应:', errorText);
        throw new Error(`List memories failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      return result;
    } catch (error) {
      console.error('获取记忆列表错误:', error);
      return {
        total: 0,
        memories: [],
      };
    }
  }
  
  // 搜索记忆
  async searchMemories(query: string, params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
    similarity_threshold?: number;
  }): Promise<SearchResponse> {
    try {
      const requestBody = {
        query,
        user_id: params?.user_id,
        agent_id: params?.agent_id,
        run_id: params?.run_id,
        actor_id: params?.actor_id,
        memory_type: params?.memory_type,
        limit: params?.limit,
        similarity_threshold: params?.similarity_threshold,
      };
      
      const response = await fetch(`${this.baseUrl}/memories/search`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestBody),
      });
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('搜索记忆失败 - 错误响应:', errorText);
        throw new Error(`Search memories failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      return result;
    } catch (error) {
      console.error('搜索记忆错误:', error);
      return {
        total: 0,
        results: [],
      };
    }
  }
  
  // 获取单个记忆
  async getMemory(id: string): Promise<MemoryResponse | null> {
    try {
      const response = await fetch(`${this.baseUrl}/memories/${id}`);
      
      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        throw new Error(`Get memory failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('Get memory error:', error);
      return null;
    }
  }
  
  // 创建记忆
  async createMemory(content: string, metadata?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    role?: string;
    memory_type?: string;
    custom?: Record<string, any>;
  }): Promise<{ success: boolean; id?: string; message: string }> {
    try {
      const requestBody = {
        content,
        user_id: metadata?.user_id,
        agent_id: metadata?.agent_id,
        run_id: metadata?.run_id,
        actor_id: metadata?.actor_id,
        role: metadata?.role,
        memory_type: metadata?.memory_type,
        custom: metadata?.custom,
      };
      
      const response = await fetch(`${this.baseUrl}/memories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestBody),
      });
      
      if (!response.ok) {
        throw new Error(`Create memory failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      return {
        success: true,
        id: result.id,
        message: result.message,
      };
    } catch (error) {
      console.error('Create memory error:', error);
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to create memory',
      };
    }
  }
  
  // 更新记忆
  async updateMemory(id: string, content: string): Promise<{ success: boolean; message: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/memories/${id}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ content }),
      });
      
      if (!response.ok) {
        throw new Error(`Update memory failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      return {
        success: true,
        message: result.message,
      };
    } catch (error) {
      console.error('Update memory error:', error);
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to update memory',
      };
    }
  }
  
  // 删除记忆
  async deleteMemory(id: string): Promise<{ success: boolean; message: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/memories/${id}`, {
        method: 'DELETE',
      });
      
      if (!response.ok) {
        throw new Error(`Delete memory failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      return {
        success: true,
        message: result.message,
      };
    } catch (error) {
      console.error('Delete memory error:', error);
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Failed to delete memory',
      };
    }
  }
  
  // 批量操作
  async batchDelete(ids: string[]): Promise<{ success: boolean; message: string; failed: string[] }> {
    const failed: string[] = [];
    
    for (const id of ids) {
      try {
        await this.deleteMemory(id);
      } catch (error) {
        failed.push(id);
      }
    }
    
    return {
      success: failed.length === 0,
      message: failed.length === 0 
        ? 'All memories deleted successfully' 
        : `Failed to delete ${failed.length} memories`,
      failed,
    };
  }
  
  // 统计信息
  async getStatistics(): Promise<{
    total_memories: number;
    by_type: Record<string, number>;
    by_user: Record<string, number>;
    by_agent: Record<string, number>;
    recent_activity: Array<{ date: string; count: number }>;
  }> {
    try {
      // 获取所有记忆
      const listResponse = await this.listMemories({ limit: 1000 });
      
      // 统计类型分布
      const byType: Record<string, number> = {};
      const byUser: Record<string, number> = {};
      const byAgent: Record<string, number> = {};
      
      // 按日期统计最近活动（最近7天）
      const recentActivity: Array<{ date: string; count: number }> = [];
      const today = new Date();
      
      for (let i = 6; i >= 0; i--) {
        const date = new Date(today);
        date.setDate(date.getDate() - i);
        const dateStr = date.toISOString().split('T')[0];
        recentActivity.push({ date: dateStr, count: 0 });
      }
      
      for (const memory of listResponse.memories) {
        // 统计类型
        const type = memory.metadata.memory_type;
        byType[type] = (byType[type] || 0) + 1;
        
        // 统计用户
        if (memory.metadata.user_id) {
          byUser[memory.metadata.user_id] = (byUser[memory.metadata.user_id] || 0) + 1;
        }
        
        // 统计代理
        if (memory.metadata.agent_id) {
          byAgent[memory.metadata.agent_id] = (byAgent[memory.metadata.agent_id] || 0) + 1;
        }
        
        // 统计最近活动
        const memoryDate = new Date(memory.created_at).toISOString().split('T')[0];
        const activityEntry = recentActivity.find(a => a.date === memoryDate);
        if (activityEntry) {
          activityEntry.count++;
        }
      }
      
      return {
        total_memories: listResponse.total,
        by_type: byType,
        by_user: byUser,
        by_agent: byAgent,
        recent_activity: recentActivity,
      };
    } catch (error) {
      console.error('Get statistics error:', error);
      return {
        total_memories: 0,
        by_type: {},
        by_user: {},
        by_agent: {},
        recent_activity: [],
      };
    }
  }

  // 优化相关方法
  
  // 启动优化任务
  async optimize(params?: {
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
  }): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/optimization`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(params || {}),
      });
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('启动优化失败 - 错误响应:', errorText);
        throw new Error(`Optimize failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('启动优化错误:', error);
      return {
        success: false,
        error: {
          code: 'OPTIMIZE_FAILED',
          message: error instanceof Error ? error.message : '启动优化失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // 获取优化任务状态
  async getOptimizationStatus(jobId: string): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/optimization/${jobId}`);
      
      if (!response.ok) {
        if (response.status === 404) {
          return {
            success: false,
            error: {
              code: 'JOB_NOT_FOUND',
              message: `优化任务 ${jobId} 不存在`,
            },
            timestamp: new Date().toISOString(),
          };
        }
        throw new Error(`Get optimization status failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('获取优化状态错误:', error);
      return {
        success: false,
        error: {
          code: 'GET_STATUS_FAILED',
          message: error instanceof Error ? error.message : '获取状态失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // 取消优化任务
  async cancelOptimization(jobId: string): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/optimization/${jobId}/cancel`, {
        method: 'POST',
      });
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('取消优化失败 - 错误响应:', errorText);
        throw new Error(`Cancel optimization failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('取消优化错误:', error);
      return {
        success: false,
        error: {
          code: 'CANCEL_FAILED',
          message: error instanceof Error ? error.message : '取消优化失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // 获取优化历史
  async getOptimizationHistory(params?: {
    limit?: number;
    offset?: number;
    status?: string;
    start_date?: string;
    end_date?: string;
  }): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const queryParams = new URLSearchParams();
      if (params?.limit) queryParams.append('limit', params.limit.toString());
      if (params?.offset) queryParams.append('offset', params.offset.toString());
      if (params?.status) queryParams.append('status', params.status);
      if (params?.start_date) queryParams.append('start_date', params.start_date);
      if (params?.end_date) queryParams.append('end_date', params.end_date);
      
      const url = `${this.baseUrl}/optimization/history${queryParams.toString() ? `?${queryParams}` : ''}`;
      const response = await fetch(url);
      
      if (!response.ok) {
        throw new Error(`Get optimization history failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('获取优化历史错误:', error);
      return {
        success: false,
        error: {
          code: 'GET_HISTORY_FAILED',
          message: error instanceof Error ? error.message : '获取历史失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // 分析优化问题（预览模式）
  async analyzeOptimization(params?: {
    memory_type?: string;
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    similarity_threshold?: number;
  }): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/optimization/analyze`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(params || {}),
      });
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('分析优化失败 - 错误响应:', errorText);
        throw new Error(`Analyze optimization failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('分析优化错误:', error);
      return {
        success: false,
        error: {
          code: 'ANALYZE_FAILED',
          message: error instanceof Error ? error.message : '分析失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // 获取优化统计
  async getOptimizationStatistics(): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/optimization/statistics`);
      
      if (!response.ok) {
        throw new Error(`Get optimization statistics failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('获取优化统计错误:', error);
      return {
        success: false,
        error: {
          code: 'GET_STATISTICS_FAILED',
          message: error instanceof Error ? error.message : '获取统计失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // 清理优化历史
  async cleanupOptimizationHistory(maxAgeDays?: number): Promise<{ success: boolean; data?: any; error?: any; timestamp: string }> {
    try {
      const response = await fetch(`${this.baseUrl}/optimization/cleanup`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ max_age_days: maxAgeDays || 7 }),
      });
      
      if (!response.ok) {
        throw new Error(`Cleanup optimization history failed: ${response.statusText}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('清理优化历史错误:', error);
      return {
        success: false,
        error: {
          code: 'CLEANUP_FAILED',
          message: error instanceof Error ? error.message : '清理失败',
        },
        timestamp: new Date().toISOString(),
      };
    }
  }

  // LLM服务状态检测
  
  // 获取详细的LLM服务状态
  async getLLMStatus(): Promise<{
    overall_status: string;
    completion_model: {
      available: boolean;
      provider: string;
      model_name: string;
      latency_ms?: number;
      error_message?: string;
      last_check: string;
    };
    embedding_model: {
      available: boolean;
      provider: string;
      model_name: string;
      latency_ms?: number;
      error_message?: string;
      last_check: string;
    };
    timestamp: string;
  }> {
    const response = await fetch(`${this.baseUrl}/llm/status`);
    
    if (!response.ok) {
      throw new Error(`Get LLM status failed: ${response.statusText}`);
    }
    
    return await response.json();
  }

  // 简单的LLM健康检查
  async llmHealthCheck(): Promise<{
    completion_model_available: boolean;
    embedding_model_available: boolean;
    timestamp: string;
  }> {
    const response = await fetch(`${this.baseUrl}/llm/health-check`);
    
    if (!response.ok) {
      throw new Error(`LLM health check failed: ${response.statusText}`);
    }
    
    return await response.json();
  }
}

// 创建默认客户端实例
export const cortexMemService = new CortexMemServiceClient(
  process.env.CORTEX_MEM_SERVICE_URL || 'http://localhost:3000'
);