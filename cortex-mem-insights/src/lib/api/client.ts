// API 客户端配置
// 在开发模式下使用相对路径，由Vite代理到API服务器
// 在生产模式下使用环境变量配置的URL
const API_BASE_URL = import.meta.env.VITE_API_URL || '';

// 通用请求函数
async function request<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  
  const defaultOptions: RequestInit = {
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    credentials: 'include',
  };
  
  try {
    const response = await fetch(url, { ...defaultOptions, ...options });
    
    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        errorData.error?.message || 
        errorData.message || 
        `HTTP ${response.status}: ${response.statusText}`
      );
    }
    
    return await response.json();
  } catch (error) {
    console.error(`API request failed: ${endpoint}`, error);
    throw error;
  }
}

// 记忆相关API
export const memoryApi = {
  // 获取记忆列表
  list: (params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
    page?: number;
  }) => {
    const queryParams = new URLSearchParams();
    if (params?.user_id) queryParams.append('user_id', params.user_id);
    if (params?.agent_id) queryParams.append('agent_id', params.agent_id);
    if (params?.run_id) queryParams.append('run_id', params.run_id);
    if (params?.actor_id) queryParams.append('actor_id', params.actor_id);
    if (params?.memory_type) queryParams.append('memory_type', params.memory_type);
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.page) queryParams.append('page', params.page.toString());
    
    return request(`/api/memories${queryParams.toString() ? `?${queryParams}` : ''}`);
  },
  
  // 搜索记忆
  search: (query: string, params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
    similarity_threshold?: number;
  }) => {
    return request('/api/memories/search', {
      method: 'POST',
      body: JSON.stringify({ query, ...params }),
    });
  },
  
  // 获取单个记忆
  get: (id: string) => {
    return request(`/api/memories/${id}`);
  },
  
  // 创建记忆
  create: (content: string, metadata?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    role?: string;
    memory_type?: string;
    custom?: Record<string, any>;
  }) => {
    return request('/api/memories', {
      method: 'POST',
      body: JSON.stringify({ content, ...metadata }),
    });
  },
  
  // 更新记忆
  update: (id: string, content: string) => {
    return request(`/api/memories/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ content }),
    });
  },
  
  // 删除记忆
  delete: (id: string) => {
    return request(`/api/memories/${id}`, {
      method: 'DELETE',
    });
  },
  
  // 批量删除
  batchDelete: (ids: string[]) => {
    return request('/api/memories/batch/delete', {
      method: 'POST',
      body: JSON.stringify({ ids }),
    });
  },

  // 批量更新
  batchUpdate: (updates: { id: string; content: string }[]) => {
    return request('/api/memories/batch/update', {
      method: 'POST',
      body: JSON.stringify({ updates }),
    });
  },
  
  // 获取统计信息
  statistics: () => {
    return request('/api/memories/stats/summary');
  },
  
  // 导出记忆
  export: (params: {
    format: 'json' | 'csv' | 'txt';
    ids?: string[];
    filters?: any;
    include_metadata?: boolean;
    include_scores?: boolean;
  }) => {
    return request('/api/memories/export', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  },
};

// 优化相关API
export const optimizationApi = {
  // 执行优化
  optimize: (params?: {
    memory_type?: string;
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    similarity_threshold?: number;
    dry_run?: boolean;
    verbose?: boolean;
  }) => {
    return request('/api/optimization', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  },
  
  // 获取优化状态
  getStatus: (jobId: string) => {
    return request(`/api/optimization/${jobId}`);
  },
  
  // 获取优化历史
  history: (params?: {
    limit?: number;
    offset?: number;
    status?: string;
    start_date?: string;
    end_date?: string;
  }) => {
    const queryParams = new URLSearchParams();
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.offset) queryParams.append('offset', params.offset.toString());
    if (params?.status) queryParams.append('status', params.status);
    if (params?.start_date) queryParams.append('start_date', params.start_date);
    if (params?.end_date) queryParams.append('end_date', params.end_date);
    
    return request(`/api/optimization/history${queryParams.toString() ? `?${queryParams}` : ''}`);
  },
  
  // 取消优化
  cancel: (jobId: string) => {
    return request(`/api/optimization/${jobId}/cancel`, {
      method: 'POST',
    });
  },
  
  // 分析优化问题（预览模式）
  analyze: (params?: {
    memory_type?: string;
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    similarity_threshold?: number;
  }) => {
    return request('/api/optimization/analyze', {
      method: 'POST',
      body: JSON.stringify(params || {}),
    });
  },
  
  // 获取优化统计
  statistics: () => {
    return request('/api/optimization/statistics');
  },
};

// 系统相关API
export const systemApi = {
  // 健康检查
  health: () => {
    return request('/health');
  },
  
  // 系统状态
  status: () => {
    return request('/api/system/status');
  },
  
  // 性能指标
  metrics: () => {
    return request('/api/system/metrics');
  },
  
  // 系统信息
  info: () => {
    return request('/api/system/info');
  },
  
  // 实时日志
  logs: (params?: {
    limit?: number;
    level?: string;
    source?: string;
  }) => {
    const queryParams = new URLSearchParams();
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.level) queryParams.append('level', params.level);
    if (params?.source) queryParams.append('source', params.source);
    
    return request(`/api/system/logs${queryParams.toString() ? `?${queryParams}` : ''}`);
  },
  
  // 资源使用情况
  resources: () => {
    return request('/api/system/resources');
  },
  
  // 清理缓存
  clearCache: () => {
    return request('/api/system/clear-cache', {
      method: 'POST',
    });
  },
  
  // 重启服务
  restart: () => {
    return request('/api/system/restart', {
      method: 'POST',
    });
  },
};

// 通用API
export const api = {
  // 测试连接
  testConnection: async () => {
    try {
      const response = await request('/health');
      return {
        connected: true,
        response,
      };
    } catch (error) {
      return {
        connected: false,
        error: error instanceof Error ? error.message : 'Connection failed',
      };
    }
  },
  
  // 获取所有服务状态
  getAllStatus: async () => {
    try {
      const [health, systemStatus, metrics] = await Promise.all([
        systemApi.health(),
        systemApi.status(),
        systemApi.metrics(),
      ]);
      
      return {
        success: true,
        health,
        systemStatus,
        metrics,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to get system status',
      };
    }
  },
};

// 导出所有API
export default {
  memory: memoryApi,
  optimization: optimizationApi,
  system: systemApi,
  api,
};