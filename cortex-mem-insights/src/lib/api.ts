// API Client for Cortex-Mem Insights

import type { FileEntryResponse, SearchResult } from './types';

const API_BASE = '/api/v2';

// Response wrapper from backend
interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
  timestamp: string;
}

class ApiClient {
  private baseUrl: string;
  private currentTenant: string | null = null;

  constructor(baseUrl: string = API_BASE) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    const json: ApiResponse<T> = await response.json();
    if (!json.success || json.data === null) {
      throw new Error(json.error || 'Request failed');
    }
    return json.data;
  }

  // Health check
  async getHealth(): Promise<{ status: string; service: string; version: string; llm_available: boolean }> {
    const response = await fetch('/health');
    return response.json();
  }

  // Tenant endpoints
  async listTenants(): Promise<string[]> {
    return this.request<string[]>('/tenants/tenants');
  }

  async switchTenant(tenantId: string): Promise<string> {
    const result = await this.request<string>('/tenants/tenants/switch', {
      method: 'POST',
      body: JSON.stringify({ tenant_id: tenantId })
    });
    this.currentTenant = tenantId;
    return result;
  }

  getCurrentTenant(): string | null {
    return this.currentTenant;
  }

  // Filesystem endpoints
  async listDirectory(path: string): Promise<FileEntryResponse[]> {
    return this.request<FileEntryResponse[]>(`/filesystem/list?uri=${encodeURIComponent(path)}`);
  }

  async readFile(path: string): Promise<string> {
    // Handle path - remove cortex:// prefix if present
    const cleanPath = path.replace(/^cortex:\/\//, '');
    return this.request<string>(`/filesystem/read/${encodeURIComponent(cleanPath)}`);
  }

  async writeFile(path: string, content: string): Promise<string> {
    return this.request<string>('/filesystem/write', {
      method: 'POST',
      body: JSON.stringify({ path, content })
    });
  }

  async getDirectoryStats(uri: string): Promise<{ file_count: number; total_size: number }> {
    return this.request<{ file_count: number; total_size: number }>(
      `/filesystem/stats?uri=${encodeURIComponent(uri)}`
    );
  }

  // Session endpoints
  async getSessions(): Promise<{ thread_id: string; status: string; message_count: number }[]> {
    return this.request('/sessions');
  }

  // Search endpoints (POST method)
  async search(keyword: string, scope: string = 'all', limit: number = 10): Promise<SearchResult[]> {
    const searchRequest = {
      query: keyword,
      limit: limit,
      min_score: 0.5,
      thread: scope === 'all' ? null : scope === 'user' ? 'user' : 'system'
    };
    
    return this.request<SearchResult[]>('/search', {
      method: 'POST',
      body: JSON.stringify(searchRequest)
    });
  }
}

export const apiClient = new ApiClient();
export default apiClient;
