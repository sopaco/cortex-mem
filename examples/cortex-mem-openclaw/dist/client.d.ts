/**
 * Cortex Memory API Client
 *
 * HTTP client for cortex-mem-service REST API
 */
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
export type ContextLayer = "L0" | "L1" | "L2";
export interface LayeredRecallResult {
    uri: string;
    score: number;
    abstract?: string;
    overview?: string;
    content?: string;
}
/**
 * Cortex Memory API Client
 */
export declare class CortexMemClient {
    private baseUrl;
    constructor(baseUrl?: string);
    /**
     * Layered semantic search (L0 -> L1 -> L2 tiered retrieval)
     */
    search(request: SearchRequest): Promise<SearchResult[]>;
    /**
     * Quick search returning only L0 abstracts
     */
    find(query: string, scope?: string, limit?: number): Promise<SearchResult[]>;
    /**
     * Layered recall with specified detail level
     *
     * @param query - Search query
     * @param layers - Which layers to return (L0, L1, L2)
     * @param scope - Optional session/thread scope
     * @param limit - Maximum results
     */
    recall(query: string, layers?: ContextLayer[], scope?: string, limit?: number): Promise<LayeredRecallResult[]>;
    /**
     * List all sessions
     */
    listSessions(): Promise<SessionResponse[]>;
    /**
     * Create a new session
     */
    createSession(request?: CreateSessionRequest): Promise<SessionResponse>;
    /**
     * Add a message to a session
     */
    addMessage(threadId: string, message: AddMessageRequest): Promise<string>;
    /**
     * Close a session
     */
    closeSession(threadId: string): Promise<SessionResponse>;
    /**
     * Health check
     */
    healthCheck(): Promise<boolean>;
    private get;
    private post;
}
//# sourceMappingURL=client.d.ts.map