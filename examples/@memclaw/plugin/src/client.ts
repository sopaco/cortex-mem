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

export class CortexMemClient {
	private baseUrl: string;

	constructor(baseUrl: string = 'http://localhost:8085') {
		this.baseUrl = baseUrl;
	}

	// ==================== Search ====================

	/**
	 * Layered semantic search with L0/L1/L2 tiered retrieval
	 */
	async search(options: SearchOptions): Promise<SearchResult[]> {
		const response = await this.fetchJson<{
			success: boolean;
			data?: SearchResult[];
			error?: string;
		}>('/api/v2/search', {
			method: 'POST',
			body: JSON.stringify({
				query: options.query,
				thread: options.thread,
				limit: options.limit ?? 10,
				min_score: options.min_score ?? 0.6,
				return_layers: options.return_layers ?? ['L0']
			})
		});

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Search failed');
		}

		return response.data;
	}

	/**
	 * Recall memories with more context (L0 + L2)
	 */
	async recall(
		query: string,
		thread?: string,
		limit: number = 10
	): Promise<SearchResult[]> {
		return this.search({
			query,
			thread,
			limit,
			return_layers: ['L0', 'L2']
		});
	}

	// ==================== Filesystem ====================

	/**
	 * List directory contents
	 */
	async ls(options: LsOptions = {}): Promise<LsResponse> {
		const params = new URLSearchParams();
		params.set('uri', options.uri ?? 'cortex://session');
		if (options.recursive) params.set('recursive', 'true');
		if (options.include_abstracts) params.set('include_abstracts', 'true');

		const response = await this.fetchJson<{
			success: boolean;
			data?: LsResponse;
			error?: string;
		}>(`/api/v2/filesystem/list?${params.toString()}`);

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'List directory failed');
		}

		return response.data;
	}

	/**
	 * Smart exploration combining search and browsing
	 */
	async explore(options: ExploreOptions): Promise<ExploreResponse> {
		const response = await this.fetchJson<{
			success: boolean;
			data?: ExploreResponse;
			error?: string;
		}>('/api/v2/filesystem/explore', {
			method: 'POST',
			body: JSON.stringify({
				query: options.query,
				start_uri: options.start_uri ?? 'cortex://session',
				return_layers: options.return_layers ?? ['L0']
			})
		});

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Explore failed');
		}

		return response.data;
	}

	// ==================== Tiered Access ====================

	/**
	 * Get L0 abstract (~100 tokens) for quick relevance check
	 */
	async getAbstract(uri: string): Promise<LayerResponse> {
		const params = new URLSearchParams();
		params.set('uri', uri);

		const response = await this.fetchJson<{
			success: boolean;
			data?: LayerResponse;
			error?: string;
		}>(`/api/v2/filesystem/abstract?${params.toString()}`);

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Get abstract failed');
		}

		return response.data;
	}

	/**
	 * Get L1 overview (~2000 tokens) for core information
	 */
	async getOverview(uri: string): Promise<LayerResponse> {
		const params = new URLSearchParams();
		params.set('uri', uri);

		const response = await this.fetchJson<{
			success: boolean;
			data?: LayerResponse;
			error?: string;
		}>(`/api/v2/filesystem/overview?${params.toString()}`);

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Get overview failed');
		}

		return response.data;
	}

	/**
	 * Get L2 full content
	 */
	async getContent(uri: string): Promise<LayerResponse> {
		const params = new URLSearchParams();
		params.set('uri', uri);

		const response = await this.fetchJson<{
			success: boolean;
			data?: LayerResponse;
			error?: string;
		}>(`/api/v2/filesystem/content?${params.toString()}`);

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Get content failed');
		}

		return response.data;
	}

	// ==================== Session Management ====================

	/**
	 * List all sessions
	 */
	async listSessions(): Promise<SessionInfo[]> {
		const response = await this.fetchJson<{
			success: boolean;
			data?: SessionInfo[];
			error?: string;
		}>('/api/v2/sessions');

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'List sessions failed');
		}

		return response.data;
	}

	/**
	 * Add a message to a session
	 */
	async addMessage(threadId: string, message: AddMessageOptions): Promise<string> {
		const response = await this.fetchJson<{
			success: boolean;
			data?: string;
			error?: string;
		}>('/api/v2/sessions/message', {
			method: 'POST',
			body: JSON.stringify({
				thread_id: threadId,
				role: message.role ?? 'user',
				content: message.content,
				metadata: message.metadata
			})
		});

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Add message failed');
		}

		return response.data;
	}

	/**
	 * Close a session and trigger memory extraction
	 */
	async closeSession(threadId: string): Promise<{
		thread_id: string;
		status: string;
		message_count: number;
	}> {
		const response = await this.fetchJson<{
			success: boolean;
			data?: {
				thread_id: string;
				status: string;
				message_count: number;
			};
			error?: string;
		}>('/api/v2/sessions/close', {
			method: 'POST',
			body: JSON.stringify({ thread_id: threadId })
		});

		if (!response.success || !response.data) {
			throw new Error(response.error ?? 'Close session failed');
		}

		return response.data;
	}

	// ==================== Tenant ====================

	/**
	 * Switch tenant context
	 */
	async switchTenant(tenantId: string): Promise<void> {
		const response = await this.fetchJson<{
			success: boolean;
			error?: string;
		}>('/api/v2/tenants/switch', {
			method: 'POST',
			body: JSON.stringify({ tenant_id: tenantId })
		});

		if (!response.success) {
			throw new Error(response.error ?? 'Switch tenant failed');
		}
	}

	// ==================== Internal ====================

	private async fetchJson<T>(
		path: string,
		options: RequestInit = {}
	): Promise<T> {
		const url = `${this.baseUrl}${path}`;
		const headers = {
			'Content-Type': 'application/json',
			...(options.headers || {})
		};

		const response = await fetch(url, {
			...options,
			headers
		});

		if (!response.ok) {
			throw new Error(`HTTP ${response.status}: ${response.statusText}`);
		}

		return response.json() as Promise<T>;
	}
}
