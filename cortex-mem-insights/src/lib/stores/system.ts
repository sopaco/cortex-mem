import { writable, derived } from 'svelte/store';
import { systemApi } from '../api/client';
import type { SystemStatus, PerformanceMetrics, SystemInfo, LogEntry } from '../../server/api/types';

// 系统状态
interface SystemState {
  status: SystemStatus | null;
  metrics: PerformanceMetrics | null;
  info: SystemInfo | null;
  logs: LogEntry[];
  loading: boolean;
  error: string | null;
  lastUpdated: string | null;
}

// 初始状态
const initialState: SystemState = {
  status: null,
  metrics: null,
  info: null,
  logs: [],
  loading: false,
  error: null,
  lastUpdated: null,
};

// 创建store
function createSystemStore() {
  const { subscribe, set, update } = writable<SystemState>(initialState);

  return {
    subscribe,
    
    // 加载系统状态
    loadStatus: async () => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await systemApi.status();
        update(state => ({ 
          ...state, 
          status: response.data,
          loading: false,
          lastUpdated: new Date().toISOString(),
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load system status',
        }));
      }
    },
    
    // 加载性能指标
    loadMetrics: async () => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await systemApi.metrics();
        update(state => ({ 
          ...state, 
          metrics: response.data,
          loading: false,
          lastUpdated: new Date().toISOString(),
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load performance metrics',
        }));
      }
    },
    
    // 加载系统信息
    loadInfo: async () => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await systemApi.info();
        update(state => ({ 
          ...state, 
          info: response.data,
          loading: false,
          lastUpdated: new Date().toISOString(),
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load system info',
        }));
      }
    },
    
    // 加载日志
    loadLogs: async (params?: {
      limit?: number;
      level?: string;
      source?: string;
    }) => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await systemApi.logs(params);
        update(state => ({ 
          ...state, 
          logs: response.data,
          loading: false,
          lastUpdated: new Date().toISOString(),
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to load logs',
        }));
      }
    },
    
    // 加载资源使用情况
    loadResources: async () => {
      try {
        const response = await systemApi.resources();
        return { success: true, resources: response.data };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to load resources',
        };
      }
    },
    
    // 清理缓存
    clearCache: async () => {
      try {
        await systemApi.clearCache();
        return { success: true, message: 'Cache cleared successfully' };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to clear cache',
        };
      }
    },
    
    // 重启服务
    restartService: async () => {
      try {
        await systemApi.restart();
        return { success: true, message: 'Service restart initiated' };
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : 'Failed to restart service',
        };
      }
    },
    
    // 刷新所有数据
    refreshAll: async () => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const [status, metrics, info, logs] = await Promise.all([
          systemApi.status(),
          systemApi.metrics(),
          systemApi.info(),
          systemApi.logs({ limit: 50 }),
        ]);
        
        update(state => ({
          ...state,
          status: status.data,
          metrics: metrics.data,
          info: info.data,
          logs: logs.data,
          loading: false,
          lastUpdated: new Date().toISOString(),
        }));
      } catch (error) {
        update(state => ({
          ...state,
          loading: false,
          error: error instanceof Error ? error.message : 'Failed to refresh system data',
        }));
      }
    },
    
    // 重置状态
    reset: () => {
      set(initialState);
    },
  };
}

export const systemStore = createSystemStore();

// 应用状态
interface AppState {
  connected: boolean;
  loading: boolean;
  error: string | null;
  lastConnectionCheck: string | null;
}

function createAppStore() {
  const { subscribe, set, update } = writable<AppState>({
    connected: false,
    loading: false,
    error: null,
    lastConnectionCheck: null,
  });

  return {
    subscribe,
    
    // 检查连接
    checkConnection: async () => {
      update(state => ({ ...state, loading: true, error: null }));
      
      try {
        const response = await systemApi.health();
        
        update(state => ({
          ...state,
          connected: response.status === 'healthy',
          loading: false,
          lastConnectionCheck: new Date().toISOString(),
        }));
        
        return { success: true, connected: response.status === 'healthy' };
      } catch (error) {
        update(state => ({
          ...state,
          connected: false,
          loading: false,
          error: error instanceof Error ? error.message : 'Connection failed',
          lastConnectionCheck: new Date().toISOString(),
        }));
        
        return { success: false, connected: false };
      }
    },
    
    // 设置连接状态
    setConnected: (connected: boolean) => {
      update(state => ({ ...state, connected }));
    },
    
    // 设置错误
    setError: (error: string | null) => {
      update(state => ({ ...state, error }));
    },
    
    // 重置状态
    reset: () => {
      set({
        connected: false,
        loading: false,
        error: null,
        lastConnectionCheck: null,
      });
    },
  };
}

export const appStore = createAppStore();

// 导出派生store
export const systemHealth = derived(systemStore, ($system) => {
  if (!$system.status) return null;
  
  return {
    overall: $system.status.status === 'healthy',
    vectorStore: $system.status.vector_store,
    llmService: $system.status.llm_service,
    timestamp: $system.status.timestamp,
  };
});

export const performanceSummary = derived(systemStore, ($system) => {
  if (!$system.metrics) return null;
  
  return {
    cpuUsage: $system.metrics.cpu_usage,
    memoryUsage: $system.metrics.memory_usage,
    diskUsage: $system.metrics.disk_usage,
    activeConnections: $system.metrics.active_connections,
    errorRate: $system.metrics.error_rate,
    responseTime: $system.metrics.response_time_avg,
  };
});

export const recentLogs = derived(systemStore, ($system) => {
  return $system.logs
    .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
    .slice(0, 20);
});

export const errorLogs = derived(systemStore, ($system) => {
  return $system.logs
    .filter(log => log.level === 'error')
    .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
    .slice(0, 10);
});

export const warningLogs = derived(systemStore, ($system) => {
  return $system.logs
    .filter(log => log.level === 'warn')
    .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
    .slice(0, 10);
});

// 主题状态
interface ThemeState {
  darkMode: boolean;
  sidebarCollapsed: boolean;
}

function createThemeStore() {
  const { subscribe, set, update } = writable<ThemeState>({
    darkMode: false,
    sidebarCollapsed: false,
  });

  return {
    subscribe,
    
    // 切换暗黑模式
    toggleDarkMode: () => {
      update(state => {
        const darkMode = !state.darkMode;
        
        // 保存到localStorage
        if (typeof window !== 'undefined') {
          localStorage.setItem('darkMode', darkMode.toString());
        }
        
        return { ...state, darkMode };
      });
    },
    
    // 切换侧边栏
    toggleSidebar: () => {
      update(state => {
        const sidebarCollapsed = !state.sidebarCollapsed;
        
        // 保存到localStorage
        if (typeof window !== 'undefined') {
          localStorage.setItem('sidebarCollapsed', sidebarCollapsed.toString());
        }
        
        return { ...state, sidebarCollapsed };
      });
    },
    
    // 从localStorage加载设置
    loadSettings: () => {
      if (typeof window === 'undefined') return;
      
      const darkMode = localStorage.getItem('darkMode') === 'true';
      const sidebarCollapsed = localStorage.getItem('sidebarCollapsed') === 'true';
      
      set({ darkMode, sidebarCollapsed });
    },
    
    // 重置设置
    reset: () => {
      set({ darkMode: false, sidebarCollapsed: false });
      
      if (typeof window !== 'undefined') {
        localStorage.removeItem('darkMode');
        localStorage.removeItem('sidebarCollapsed');
      }
    },
  };
}

export const themeStore = createThemeStore();