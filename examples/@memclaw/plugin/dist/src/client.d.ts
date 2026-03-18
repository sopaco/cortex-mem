/**
 * Cortex Mem Client
 *
 * HTTP client for cortex-mem-service REST API.
 */
/** Layer types */
export type Layer = 'L0' | 'L1' | 'L2';
export interface SearchOptions {
    query: string;
    thread?: string;
    limit?: number;
    min_score?: number;
    /** Which layers to return: ["L0"], ["L0","L1"], ["L0","L1","L2"] */
    return_layers?: Layer[];
}
export interface SearchResult {
    uri: string;
    score: number;
    snippet: string;
    overview?: string;
    content?: string;
    source: string;
    layers: Layer[];
}
export interface LsOptions {
    uri?: string;
    recursive?: boolean;
    include_abstracts?: boolean;
}
export interface LsEntry {
    uri: string;
    name: string;
    is_directory: boolean;
    size: number;
    modified: string;
    abstract_text?: string;
}
export interface LsResponse {
    uri: string;
    total: number;
    entries: LsEntry[];
}
export interface ExploreOptions {
    query: string;
    start_uri?: string;
    return_layers?: Layer[];
}
export interface ExploreResponse {
    query: string;
    exploration_path: ExplorationPathItem[];
    matches: SearchResult[];
    total_explored: number;
    total_matches: number;
}
export interface ExplorationPathItem {
    uri: string;
    relevance_score: number;
    abstract_text?: string;
}
export interface LayerResponse {
    uri: string;
    content: string;
    layer: Layer;
    token_count: number;
}
export interface SessionInfo {
    thread_id: string;
    status: string;
    message_count: number;
    created_at: string;
    updated_at: string;
}
export interface AddMessageOptions {
    content: string;
    role?: 'user' | 'assistant' | 'system';
    metadata?: Record<string, unknown>;
}
export declare class CortexMemClient {
    private baseUrl;
    constructor(baseUrl?: string);
    /**
     * Layered semantic search with L0/L1/L2 tiered retrieval
     */
    search(options: SearchOptions): Promise<SearchResult[]>;
    /**
     * Recall memories with more context (L0 + L2)
     */
    recall(query: string, thread?: string, limit?: number): Promise<SearchResult[]>;
    /**
     * List directory contents
     */
    ls(options?: LsOptions): Promise<LsResponse>;
    /**
     * Smart exploration combining search and browsing
     */
    explore(options: ExploreOptions): Promise<ExploreResponse>;
    /**
     * Get L0 abstract (~100 tokens) for quick relevance check
     */
    getAbstract(uri: string): Promise<LayerResponse>;
    /**
     * Get L1 overview (~2000 tokens) for core information
     */
    getOverview(uri: string): Promise<LayerResponse>;
    /**
     * Get L2 full content
     */
    getContent(uri: string): Promise<LayerResponse>;
    /**
     * List all sessions
     */
    listSessions(): Promise<SessionInfo[]>;
    /**
     * Add a message to a session
     */
    addMessage(threadId: string, message: AddMessageOptions): Promise<string>;
    /**
     * Close a session and trigger memory extraction
     */
    closeSession(threadId: string): Promise<{
        thread_id: string;
        status: string;
        message_count: number;
    }>;
    /**
     * Switch tenant context
     */
    switchTenant(tenantId: string): Promise<void>;
    private fetchJson;
}
//# sourceMappingURL=client.d.ts.map