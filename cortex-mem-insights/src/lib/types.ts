// Types for Cortex-Mem Insights

export interface HealthStatus {
  status: string;
  service: string;
  version: string;
  llm_available: boolean;
}

export interface FileEntryResponse {
  uri: string;
  name: string;
  is_directory: boolean;
  size: number;
  modified: string;
}

export interface SessionInfo {
  thread_id: string;
  status: string;
  message_count: number;
  created_at?: string;
  updated_at?: string;
}

export interface SearchResult {
  uri: string;
  score: number;
  snippet: string;
  content?: string;
  source: string;
}