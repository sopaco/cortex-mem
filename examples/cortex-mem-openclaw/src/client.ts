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
  role: 'user' | 'assistant' | 'system';
  content: string;
}

// Layer types
export type ContextLayer = 'L0' | 'L1' | 'L2';

export interface LayeredRecallResult {
  uri: string;
  score: number;
  abstract?: string;    // L0: ~100 tokens
  overview?: string;    // L1: ~2000 tokens
  content?: string;     // L2: full content
}

/**
 * Cortex Memory API Client
 */
export class CortexMemClient {
  private baseUrl: string;

  constructor(baseUrl: string = 'http://127.0.0.1:8085') {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  /**
   * Layered semantic search (L0 -> L1 -> L2 tiered retrieval)
   */
  async search(request: SearchRequest): Promise<SearchResult[]> {
    const response = await this.post<SearchResult[]>('/api/v2/search', request);
    return response;
  }

  /**
   * Quick search returning only L0 abstracts
   */
  async find(query: string, scope?: string, limit: number = 5): Promise<SearchResult[]> {
    return this.search({
      query,
      thread: scope,
      limit,
      min_score: 0.5,
    });
  }

  /**
   * Layered recall with specified detail level
   *
   * @param query - Search query
   * @param layers - Which layers to return (L0, L1, L2)
   * @param scope - Optional session/thread scope
   * @param limit - Maximum results
   */
  async recall(
    query: string,
    layers: ContextLayer[] = ['L0'],
    scope?: string,
    limit: number = 10
  ): Promise<LayeredRecallResult[]> {
    // First do search to get URIs
    const searchResults = await this.search({
      query,
      thread: scope,
      limit,
    });

    // For now, return search results with snippets
    // In a full implementation, we would make additional calls
    // to get L1 overview and L2 content based on requested layers
    return searchResults.map(result => ({
      uri: result.uri,
      score: result.score,
      abstract: result.snippet,  // L0 from snippet
      overview: undefined,        // Would need additional API call
      content: result.content,    // L2 if available
    }));
  }

  /**
   * List all sessions
   */
  async listSessions(): Promise<SessionResponse[]> {
    const response = await this.get<SessionResponse[]>('/api/v2/sessions');
    return response;
  }

  /**
   * Create a new session
   */
  async createSession(request: CreateSessionRequest = {}): Promise<SessionResponse> {
    const response = await this.post<SessionResponse>('/api/v2/sessions', request);
    return response;
  }

  /**
   * Add a message to a session
   */
  async addMessage(threadId: string, message: AddMessageRequest): Promise<string> {
    const response = await this.post<string>(
      `/api/v2/sessions/${threadId}/messages`,
      message
    );
    return response;
  }

  /**
   * Close a session
   */
  async closeSession(threadId: string): Promise<SessionResponse> {
    const response = await this.post<SessionResponse>(
      `/api/v2/sessions/${threadId}/close`,
      {}
    );
    return response;
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
    const data = await response.json() as ApiResponse<T>;
    if (!data.success) {
      throw new Error(data.error || 'API request failed');
    }
    return data.data!;
  }

  private async post<T>(path: string, body: object): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(`API error: ${response.status} ${response.statusText}`);
    }
    const data = await response.json() as ApiResponse<T>;
    if (!data.success) {
      throw new Error(data.error || 'API request failed');
    }
    return data.data!;
  }
}
