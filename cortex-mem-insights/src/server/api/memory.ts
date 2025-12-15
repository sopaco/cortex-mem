import { Elysia, t } from 'elysia';
import { cortexMemService } from '../integrations/cortex-mem';

// 类型定义
interface MemoryResponse {
  id: string;
  content: string;
  metadata: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    role?: string;
    memory_type: string;
    hash: string;
    custom: Record<string, any>;
  };
  created_at: string;
  updated_at: string;
}

interface ListResponse {
  total: number;
  memories: MemoryResponse[];
}

interface SearchRequest {
  query: string;
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  memory_type?: string;
  limit?: number;
  similarity_threshold?: number;
}

interface SearchResponse {
  total: number;
  results: Array<{
    memory: MemoryResponse;
    score: number;
  }>;
}

// 内存API路由
export const memoryRoutes = new Elysia({ prefix: '/api/memories' })
  // 获取记忆列表
  .get('/', async ({ query }) => {
    try {
      const response = await cortexMemService.listMemories({
        user_id: query.user_id,
        agent_id: query.agent_id,
        run_id: query.run_id,
        actor_id: query.actor_id,
        memory_type: query.memory_type,
        limit: query.limit ? parseInt(query.limit) : undefined
      });
      
      return {
        total: response.total,
        memories: response.memories
      };
    } catch (error) {
      console.error('获取记忆列表失败:', error);
      throw error;
    }
  }, {
    query: t.Object({
      user_id: t.Optional(t.String()),
      agent_id: t.Optional(t.String()),
      run_id: t.Optional(t.String()),
      actor_id: t.Optional(t.String()),
      memory_type: t.Optional(t.String()),
      limit: t.Optional(t.String())
    })
  })
  
  // 搜索记忆
  .post('/search', async ({ body }) => {
    try {
      const { query, ...params } = body;
      const response = await cortexMemService.searchMemories(query, params);
      return response;
    } catch (error) {
      console.error('搜索记忆失败:', error);
      throw error;
    }
  }, {
    body: t.Object({
      query: t.String(),
      user_id: t.Optional(t.String()),
      agent_id: t.Optional(t.String()),
      run_id: t.Optional(t.String()),
      actor_id: t.Optional(t.String()),
      memory_type: t.Optional(t.String()),
      limit: t.Optional(t.Number()),
      similarity_threshold: t.Optional(t.Number())
    })
  })
  
  // 获取单个记忆
  .get('/:id', async ({ params }) => {
    try {
      const memory = await cortexMemService.getMemory(params.id);
      return memory;
    } catch (error) {
      console.error(`获取记忆 ${params.id} 失败:`, error);
      throw error;
    }
  }, {
    params: t.Object({
      id: t.String()
    })
  })
  
  // 创建记忆
  .post('/', async ({ body }) => {
    try {
      const response = await cortexMemService.createMemory(body);
      return response;
    } catch (error) {
      console.error('创建记忆失败:', error);
      throw error;
    }
  }, {
    body: t.Object({
      content: t.String(),
      user_id: t.Optional(t.String()),
      agent_id: t.Optional(t.String()),
      run_id: t.Optional(t.String()),
      actor_id: t.Optional(t.String()),
      role: t.Optional(t.String()),
      memory_type: t.Optional(t.String()),
      custom: t.Optional(t.Record(t.String(), t.Any()))
    })
  })
  
  // 更新记忆
  .put('/:id', async ({ params, body }) => {
    try {
      const response = await cortexMemService.updateMemory(params.id, body.content);
      return response;
    } catch (error) {
      console.error(`更新记忆 ${params.id} 失败:`, error);
      throw error;
    }
  }, {
    params: t.Object({
      id: t.String()
    }),
    body: t.Object({
      content: t.String()
    })
  })
  
  // 删除记忆
  .delete('/:id', async ({ params }) => {
    try {
      const response = await cortexMemService.deleteMemory(params.id);
      return response;
    } catch (error) {
      console.error(`删除记忆 ${params.id} 失败:`, error);
      throw error;
    }
  }, {
    params: t.Object({
      id: t.String()
    })
  })
  
  // 获取统计信息
  .get('/stats/summary', async () => {
    try {
      const memories = await cortexMemService.listMemories({});
      
      // 计算基本统计
      const total = memories.total;
      const types = memories.memories.reduce((acc, memory) => {
        const type = memory.metadata.memory_type;
        acc[type] = (acc[type] || 0) + 1;
        return acc;
      }, {} as Record<string, number>);
      
      // 按用户分组
      const users = memories.memories.reduce((acc, memory) => {
        const userId = memory.metadata.user_id || 'unknown';
        acc[userId] = (acc[userId] || 0) + 1;
        return acc;
      }, {} as Record<string, number>);
      
      // 最近记忆
      const recent = memories.memories
        .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
        .slice(0, 10);
      
      return {
        total,
        types,
        users: Object.keys(users).length,
        user_distribution: users,
        recent_count: recent.length
      };
    } catch (error) {
      console.error('获取统计信息失败:', error);
      throw error;
    }
  })
  
  // 获取类型分布
  .get('/stats/types', async () => {
    try {
      const memories = await cortexMemService.listMemories({});
      
      const typeDistribution = memories.memories.reduce((acc, memory) => {
        const type = memory.metadata.memory_type;
        acc[type] = (acc[type] || 0) + 1;
        return acc;
      }, {} as Record<string, number>);
      
      return {
        distribution: typeDistribution,
        total: memories.total
      };
    } catch (error) {
      console.error('获取类型分布失败:', error);
      throw error;
    }
  })
  
  // 批量操作
  .post('/batch/delete', async ({ body }) => {
    try {
      const results = await Promise.allSettled(
        body.ids.map((id: string) => cortexMemService.deleteMemory(id))
      );
      
      const succeeded = results.filter(r => r.status === 'fulfilled').length;
      const failed = results.filter(r => r.status === 'rejected').length;
      
      return {
        total: body.ids.length,
        succeeded,
        failed,
        results: results.map((r, i) => ({
          id: body.ids[i],
          status: r.status,
          error: r.status === 'rejected' ? (r as PromiseRejectedResult).reason.message : undefined
        }))
      };
    } catch (error) {
      console.error('批量删除失败:', error);
      throw error;
    }
  }, {
    body: t.Object({
      ids: t.Array(t.String())
    })
  });