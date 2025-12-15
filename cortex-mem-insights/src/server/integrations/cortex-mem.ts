import { MemoryResponse, SearchResponse, ListResponse, HealthResponse } from '../api/types';

// Cortex-mem-service API 客户端
export class CortexMemServiceClient {
  private baseUrl: string;
  private useMockData: boolean;
  
  constructor(baseUrl: string = 'http://localhost:3000', useMockData: boolean = false) {
    this.baseUrl = baseUrl;
    this.useMockData = useMockData || process.env.MOCK_CORTEX_MEM === 'true';
  }
  
  // 健康检查
  async healthCheck(): Promise<HealthResponse> {
    try {
      // 如果使用Mock数据，返回健康状态
      if (this.useMockData) {
        return {
          status: 'healthy',
          vector_store: true,
          llm_service: true,
          timestamp: new Date().toISOString(),
          mock_mode: true
        };
      }
      
      const response = await fetch(`${this.baseUrl}/health`);
      if (!response.ok) {
        throw new Error(`Health check failed: ${response.statusText}`);
      }
      return await response.json();
    } catch (error) {
      console.error('Health check error:', error);
      return {
        status: 'unhealthy',
        vector_store: false,
        llm_service: false,
        timestamp: new Date().toISOString(),
        mock_mode: false
      };
    }
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
      console.log('获取记忆列表 - 参数:', params);
      
      // 如果启用了mock数据，返回mock数据
      if (this.useMockData) {
        console.log('使用Mock数据');
        return this.getMockMemories(params);
      }
      
      const queryParams = new URLSearchParams();
      if (params?.user_id) queryParams.append('user_id', params.user_id);
      if (params?.agent_id) queryParams.append('agent_id', params.agent_id);
      if (params?.run_id) queryParams.append('run_id', params.run_id);
      if (params?.actor_id) queryParams.append('actor_id', params.actor_id);
      if (params?.memory_type) queryParams.append('memory_type', params.memory_type);
      if (params?.limit) queryParams.append('limit', params.limit.toString());
      
      const url = `${this.baseUrl}/memories${queryParams.toString() ? `?${queryParams}` : ''}`;
      console.log('获取记忆列表 - 目标URL:', url);
      
      const response = await fetch(url);
      console.log('获取记忆列表 - 响应状态:', response.status, response.statusText);
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('获取记忆列表失败 - 错误响应:', errorText);
        throw new Error(`List memories failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      console.log('获取记忆列表 - 成功结果，总数:', result.total);
      return result;
    } catch (error) {
      console.error('获取记忆列表错误:', error);
      // 如果实际服务失败，尝试使用mock数据作为回退
      if (!this.useMockData) {
        console.log('实际服务失败，回退到Mock数据');
        return this.getMockMemories(params);
      }
      return {
        total: 0,
        memories: [],
      };
    }
  }
  
  // 获取Mock记忆数据
  private getMockMemories(params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
  }): ListResponse {
    console.log('生成Mock记忆数据');
    
    // 基本mock数据
    const mockMemories = [
      {
        id: '023f3938-9d1f-42e8-a70d-d9ddf9e27bf0',
        content: '用户SkyronJ确认其在2026年1月将有一笔收入，来源于快手科技2025年度的年终奖发放。互联网公司通常在次年第一季度发放年终奖，此信息符合行业惯例。该信息可用于后续财务规划、目标设定或生活决策类对话的上下文支持。',
        metadata: {
          user_id: 'SkyronJ',
          agent_id: null,
          run_id: null,
          actor_id: null,
          role: null,
          memory_type: 'Personal',
          hash: 'fd8777390ba83d10ad1a621094829fcae26f5f8aac3b05b2c2b07e44177093ae',
          custom: {}
        },
        created_at: '2025-12-12T08:36:49.038512+00:00',
        updated_at: '2025-12-12T08:36:49.038512+00:00'
      },
      {
        id: '024c539e-dd3c-4671-813f-51005f0cfe33',
        content: '我已经了解了用户Jiang Meng的cortex-mem项目。这是一个基于Rust开发的AI智能体持久化记忆管理系统，旨在为AI代理提供结构化、可检索、可优化的记忆能力。',
        metadata: {
          user_id: 'jiangmeng',
          agent_id: 'zed_agent',
          run_id: null,
          actor_id: null,
          role: null,
          memory_type: 'Personal',
          hash: 'd6ad6d7cb5f93188983744dada52acc40432cd60075305d20bf5d897d11a62b2',
          custom: {}
        },
        created_at: '2025-12-11T09:44:27.158048+00:00',
        updated_at: '2025-12-11T09:44:27.158048+00:00'
      },
      {
        id: '05bd5411-be26-487d-aedd-3b26a8e46ff2',
        content: '当前面临核心管理困境：一名37岁女性员工因年龄、薪酬与绩效潜力不匹配，已被公司人才盘点系统标记为优化对象。',
        metadata: {
          user_id: 'SkyronJ',
          agent_id: null,
          run_id: null,
          actor_id: null,
          role: null,
          memory_type: 'Episodic',
          hash: '8cd29406940fe32d76ff681ea70603e60a841d355eed16e438e12025a766ff12',
          custom: {}
        },
        created_at: '2025-12-14T06:29:05.135651+00:00',
        updated_at: '2025-12-14T06:29:05.135651+00:00'
      },
      {
        id: '071a4916-afea-4d6f-b5de-7916b630ebc7',
        content: '用户是cortex-mem项目的开发者，这是一个用Rust编写的AI智能体持久化记忆管理系统。',
        metadata: {
          user_id: 'user_of_zed_agent',
          agent_id: 'zed_agent',
          run_id: null,
          actor_id: null,
          role: null,
          memory_type: 'Factual',
          hash: '954eb88f16c2b1304d1c7b5278335183ef03e0cb79f281bc71a0aa5ee539208b',
          custom: {}
        },
        created_at: '2025-12-10T03:08:21.738183+00:00',
        updated_at: '2025-12-10T03:08:21.738183+00:00'
      }
    ];
    
    // 应用过滤
    let filteredMemories = [...mockMemories];
    
    if (params?.user_id) {
      filteredMemories = filteredMemories.filter(m => m.metadata.user_id === params.user_id);
    }
    
    if (params?.agent_id) {
      filteredMemories = filteredMemories.filter(m => m.metadata.agent_id === params.agent_id);
    }
    
    if (params?.memory_type) {
      filteredMemories = filteredMemories.filter(m => m.metadata.memory_type === params.memory_type);
    }
    
    // 应用限制
    if (params?.limit) {
      filteredMemories = filteredMemories.slice(0, params.limit);
    }
    
    return {
      total: filteredMemories.length,
      memories: filteredMemories
    };
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
      console.log('搜索记忆 - 查询:', query);
      console.log('搜索记忆 - 参数:', params);
      
      // 如果启用了mock数据，返回mock搜索结果
      if (this.useMockData) {
        console.log('使用Mock搜索数据');
        return this.getMockSearchResults(query, params);
      }
      
      console.log('搜索记忆 - 目标URL:', `${this.baseUrl}/memories/search`);
      
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
      
      console.log('搜索记忆 - 请求体:', JSON.stringify(requestBody));
      
      const response = await fetch(`${this.baseUrl}/memories/search`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestBody),
      });
      
      console.log('搜索记忆 - 响应状态:', response.status, response.statusText);
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('搜索记忆失败 - 错误响应:', errorText);
        throw new Error(`Search memories failed: ${response.statusText}`);
      }
      
      const result = await response.json();
      console.log('搜索记忆 - 成功结果:', result);
      return result;
    } catch (error) {
      console.error('搜索记忆错误:', error);
      // 如果实际服务失败，尝试使用mock数据作为回退
      if (!this.useMockData) {
        console.log('实际服务失败，回退到Mock搜索数据');
        return this.getMockSearchResults(query, params);
      }
      return {
        total: 0,
        results: [],
      };
    }
  }
  
  // 获取Mock搜索结果
  private getMockSearchResults(query: string, params?: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    limit?: number;
    similarity_threshold?: number;
  }): SearchResponse {
    console.log('生成Mock搜索结果');
    
    // 获取所有mock记忆
    const allMemories = this.getMockMemories(params).memories;
    
    // 简单的文本匹配搜索
    const results = allMemories
      .map(memory => ({
        memory,
        score: this.calculateSimpleSimilarity(query, memory.content)
      }))
      .filter(result => result.score > (params?.similarity_threshold || 0.1))
      .sort((a, b) => b.score - a.score);
    
    // 应用限制
    const limitedResults = params?.limit ? results.slice(0, params.limit) : results;
    
    return {
      total: limitedResults.length,
      results: limitedResults
    };
  }
  
  // 简单的文本相似度计算
  private calculateSimpleSimilarity(query: string, text: string): number {
    if (!query.trim()) return 0;
    
    const queryLower = query.toLowerCase();
    const textLower = text.toLowerCase();
    
    // 简单的包含检查
    if (textLower.includes(queryLower)) {
      // 根据查询在文本中的位置和频率计算分数
      const matches = textLower.split(queryLower).length - 1;
      const positionScore = 1 - (textLower.indexOf(queryLower) / textLower.length);
      return Math.min(0.95, 0.5 + (matches * 0.1) + (positionScore * 0.3));
    }
    
    // 检查单词匹配
    const queryWords = queryLower.split(/\s+/);
    const textWords = textLower.split(/\s+/);
    const matchingWords = queryWords.filter(word => textWords.includes(word));
    
    if (matchingWords.length > 0) {
      return Math.min(0.8, matchingWords.length / queryWords.length);
    }
    
    return 0.1;
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
}

// 创建默认客户端实例
// 默认启用Mock数据，可以通过环境变量 MOCK_CORTEX_MEM=false 禁用
export const cortexMemService = new CortexMemServiceClient(
  process.env.CORTEX_MEM_SERVICE_URL || 'http://localhost:3000',
  process.env.MOCK_CORTEX_MEM !== 'false'
);