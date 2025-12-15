import { writable, derived } from 'svelte/store';
import { memoryApi } from '../api/client';
import type { MemoryResponse, SearchResponse, ListResponse } from '../../server/api/types';

// 记忆列表状态
interface MemoryListState {
  memories: MemoryResponse[];
  total: number;
  loading: boolean;
  error: string | null;
  filters: {
    user_id?: string;
    agent_id?: string;
    run_id?: string;
    actor_id?: string;
    memory_type?: string;
    search_query?: string;
  };
  pagination: {
    page: number;
    limit: number;
    total_pages: number;
  };
}

// 初始状态
const initialState: MemoryListState = {
  memories: [],
  total: 0,
  loading: false,
  error: null,
  filters: {
    user_id: undefined,
    agent_id: undefined,
    run_id: undefined,
    actor_id: undefined,
    memory_type: undefined,
    search_query: undefined,
  },
  pagination: {
    page: 1,
    limit: 20,
    total_pages: 1,
  },
};

// 创建store
function createMemoryStore() {
  const { subscribe, set, update } = writable<MemoryListState>(initialState);

  return {
    subscribe,
    
    // 加载记忆列表
    loadMemories: async (params?: {
      page?: number;
      limit?: number;
      user_id?: string;
      agent_id?: string;
      run_id?: string;
      actor_id?: string;
      memory_type?: string;
    }) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await memoryApi.list({
          ...params,
          limit: params?.limit || initialState.pagination.limit,
        }) as ListResponse;
        
        update(state => ({
          ...state,
          memories: response.memories,
          total: response.total,
          loading: false,
          filters: {
            ...state.filters,
            user_id: params?.user_id,
            agent_id: params?.agent_id,
            run_id: params?.run_id,
            actor_id: params?.actor_id,
            memory_type: params?.memory_type,
          },
          pagination: {
            ...state.pagination,
            page: params?.page || 1,
            limit: params?.limit || state.pagination.limit,
            total_pages: Math.ceil(response.total / (params?.limit || state.pagination.limit)),
          },
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load memories',
        }));
      }
    },
    
    // 搜索记忆
    searchMemories: async (query: string, params?: {
      user_id?: string;
      agent_id?: string;
      run_id?: string;
      actor_id?: string;
      memory_type?: string;
      limit?: number;
      similarity_threshold?: number;
    }) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await memoryApi.search(query, params) as SearchResponse;
        
        update(state => ({
          ...state,
          memories: response.results.map(r => r.memory),
          total: response.total,
          loading: false,
          filters: {
            ...state.filters,
            user_id: params?.user_id,
            agent_id: params?.agent_id,
            run_id: params?.run_id,
            actor_id: params?.actor_id,
            memory_type: params?.memory_type,
            search_query: query,
          },
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to search memories',
        }));
      }
    },
    
    // 清除搜索
    clearSearch: () => {
      update(state => ({
        ...state,
        filters: {
          ...state.filters,
          search_query: undefined,
        },
      }));
    },
    
    // 设置过滤器
    setFilter: (filter: keyof MemoryListState['filters'], value: string | undefined) => {
      update(state => ({
        ...state,
        filters: {
          ...state.filters,
          [filter]: value,
        },
      }));
    },
    
    // 清除所有过滤器
    clearFilters: () => {
      update(state => ({
        ...state,
        filters: initialState.filters,
      }));
    },
    
    // 设置分页
    setPagination: (page: number, limit?: number) => {
      update(state => ({
        ...state,
        pagination: {
          ...state.pagination,
          page,
          limit: limit || state.pagination.limit,
        },
      }));
    },
    
    // 删除记忆
    deleteMemory: async (id: string) => {
      try {
        await memoryApi.delete(id);
        
        update(state => ({
          ...state,
          memories: state.memories.filter(memory => memory.id !== id),
          total: state.total - 1,
        }));
        
        return { success: true };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to delete memory',
        };
      }
    },
    
    // 批量删除
    batchDelete: async (ids: string[]) => {
      try {
        await memoryApi.batchDelete(ids);
        
        update(state => ({
          ...state,
          memories: state.memories.filter(memory => !ids.includes(memory.id)),
          total: state.total - ids.length,
        }));
        
        return { success: true, deleted: ids.length };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to delete memories',
        };
      }
    },
    
    // 重置状态
    reset: () => {
      set(initialState);
    },
  };
}

export const memoryStore = createMemoryStore();

// 单个记忆状态
interface MemoryDetailState {
  memory: MemoryResponse | null;
  loading: boolean;
  error: string | null;
}

function createMemoryDetailStore() {
  const { subscribe, set, update } = writable<MemoryDetailState>({
    memory: null,
    loading: false,
    error: null,
  });

  return {
    subscribe,
    
    // 加载记忆详情
    loadMemory: async (id: string) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const memory = await memoryApi.get(id) as MemoryResponse;
        update(state => ({ ...state, memory, loading: false }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load memory',
        }));
      }
    },
    
    // 更新记忆
    updateMemory: async (id: string, content: string) => {
      try {
        await memoryApi.update(id, content);
        
        update(state => {
          if (state.memory?.id === id) {
            return {
              ...state,
              memory: {
                ...state.memory,
                content,
                updated_at: new Date().toISOString(),
              },
            };
          }
          return state;
        });
        
        return { success: true };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to update memory',
        };
      }
    },
    
    // 清除状态
    clear: () => {
      set({ memory: null, loading: false, error: null });
    },
  };
}

export const memoryDetailStore = createMemoryDetailStore();

// 记忆统计状态
interface MemoryStatsState {
  statistics: {
    total_memories: number;
    by_type: Record<string, number>;
    by_user: Record<string, number>;
    by_agent: Record<string, number>;
    recent_activity: Array<{ date: string; count: number }>;
  } | null;
  loading: boolean;
  error: string | null;
}

function createMemoryStatsStore() {
  const { subscribe, set, update } = writable<MemoryStatsState>({
    statistics: null,
    loading: false,
    error: null,
  });

  return {
    subscribe,
    
    // 加载统计信息
    loadStatistics: async () => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const statistics = await memoryApi.statistics();
        update(state => ({ ...state, statistics, loading: false }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load statistics',
        }));
      }
    },
    
    // 清除状态
    clear: () => {
      set({ statistics: null, loading: false, error: null });
    },
  };
}

export const memoryStatsStore = createMemoryStatsStore();

// 导出派生store
export const memoryTypes = derived(memoryStatsStore, ($stats) => {
  if (!$stats.statistics) return [];
  
  return Object.entries($stats.statistics.by_type)
    .map(([type, count]) => ({ type, count }))
    .sort((a, b) => b.count - a.count);
});

export const topUsers = derived(memoryStatsStore, ($stats) => {
  if (!$stats.statistics) return [];
  
  return Object.entries($stats.statistics.by_user)
    .map(([user, count]) => ({ user, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);
});

export const topAgents = derived(memoryStatsStore, ($stats) => {
  if (!$stats.statistics) return [];
  
  return Object.entries($stats.statistics.by_agent)
    .map(([agent, count]) => ({ agent, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);
});