/**
 * Cortex Memory API Client
 *
 * HTTP client for cortex-mem-service REST API
 */

// Response types
interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

// Search types
export interface SearchRequest {
  query: string;
  thread?: string;
  limit?: number;
  min_score?: number;
}

export interface SearchResult {
  uri: string;
  score: number;
  snippet: string;
  content?: string;
  source: string;
}

// Session types
export interface SessionResponse {
  thread_id: string;
  status: string;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export interface CreateSessionRequest {
  thread_id?: string;
  title?: string;
  user_id?: string;
  agent_id?: string;
}

export interface AddMessageRequest {
  role: "user" | "assistant" | "system";
  content: string;
}

/**
 * Cortex Memory API Client
 */
export class CortexMemClient {
  private baseUrl: string;

  constructor(baseUrl: string = "http://localhost:8085") {
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }

  /**
   * Layered semantic search (L0 -> L1 -> L2 tiered retrieval)
   */
  async search(request: SearchRequest): Promise<SearchResult[]> {
    const response = await this.post<SearchResult[]>("/api/v2/search", request);
    return response;
  }

  /**
   * Quick search returning only L0 abstracts
   */
  async find(
    query: string,
    scope?: string,
    limit: number = 5,
  ): Promise<SearchResult[]> {
    return this.search({
      query,
      thread: scope,
      limit,
      min_score: 0.5,
    });
  }

  /**
   * Layered recall - uses L0/L1/L2 tiered search internally
   *
   * The search engine performs tiered retrieval (L0→L1→L2) internally,
   * but returns unified results with snippet and content.
   *
   * @param query - Search query
   * @param scope - Optional session/thread scope
   * @param limit - Maximum results
   */
  async recall(
    query: string,
    scope?: string,
    limit: number = 10,
  ): Promise<SearchResult[]> {
    return this.search({
      query,
      thread: scope,
      limit,
      min_score: 0.5,
    });
  }

  /**
   * List all sessions
   */
  async listSessions(): Promise<SessionResponse[]> {
    const response = await this.get<SessionResponse[]>("/api/v2/sessions");
    return response;
  }

  /**
   * Create a new session
   */
  async createSession(
    request: CreateSessionRequest = {},
  ): Promise<SessionResponse> {
    const response = await this.post<SessionResponse>(
      "/api/v2/sessions",
      request,
    );
    return response;
  }

  /**
   * Add a message to a session
   */
  async addMessage(
    threadId: string,
    message: AddMessageRequest,
  ): Promise<string> {
    const response = await this.post<string>(
      `/api/v2/sessions/${threadId}/messages`,
      message,
    );
    return response;
  }

  /**
   * Close a session
   */
  async closeSession(threadId: string): Promise<SessionResponse> {
    const response = await this.post<SessionResponse>(
      `/api/v2/sessions/${threadId}/close`,
      {},
    );
    return response;
  }

  /**
   * Switch tenant
   */
  async switchTenant(tenantId: string): Promise<void> {
    await this.post("/api/v2/tenants/switch", { tenant_id: tenantId });
  }

  /**
   * Health check
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/health`);
      return response.ok;
    } catch {
      return false;
    }
  }

  // Private helpers
  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`);
    if (!response.ok) {
      throw new Error(`API error: ${response.status} ${response.statusText}`);
    }
    const data = (await response.json()) as ApiResponse<T>;
    if (!data.success) {
      throw new Error(data.error || "API request failed");
    }
    return data.data!;
  }

  private async post<T>(path: string, body: object): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(
        `API error: ${response.status} ${response.statusText} - ${errorText}`,
      );
    }
    const data = (await response.json()) as ApiResponse<T>;
    if (!data.success) {
      throw new Error(data.error || "API request failed");
    }
    return data.data!;
  }
}
