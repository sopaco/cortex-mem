"use strict";
/**
 * Cortex Memory API Client
 *
 * HTTP client for cortex-mem-service REST API
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.CortexMemClient = void 0;
/**
 * Cortex Memory API Client
 */
class CortexMemClient {
    baseUrl;
    constructor(baseUrl = "http://localhost:8085") {
        this.baseUrl = baseUrl.replace(/\/$/, "");
    }
    /**
     * Layered semantic search (L0 -> L1 -> L2 tiered retrieval)
     */
    async search(request) {
        const response = await this.post("/api/v2/search", request);
        return response;
    }
    /**
     * Quick search returning only L0 abstracts
     */
    async find(query, scope, limit = 5) {
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
    async recall(query, layers = ["L0"], scope, limit = 10) {
        // First do search to get URIs
        const searchResults = await this.search({
            query,
            thread: scope,
            limit,
        });
        // For now, return search results with snippets
        // In a full implementation, we would make additional calls
        // to get L1 overview and L2 content based on requested layers
        return searchResults.map((result) => ({
            uri: result.uri,
            score: result.score,
            abstract: result.snippet, // L0 from snippet
            overview: undefined, // Would need additional API call
            content: result.content, // L2 if available
        }));
    }
    /**
     * List all sessions
     */
    async listSessions() {
        const response = await this.get("/api/v2/sessions");
        return response;
    }
    /**
     * Create a new session
     */
    async createSession(request = {}) {
        const response = await this.post("/api/v2/sessions", request);
        return response;
    }
    /**
     * Add a message to a session
     */
    async addMessage(threadId, message) {
        const response = await this.post(`/api/v2/sessions/${threadId}/messages`, message);
        return response;
    }
    /**
     * Close a session
     */
    async closeSession(threadId) {
        const response = await this.post(`/api/v2/sessions/${threadId}/close`, {});
        return response;
    }
    /**
     * Health check
     */
    async healthCheck() {
        try {
            const response = await fetch(`${this.baseUrl}/health`);
            return response.ok;
        }
        catch {
            return false;
        }
    }
    // Private helpers
    async get(path) {
        const response = await fetch(`${this.baseUrl}${path}`);
        if (!response.ok) {
            throw new Error(`API error: ${response.status} ${response.statusText}`);
        }
        const data = (await response.json());
        if (!data.success) {
            throw new Error(data.error || "API request failed");
        }
        return data.data;
    }
    async post(path, body) {
        const response = await fetch(`${this.baseUrl}${path}`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(body),
        });
        if (!response.ok) {
            throw new Error(`API error: ${response.status} ${response.statusText}`);
        }
        const data = (await response.json());
        if (!data.success) {
            throw new Error(data.error || "API request failed");
        }
        return data.data;
    }
}
exports.CortexMemClient = CortexMemClient;
//# sourceMappingURL=client.js.map