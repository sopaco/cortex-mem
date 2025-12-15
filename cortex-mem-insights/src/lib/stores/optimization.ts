import { writable, derived } from 'svelte/store';
import { optimizationApi } from '../api/client';
import type { OptimizationResult, OptimizationHistory } from '../../server/api/types';

// 优化状态
interface OptimizationState {
  currentJob: OptimizationResult | null;
  history: OptimizationHistory[];
  loading: boolean;
  error: string | null;
  filters: {
    status?: string;
    start_date?: string;
    end_date?: string;
  };
  pagination: {
    page: number;
    limit: number;
    total: number;
  };
}

// 初始状态
const initialState: OptimizationState = {
  currentJob: null,
  history: [],
  loading: false,
  error: null,
  filters: {
    status: undefined,
    start_date: undefined,
    end_date: undefined,
  },
  pagination: {
    page: 1,
    limit: 20,
    total: 0,
  },
};

// 创建store
function createOptimizationStore() {
  const { subscribe, set, update } = writable<OptimizationState>(initialState);

  return {
    subscribe,
    
    // 执行优化
    runOptimization: async (params?: {
      memory_type?: string;
      user_id?: string;
      agent_id?: string;
      run_id?: string;
      actor_id?: string;
      similarity_threshold?: number;
      dry_run?: boolean;
      verbose?: boolean;
    }) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await optimizationApi.optimize(params);
        
        update(state => ({
          ...state,
          currentJob: response.data,
          loading: false,
        }));
        
        return { success: true, jobId: response.data?.job_id };
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to run optimization',
        }));
        
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to run optimization',
        };
      }
    },
    
    // 获取优化状态
    getJobStatus: async (jobId: string) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await optimizationApi.getStatus(jobId);
        
        update(state => ({
          ...state,
          currentJob: response.data,
          loading: false,
        }));
        
        return { success: true, job: response.data };
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to get job status',
        }));
        
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to get job status',
        };
      }
    },
    
    // 加载优化历史
    loadHistory: async (params?: {
      page?: number;
      limit?: number;
      status?: string;
      start_date?: string;
      end_date?: string;
    }) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await optimizationApi.history({
          limit: params?.limit || initialState.pagination.limit,
          offset: ((params?.page || 1) - 1) * (params?.limit || initialState.pagination.limit),
          status: params?.status,
          start_date: params?.start_date,
          end_date: params?.end_date,
        });
        
        update(state => ({
          ...state,
          history: response.data?.history || [],
          loading: false,
          filters: {
            ...state.filters,
            status: params?.status,
            start_date: params?.start_date,
            end_date: params?.end_date,
          },
          pagination: {
            ...state.pagination,
            page: params?.page || 1,
            limit: params?.limit || state.pagination.limit,
            total: response.data?.total || 0,
          },
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load optimization history',
        }));
      }
    },
    
    // 取消优化
    cancelOptimization: async (jobId: string) => {
      try {
        await optimizationApi.cancel(jobId);
        
        update(state => {
          if (state.currentJob?.job_id === jobId) {
            return {
              ...state,
              currentJob: {
                ...state.currentJob,
                status: 'failed',
                message: 'Optimization cancelled by user',
              },
            };
          }
          
          return {
            ...state,
            history: state.history.map(job => 
              job.job_id === jobId 
                ? { ...job, status: 'cancelled' }
                : job
            ),
          };
        });
        
        return { success: true };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to cancel optimization',
        };
      }
    },
    
    // 获取优化统计
    loadStatistics: async () => {
      try {
        const response = await optimizationApi.statistics();
        return { success: true, statistics: response.data };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to load optimization statistics',
        };
      }
    },
    
    // 设置过滤器
    setFilter: (filter: keyof OptimizationState['filters'], value: string | undefined) => {
      update(state => ({
        ...state,
        filters: {
          ...state.filters,
          [filter]: value,
        },
      }));
    },
    
    // 清除过滤器
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
    
    // 清除当前任务
    clearCurrentJob: () => {
      update(state => ({
        ...state,
        currentJob: null,
      }));
    },
    
    // 重置状态
    reset: () => {
      set(initialState);
    },
  };
}

export const optimizationStore = createOptimizationStore();

// 优化统计状态
interface OptimizationStatsState {
  statistics: {
    total_jobs: number;
    successful_jobs: number;
    failed_jobs: number;
    cancelled_jobs: number;
    total_memories_processed: number;
    total_memories_deduplicated: number;
    total_memories_merged: number;
    total_memories_enhanced: number;
    avg_duration: number;
    last_run: string | null;
  } | null;
  loading: boolean;
  error: string | null;
}

function createOptimizationStatsStore() {
  const { subscribe, set, update } = writable<OptimizationStatsState>({
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
        const response = await optimizationApi.statistics();
        update(state => ({ ...state, statistics: response.data, loading: false }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load optimization statistics',
        }));
      }
    },
    
    // 清除状态
    clear: () => {
      set({ statistics: null, loading: false, error: null });
    },
  };
}

export const optimizationStatsStore = createOptimizationStatsStore();

// 导出派生store
export const optimizationStatus = derived(optimizationStore, ($optimization) => {
  if (!$optimization.currentJob) return null;
  
  return {
    jobId: $optimization.currentJob.job_id,
    status: $optimization.currentJob.status,
    progress: $optimization.currentJob.processed_memories / $optimization.currentJob.total_memories * 100,
    message: $optimization.currentJob.message,
    duration: $optimization.currentJob.duration,
  };
});

export const recentOptimizations = derived(optimizationStore, ($optimization) => {
  return $optimization.history
    .sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime())
    .slice(0, 10);
});

export const optimizationMetrics = derived(optimizationStatsStore, ($stats) => {
  if (!$stats.statistics) return null;
  
  return {
    successRate: $stats.statistics.total_jobs > 0 
      ? ($stats.statistics.successful_jobs / $stats.statistics.total_jobs) * 100 
      : 0,
    avgMemoriesPerJob: $stats.statistics.total_jobs > 0
      ? $stats.statistics.total_memories_processed / $stats.statistics.total_jobs
      : 0,
    deduplicationRate: $stats.statistics.total_memories_processed > 0
      ? ($stats.statistics.total_memories_deduplicated / $stats.statistics.total_memories_processed) * 100
      : 0,
    mergeRate: $stats.statistics.total_memories_processed > 0
      ? ($stats.statistics.total_memories_merged / $stats.statistics.total_memories_processed) * 100
      : 0,
    enhancementRate: $stats.statistics.total_memories_processed > 0
      ? ($stats.statistics.total_memories_enhanced / $stats.statistics.total_memories_processed) * 100
      : 0,
  };
});