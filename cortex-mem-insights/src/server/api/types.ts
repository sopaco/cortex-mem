// API 响应类型
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
  timestamp: string;
}

// 记忆相关类型
export interface MemoryMetadataResponse {
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  role?: string;
  memory_type: string;
  hash: string;
  importance_score?: number;
  custom?: Record<string, any>;
}

export interface MemoryResponse {
  id: string;
  content: string;
  metadata: MemoryMetadataResponse;
  created_at: string;
  updated_at: string;
}

export interface ScoredMemoryResponse {
  memory: MemoryResponse;
  score: number;
}

// 列表响应
export interface ListResponse {
  total: number;
  memories: MemoryResponse[];
}

// 搜索响应
export interface SearchResponse {
  total: number;
  results: ScoredMemoryResponse[];
}

// 健康检查响应
export interface HealthResponse {
  status: string;
  vector_store: boolean;
  llm_service: boolean;
  timestamp: string;
}

// 优化相关类型
export interface OptimizationRequest {
  memory_type?: string;
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  similarity_threshold?: number;
  dry_run?: boolean;
  verbose?: boolean;
}

export interface OptimizationResult {
  job_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  total_memories: number;
  processed_memories: number;
  deduplicated: number;
  merged: number;
  enhanced: number;
  errors: number;
  start_time: string;
  end_time?: string;
  duration?: number;
  message?: string;
}

export interface OptimizationHistory {
  job_id: string;
  status: string;
  total_memories: number;
  processed_memories: number;
  start_time: string;
  end_time?: string;
  duration?: number;
}

// 系统相关类型
export interface SystemStatus {
  status: 'healthy' | 'unhealthy';
  vector_store: boolean;
  llm_service: boolean;
  timestamp: string;
}

export interface PerformanceMetrics {
  cpu_usage: number;
  memory_usage: number;
  disk_usage: number;
  active_connections: number;
  request_count: number;
  error_rate: number;
  response_time_avg: number;
  timestamp: string;
}

export interface SystemInfo {
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

export interface LogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  source: string;
  metadata?: Record<string, any>;
}

// 统计类型
export interface Statistics {
  total_memories: number;
  by_type: Record<string, number>;
  by_user: Record<string, number>;
  by_agent: Record<string, number>;
  recent_activity: Array<{ date: string; count: number }>;
}

// 分页参数
export interface PaginationParams {
  page?: number;
  limit?: number;
  sort_by?: string;
  sort_order?: 'asc' | 'desc';
}

// 过滤参数
export interface FilterParams {
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  memory_type?: string;
  start_date?: string;
  end_date?: string;
  min_score?: number;
  max_score?: number;
}

// 搜索参数
export interface SearchParams extends FilterParams {
  query: string;
  limit?: number;
  similarity_threshold?: number;
}

// 创建记忆请求
export interface CreateMemoryRequest {
  content: string;
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  role?: string;
  memory_type?: string;
  custom?: Record<string, any>;
}

// 更新记忆请求
export interface UpdateMemoryRequest {
  content: string;
}

// 批量操作请求
export interface BatchOperationRequest {
  ids: string[];
  operation: 'delete' | 'export' | 'tag';
  tags?: string[];
}

// 批量操作响应
export interface BatchOperationResponse {
  success: boolean;
  message: string;
  total: number;
  succeeded: number;
  failed: number;
  failed_ids?: string[];
}

// 导出格式
export interface ExportFormat {
  format: 'json' | 'csv' | 'txt';
  include_metadata?: boolean;
  include_scores?: boolean;
  compress?: boolean;
}

// 导出响应
export interface ExportResponse {
  success: boolean;
  download_url?: string;
  file_size?: number;
  format: string;
  item_count: number;
  message?: string;
}