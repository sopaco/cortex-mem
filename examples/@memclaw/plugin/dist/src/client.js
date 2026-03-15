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
     * Layered recall - uses L0/L1/L2 tiered search internally
     *
     * The search engine performs tiered retrieval (L0→L1→L2) internally,
     * but returns unified results with snippet and content.
     *
     * @param query - Search query
     * @param scope - Optional session/thread scope
     * @param limit - Maximum results
     */
    async recall(query, scope, limit = 10) {
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
     * Switch tenant
     */
    async switchTenant(tenantId) {
        await this.post("/api/v2/tenants/switch", { tenant_id: tenantId });
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
            const errorText = await response.text();
            throw new Error(`API error: ${response.status} ${response.statusText} - ${errorText}`);
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