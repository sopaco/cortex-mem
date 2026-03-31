/**
 * MemClaw Plugin Implementation
 *
 * Provides layered semantic memory for OpenClaw with:
 * - Automatic service startup
 * - Memory tools (search, recall, add, close)
 * - Tiered access (L0/L1/L2)
 * - Filesystem browsing
 * - Smart exploration
 * - Migration from OpenClaw native memory
 */

import { CortexMemClient } from './src/client.js';
import {
	ensureConfigExists,
	openConfigFile,
	parseConfig,
	validateConfig,
	getDataDir,
	getConfigPath,
	updateConfigFromPlugin,
	mergeConfigWithPlugin,
	type PluginProvidedConfig
} from './src/config.js';
import {
	ensureAllServices,
	checkServiceStatus,
	isBinaryAvailable,
	executeCliCommand
} from './src/binaries.js';
import { migrateFromOpenClaw, canMigrate } from './src/migrate.js';

// Plugin configuration
interface PluginConfig {
	serviceUrl?: string;
	defaultSessionId?: string;
	searchLimit?: number;
	minScore?: number;
	tenantId?: string;
	autoStartServices?: boolean;
	qdrantPort?: number;
	servicePort?: number;
	// LLM/Embedding configuration (synced to config.toml)
	llmApiBaseUrl?: string;
	llmApiKey?: string;
	llmModel?: string;
	embeddingApiBaseUrl?: string;
	embeddingApiKey?: string;
	embeddingModel?: string;
}

// OpenClaw Plugin API types
interface PluginLogger {
	debug?: (msg: string, ...args: unknown[]) => void;
	info: (msg: string, ...args: unknown[]) => void;
	warn: (msg: string, ...args: unknown[]) => void;
	error: (msg: string, ...args: unknown[]) => void;
}

interface PluginAPI {
	pluginConfig?: Record<string, unknown>;
	registerTool(tool: ToolDefinition, opts?: { optional?: boolean }): void;
	registerService(service: {
		id: string;
		start: () => Promise<void>;
		stop: () => Promise<void>;
	}): void;
	logger: PluginLogger;
}

interface ToolDefinition {
	name: string;
	description: string;
	parameters: object;
	execute: (_id: string, params: Record<string, unknown>) => Promise<unknown>;
	optional?: boolean;
}

// Tool schemas
const toolSchemas = {
	cortex_search: {
		name: 'cortex_search',
		description: `Layered semantic search across memory using L0/L1/L2 tiered retrieval.

**Key Features:**
- Tiered retrieval: L0 (abstract) -> L1 (overview) -> L2 (full content)
- Token-efficient: Control exactly which layers to return

**Parameters:**
- return_layers: ["L0"] (default, ~100 tokens), ["L0","L1"] (~2100 tokens), ["L0","L1","L2"] (full)

**When to use:**
- Finding past conversations or decisions
- Searching across all sessions
- Discovering related memories by semantic similarity`,
		inputSchema: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description: 'The search query - can be natural language or keywords'
				},
				scope: {
					type: 'string',
					description: 'Optional session/thread ID to limit search scope'
				},
				limit: {
					type: 'integer',
					description: 'Maximum number of results to return (default: 10)',
					default: 10
				},
				min_score: {
					type: 'number',
					description: 'Minimum relevance score threshold (0-1, default: 0.6)',
					default: 0.6
				},
				return_layers: {
					type: 'array',
					items: {
						type: 'string',
						enum: ['L0', 'L1', 'L2']
					},
					description: 'Which layers to return. Default: ["L0"]. Use ["L0","L1"] for more context, ["L0","L1","L2"] for full content.',
					default: ['L0']
				}
			},
			required: ['query']
		}
	},

	cortex_recall: {
		name: 'cortex_recall',
		description: `Recall memories with full context (L0 snippet + L2 content).

This is a convenience wrapper that returns both abstract and full content.
Use cortex_search with return_layers=["L0","L2"] for more control.`,
		inputSchema: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description: 'The search query'
				},
				scope: {
					type: 'string',
					description: 'Optional session/thread ID to limit search scope'
				},
				limit: {
					type: 'integer',
					description: 'Maximum number of results (default: 10)',
					default: 10
				}
			},
			required: ['query']
		}
	},

	cortex_add_memory: {
		name: 'cortex_add_memory',
		description: `Add a message to memory for a specific session.

This stores the message and automatically triggers:
- Vector embedding for semantic search
- L0/L1 layer generation (async)

**Metadata support:**
You can attach metadata like tags, importance, or custom fields.

Use this to persist important information that should be searchable later.`,
		inputSchema: {
			type: 'object',
			properties: {
				content: {
					type: 'string',
					description: 'The content to store in memory'
				},
				role: {
					type: 'string',
					enum: ['user', 'assistant', 'system'],
					description: 'Role of the message sender (default: user)',
					default: 'user'
				},
				session_id: {
					type: 'string',
					description: 'Session/thread ID (uses default if not specified)'
				},
				metadata: {
					type: 'object',
					description: 'Optional metadata (tags, importance, custom fields)',
					additionalProperties: true
				}
			},
			required: ['content']
		}
	},

	cortex_commit_session: {
		name: 'cortex_commit_session',
		description: `Commit accumulated conversation content and trigger memory extraction.

**IMPORTANT - Call this tool proactively and periodically, NOT just at conversation end.**

This commits the session and triggers the complete memory processing pipeline:
1. Extracts structured memories (user preferences, entities, decisions)
2. Generates complete L0/L1 layer summaries
3. Indexes all extracted memories into the vector database

**When to call this tool:**
- After completing a significant task or topic discussion
- After the user has shared important preferences or decisions
- When the conversation topic shifts to something new
- After accumulating substantial conversation content (every 10-20 exchanges)
- Before ending a conversation session

**Do NOT wait until the very end of conversation** - the user may forget or the session may end abruptly.

**Guidelines:**
- Call this tool at natural checkpoints in the conversation
- Avoid calling too frequently (not after every message)
- A good rhythm: once per significant topic completion
- This is a long-running operation (30-60s) but runs asynchronously`,
		inputSchema: {
			type: 'object',
			properties: {
				session_id: {
					type: 'string',
					description: 'Session/thread ID to commit (uses default if not specified)'
				}
			}
		}
	},

	// ==================== Filesystem Tools ====================

	cortex_ls: {
		name: 'cortex_ls',
		description: `List directory contents to browse the memory space like a virtual filesystem.

This allows you to explore the hierarchical structure of memories:
- cortex://session - List all sessions
- cortex://session/{session_id} - Browse a specific session's contents
- cortex://session/{session_id}/timeline - View timeline messages
- cortex://session/{session_id}/memories - View extracted memories

**Parameters:**
- recursive: List all subdirectories recursively
- include_abstracts: Show L0 abstracts for each file (for quick preview)

Use this when:
- Semantic search doesn't find what you need
- You want to understand the overall memory layout
- You need to manually navigate to find specific information`,
		inputSchema: {
			type: 'object',
			properties: {
				uri: {
					type: 'string',
					description: 'Directory URI to list (default: cortex://session)',
					default: 'cortex://session'
				},
				recursive: {
					type: 'boolean',
					description: 'Whether to recursively list subdirectories',
					default: false
				},
				include_abstracts: {
					type: 'boolean',
					description: 'Whether to include L0 abstracts for each file',
					default: false
				}
			}
		}
	},

	// ==================== Tiered Access Tools ====================

	cortex_get_abstract: {
		name: 'cortex_get_abstract',
		description: `Get L0 abstract layer (~100 tokens) for quick relevance checking.

Abstracts are short summaries ideal for quickly determining if content is relevant
before committing to reading more. Use this to minimize token consumption.

Use when:
- You found a URI from cortex_ls and want to quickly check relevance
- You need to filter many candidates before deep reading
- You want the most token-efficient preview`,
		inputSchema: {
			type: 'object',
			properties: {
				uri: {
					type: 'string',
					description: 'Content URI (file or directory)'
				}
			},
			required: ['uri']
		}
	},

	cortex_get_overview: {
		name: 'cortex_get_overview',
		description: `Get L1 overview layer (~2000 tokens) with core information and context.

Overviews contain key points and contextual information. Use this when:
- The abstract was relevant but you need more details
- You want to understand the gist without full content
- You need moderate detail for decision making`,
		inputSchema: {
			type: 'object',
			properties: {
				uri: {
					type: 'string',
					description: 'Content URI (file or directory)'
				}
			},
			required: ['uri']
		}
	},

	cortex_get_content: {
		name: 'cortex_get_content',
		description: `Get L2 full content layer - the complete original content.

Use this ONLY when you need the complete, unprocessed content.
This returns the full content which may be large.

Use when:
- You need exact details or quotes
- Abstract and overview don't provide enough information
- You need to see the original, unsummarized content`,
		inputSchema: {
			type: 'object',
			properties: {
				uri: {
					type: 'string',
					description: 'Content URI (file only)'
				}
			},
			required: ['uri']
		}
	},

	// ==================== Exploration Tool ====================

	cortex_explore: {
		name: 'cortex_explore',
		description: `Smart exploration of memory space, combining search and browsing.

This tool performs a guided exploration:
1. Searches within a specified scope (start_uri)
2. Returns an exploration path showing relevance scores
3. Returns matching results with requested layers

**When to use:**
- When you need to "wander" through memories with a purpose
- When you want to discover related content in a specific area
- When combining keyword hints with semantic discovery

**Parameters:**
- start_uri: Where to begin exploration (default: cortex://session)
- return_layers: Which layers to include in matches`,
		inputSchema: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description: 'Exploration query - what to look for'
				},
				start_uri: {
					type: 'string',
					description: 'Starting URI for exploration',
					default: 'cortex://session'
				},
				return_layers: {
					type: 'array',
					items: {
						type: 'string',
						enum: ['L0', 'L1', 'L2']
					},
					description: 'Which layers to return in matches',
					default: ['L0']
				}
			},
			required: ['query']
		}
	},

	// ==================== Migration & Maintenance ====================

	cortex_migrate: {
		name: 'cortex_migrate',
		description: `Migrate memories from OpenClaw's native memory system to MemClaw.

This will:
1. Find your OpenClaw memory files (memory/*.md and MEMORY.md)
2. Convert them to MemClaw's L2 format
3. Generate L0/L1 layers and vector index

Use this once during initial setup to preserve your existing memories.`,
		inputSchema: {
			type: 'object',
			properties: {}
		}
	},

	cortex_maintenance: {
		name: 'cortex_maintenance',
		description: `Perform periodic maintenance on MemClaw data.

This executes:
1. vector prune - Remove vectors whose source files no longer exist
2. vector reindex - Rebuild vector index and remove stale entries
3. layers ensure-all - Generate missing L0/L1 layer files

**This tool is typically called automatically by a scheduled Cron job.**
You can also call it manually when:
- Search results seem incomplete or stale
- After recovering from a crash or data corruption
- When disk space cleanup is needed

**Parameters:**
- dryRun: Preview changes without executing (default: false)
- commands: Which commands to run (default: all)`,
		inputSchema: {
			type: 'object',
			properties: {
				dryRun: {
					type: 'boolean',
					description: 'Preview changes without executing',
					default: false
				},
				commands: {
					type: 'array',
					items: {
						type: 'string',
						enum: ['prune', 'reindex', 'ensure-all']
					},
					description: 'Which maintenance commands to run',
					default: ['prune', 'reindex', 'ensure-all']
				}
			}
		}
	}
};

// Maintenance interval: 3 hours
const MAINTENANCE_INTERVAL_MS = 3 * 60 * 60 * 1000;

export function createPlugin(api: PluginAPI) {
	const config = (api.pluginConfig ?? {}) as PluginConfig;
	const serviceUrl = config.serviceUrl ?? 'http://localhost:8085';
	const defaultSessionId = config.defaultSessionId ?? 'default';
	const searchLimit = config.searchLimit ?? 10;
	const minScore = config.minScore ?? 0.6;
	const tenantId = config.tenantId ?? 'tenant_claw';
	const autoStartServices = config.autoStartServices ?? true;

	const client = new CortexMemClient(serviceUrl);
	let servicesStarted = false;
	let maintenanceTimer: ReturnType<typeof setInterval> | null = null;

	const log = (msg: string) => api.logger.info(`[memclaw] ${msg}`);

	log('Initializing MemClaw plugin...');

	// Ensure config file exists
	const { created, path: configPath } = ensureConfigExists();

	if (created) {
		log(`Created configuration file: ${configPath}`);
		log('Opening configuration file for editing...');

		openConfigFile(configPath).catch((err) => {
			api.logger.warn(`[memclaw] Could not open config file: ${err}`);
			api.logger.warn(`[memclaw] Please manually edit: ${configPath}`);
		});

		api.logger.info(`
╔══════════════════════════════════════════════════════════╗
║  MemClaw First Run                                       ║
║                                                          ║
║  A configuration file has been created:                  ║
║  ${configPath.padEnd(52)}║
║                                                          ║
║  Please fill in the required fields:                     ║
║  - llm.api_key (your LLM API key)                        ║
║  - embedding.api_key (your embedding API key)            ║
║                                                          ║
║  Save the file and restart OpenClaw to apply changes.    ║
╚══════════════════════════════════════════════════════════╝
    `);
	}

	// Register service lifecycle
	api.registerService({
		id: 'memclaw',
		start: async () => {
			// Skip service startup if config was just created (first run)
			// User needs to fill in API keys first
			if (created) {
				log('First run detected. Please complete configuration and restart OpenClaw.');
				return;
			}

			if (!autoStartServices) {
				log('Auto-start disabled, skipping service startup');
				return;
			}

			// Sync plugin config to config.toml if LLM/Embedding settings provided
			const pluginProvidedConfig: PluginProvidedConfig = {
				llmApiBaseUrl: config.llmApiBaseUrl,
				llmApiKey: config.llmApiKey,
				llmModel: config.llmModel,
				embeddingApiBaseUrl: config.embeddingApiBaseUrl,
				embeddingApiKey: config.embeddingApiKey,
				embeddingModel: config.embeddingModel
			};

			const syncResult = updateConfigFromPlugin(pluginProvidedConfig);
			if (syncResult.updated) {
				log(`Synced LLM/Embedding config from OpenClaw to: ${syncResult.path}`);
			}

			// Check if binaries are available
			const hasQdrant = isBinaryAvailable('qdrant');
			const hasService = isBinaryAvailable('cortex-mem-service');

			if (!hasQdrant || !hasService) {
				log('Some binaries are missing. Services may need manual setup.');
				log(`Run 'memclaw setup' or check the admin skill for installation instructions.`);
			}

			// Parse and merge config (plugin config takes precedence)
			const fileConfig = parseConfig(configPath);
			const mergedConfig = mergeConfigWithPlugin(fileConfig, pluginProvidedConfig);
			const validation = validateConfig(mergedConfig);

			if (!validation.valid) {
				api.logger.warn(`[memclaw] Configuration incomplete: ${validation.errors.join(', ')}`);
				api.logger.warn(
					`[memclaw] Please configure LLM/Embedding API keys in OpenClaw plugin settings or edit: ${configPath}`
				);
				return;
			}

			// Start services
			try {
				log('Starting services...');
				await ensureAllServices(log);

				// Switch tenant
				await client.switchTenant(tenantId);
				log(`Switched to tenant: ${tenantId}`);

				// Mark services as started only after tenant switch succeeds
				servicesStarted = true;

				log('MemClaw services started successfully');

				// Start maintenance timer (runs every 3 hours)
				maintenanceTimer = setInterval(async () => {
					try {
						log('Running scheduled maintenance...');
						const currentConfigPath = getConfigPath();

						// Run maintenance commands
						const commands = [
							['vector', 'prune'],
							['vector', 'reindex'],
							['layers', 'ensure-all']
						];

						for (const cmd of commands) {
							const result = await executeCliCommand(cmd, currentConfigPath, tenantId, 300000);
							if (!result.success) {
								log(`Maintenance command '${cmd.join(' ')}' failed: ${result.stderr}`);
							}
						}

						log('Scheduled maintenance completed');
					} catch (err) {
						log(`Maintenance error: ${err}`);
					}
				}, MAINTENANCE_INTERVAL_MS);

				log('Maintenance timer started (runs every 3 hours)');
			} catch (err) {
				api.logger.error(`[memclaw] Failed to start services: ${err}`);
				api.logger.warn('[memclaw] Memory features may not work correctly');
			}
		},
		stop: async () => {
			log('Stopping MemClaw...');

			// Clear maintenance timer
			if (maintenanceTimer) {
				clearInterval(maintenanceTimer);
				maintenanceTimer = null;
				log('Maintenance timer stopped');
			}

			servicesStarted = false;
		}
	});

	// Helper to check if services are ready
	const ensureServicesReady = async (): Promise<void> => {
		if (!servicesStarted) {
			const status = await checkServiceStatus();
			if (!status.cortexMemService) {
				throw new Error('cortex-mem-service is not running. Please start the service first.');
			}
		}
	};

	// ==================== Register Tools ====================

	// cortex_search
	api.registerTool({
		name: toolSchemas.cortex_search.name,
		description: toolSchemas.cortex_search.description,
		parameters: toolSchemas.cortex_search.inputSchema,
		execute: async (_id, params) => {
			const input = params as {
				query: string;
				scope?: string;
				limit?: number;
				min_score?: number;
				return_layers?: ('L0' | 'L1' | 'L2')[];
			};

			try {
				await ensureServicesReady();

				const results = await client.search({
					query: input.query,
					thread: input.scope,
					limit: input.limit ?? searchLimit,
					min_score: input.min_score ?? minScore,
					return_layers: input.return_layers ?? ['L0']
				});

				const formatted = results
					.map((r, i) => {
						let content = `${i + 1}. [Score: ${r.score.toFixed(2)}] URI: ${r.uri}\n`;
						content += `   Layers: ${r.layers.join(', ')}\n`;
						content += `   Snippet: ${r.snippet}\n`;
						if (r.overview) {
							content += `   Overview: ${r.overview.substring(0, 200)}...\n`;
						}
						if (r.content) {
							const preview = r.content.length > 200 ? r.content.substring(0, 200) + '...' : r.content;
							content += `   Content: ${preview}\n`;
						}
						return content;
					})
					.join('\n');

				return {
					content: `Found ${results.length} results for "${input.query}":\n\n${formatted}`,
					results: results.map((r) => ({
						uri: r.uri,
						score: r.score,
						snippet: r.snippet,
						overview: r.overview,
						content: r.content,
						layers: r.layers
					})),
					total: results.length
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_search failed: ${message}`);
				return { error: `Search failed: ${message}` };
			}
		}
	});

	// cortex_recall
	api.registerTool({
		name: toolSchemas.cortex_recall.name,
		description: toolSchemas.cortex_recall.description,
		parameters: toolSchemas.cortex_recall.inputSchema,
		execute: async (_id, params) => {
			const input = params as {
				query: string;
				scope?: string;
				limit?: number;
			};

			try {
				await ensureServicesReady();

				const results = await client.recall(input.query, input.scope, input.limit ?? 10);

				const formatted = results
					.map((r, i) => {
						let content = `${i + 1}. [Score: ${r.score.toFixed(2)}] URI: ${r.uri}\n`;
						content += `   Snippet: ${r.snippet}\n`;
						if (r.content) {
							const preview =
								r.content.length > 300 ? r.content.substring(0, 300) + '...' : r.content;
							content += `   Content: ${preview}\n`;
						}
						return content;
					})
					.join('\n');

				return {
					content: `Recalled ${results.length} memories:\n\n${formatted}`,
					results,
					total: results.length
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_recall failed: ${message}`);
				return { error: `Recall failed: ${message}` };
			}
		}
	});

	// cortex_add_memory
	api.registerTool({
		name: toolSchemas.cortex_add_memory.name,
		description: toolSchemas.cortex_add_memory.description,
		parameters: toolSchemas.cortex_add_memory.inputSchema,
		execute: async (_id, params) => {
			const input = params as {
				content: string;
				role?: string;
				session_id?: string;
				metadata?: Record<string, unknown>;
			};

			try {
				await ensureServicesReady();

				const sessionId = input.session_id ?? defaultSessionId;
				const result = await client.addMessage(sessionId, {
					role: (input.role ?? 'user') as 'user' | 'assistant' | 'system',
					content: input.content,
					metadata: input.metadata
				});

				return {
					content: `Memory stored successfully in session "${sessionId}".\nResult: ${result}`,
					success: true,
					message_uri: result
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_add_memory failed: ${message}`);
				return { error: `Failed to add memory: ${message}` };
			}
		}
	});

	// cortex_commit_session
	api.registerTool({
		name: toolSchemas.cortex_commit_session.name,
		description: toolSchemas.cortex_commit_session.description,
		parameters: toolSchemas.cortex_commit_session.inputSchema,
		execute: async (_id, params) => {
			const input = params as { session_id?: string };

			try {
				await ensureServicesReady();

				const sessionId = input.session_id ?? defaultSessionId;
				const result = await client.commitSession(sessionId);

				return {
					content: `Session "${sessionId}" committed successfully.\nStatus: ${result.status}, Messages: ${result.message_count}\n\nMemory extraction pipeline triggered.`,
					success: true,
					session: {
						thread_id: result.thread_id,
						status: result.status,
						message_count: result.message_count
					}
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_commit_session failed: ${message}`);
				return { error: `Failed to commit session: ${message}` };
			}
		}
	});

	// cortex_ls
	api.registerTool({
		name: toolSchemas.cortex_ls.name,
		description: toolSchemas.cortex_ls.description,
		parameters: toolSchemas.cortex_ls.inputSchema,
		execute: async (_id, params) => {
			const input = params as {
				uri?: string;
				recursive?: boolean;
				include_abstracts?: boolean;
			};

			try {
				await ensureServicesReady();

				const result = await client.ls({
					uri: input.uri ?? 'cortex://session',
					recursive: input.recursive ?? false,
					include_abstracts: input.include_abstracts ?? false
				});

				if (result.entries.length === 0) {
					return { content: `Directory "${result.uri}" is empty or does not exist.` };
				}

				const formatted = result.entries
					.map((e, i) => {
						let content = `${i + 1}. ${e.is_directory ? '📁' : '📄'} ${e.name}\n`;
						content += `   URI: ${e.uri}\n`;
						if (e.is_directory) {
							content += `   Type: Directory\n`;
						} else {
							content += `   Size: ${e.size} bytes\n`;
						}
						if (e.abstract_text) {
							const preview = e.abstract_text.length > 100 
								? e.abstract_text.substring(0, 100) + '...' 
								: e.abstract_text;
							content += `   Abstract: ${preview}\n`;
						}
						return content;
					})
					.join('\n');

				return {
					content: `Directory "${result.uri}" (${result.total} entries):\n\n${formatted}`,
					entries: result.entries,
					total: result.total
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_ls failed: ${message}`);
				return { error: `List directory failed: ${message}` };
			}
		}
	});

	// cortex_get_abstract
	api.registerTool({
		name: toolSchemas.cortex_get_abstract.name,
		description: toolSchemas.cortex_get_abstract.description,
		parameters: toolSchemas.cortex_get_abstract.inputSchema,
		execute: async (_id, params) => {
			const input = params as { uri: string };

			try {
				await ensureServicesReady();

				const result = await client.getAbstract(input.uri);

				return {
					content: `L0 Abstract for "${result.uri}" (~${result.token_count} tokens):\n\n${result.content}`,
					uri: result.uri,
					abstract: result.content,
					token_count: result.token_count,
					layer: result.layer
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_get_abstract failed: ${message}`);
				return { error: `Get abstract failed: ${message}` };
			}
		}
	});

	// cortex_get_overview
	api.registerTool({
		name: toolSchemas.cortex_get_overview.name,
		description: toolSchemas.cortex_get_overview.description,
		parameters: toolSchemas.cortex_get_overview.inputSchema,
		execute: async (_id, params) => {
			const input = params as { uri: string };

			try {
				await ensureServicesReady();

				const result = await client.getOverview(input.uri);

				return {
					content: `L1 Overview for "${result.uri}" (~${result.token_count} tokens):\n\n${result.content}`,
					uri: result.uri,
					overview: result.content,
					token_count: result.token_count,
					layer: result.layer
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_get_overview failed: ${message}`);
				return { error: `Get overview failed: ${message}` };
			}
		}
	});

	// cortex_get_content
	api.registerTool({
		name: toolSchemas.cortex_get_content.name,
		description: toolSchemas.cortex_get_content.description,
		parameters: toolSchemas.cortex_get_content.inputSchema,
		execute: async (_id, params) => {
			const input = params as { uri: string };

			try {
				await ensureServicesReady();

				const result = await client.getContent(input.uri);

				return {
					content: `L2 Full Content for "${result.uri}" (~${result.token_count} tokens):\n\n${result.content}`,
					uri: result.uri,
					full_content: result.content,
					token_count: result.token_count,
					layer: result.layer
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_get_content failed: ${message}`);
				return { error: `Get content failed: ${message}` };
			}
		}
	});

	// cortex_explore
	api.registerTool({
		name: toolSchemas.cortex_explore.name,
		description: toolSchemas.cortex_explore.description,
		parameters: toolSchemas.cortex_explore.inputSchema,
		execute: async (_id, params) => {
			const input = params as {
				query: string;
				start_uri?: string;
				return_layers?: ('L0' | 'L1' | 'L2')[];
			};

			try {
				await ensureServicesReady();

				const result = await client.explore({
					query: input.query,
					start_uri: input.start_uri ?? 'cortex://session',
					return_layers: input.return_layers ?? ['L0']
				});

				// Format exploration path
				const pathFormatted = result.exploration_path
					.map((item, i) => {
						let content = `${i + 1}. [${item.relevance_score.toFixed(2)}] ${item.uri}\n`;
						if (item.abstract_text) {
							const preview = item.abstract_text.length > 80
								? item.abstract_text.substring(0, 80) + '...'
								: item.abstract_text;
							content += `   Abstract: ${preview}\n`;
						}
						return content;
					})
					.join('\n');

				// Format matches
				const matchesFormatted = result.matches
					.map((m, i) => {
						let content = `${i + 1}. [${m.score.toFixed(2)}] ${m.uri}\n`;
						content += `   Layers: ${m.layers.join(', ')}\n`;
						content += `   Snippet: ${m.snippet}\n`;
						return content;
					})
					.join('\n');

				return {
					content: `Exploration for "${input.query}" starting from "${input.start_uri ?? 'cortex://session'}":\n\n` +
						`**Exploration Path** (${result.total_explored} items):\n${pathFormatted}\n\n` +
						`**Matches** (${result.total_matches} found):\n${matchesFormatted}`,
					exploration_path: result.exploration_path,
					matches: result.matches,
					total_explored: result.total_explored,
					total_matches: result.total_matches
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_explore failed: ${message}`);
				return { error: `Explore failed: ${message}` };
			}
		}
	});

	// cortex_migrate
	api.registerTool({
		name: toolSchemas.cortex_migrate.name,
		description: toolSchemas.cortex_migrate.description,
		parameters: toolSchemas.cortex_migrate.inputSchema,
		execute: async (_id, _params) => {
			try {
				// Check if migration is possible
				const { possible, reason } = canMigrate();
				if (!possible) {
					return { content: `Migration not possible: ${reason}` };
				}

				// Run migration
				const result = await migrateFromOpenClaw((msg) => api.logger.info(`[migrate] ${msg}`));

				return {
					content: `Migration completed!\n- Daily logs migrated: ${result.dailyLogsMigrated}\n- MEMORY.md migrated: ${result.memoryMdMigrated}\n- Sessions created: ${result.sessionsCreated.length}\n${result.errors.length > 0 ? `- Errors: ${result.errors.length}` : ''}`,
					result
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`cortex_migrate failed: ${message}`);
				return { error: `Migration failed: ${message}` };
			}
		}
	});

	// cortex_maintenance
	api.registerTool({
		name: toolSchemas.cortex_maintenance.name,
		description: toolSchemas.cortex_maintenance.description,
		parameters: toolSchemas.cortex_maintenance.inputSchema,
		execute: async (_id, params) => {
			const input = params as {
				dryRun?: boolean;
				commands?: string[];
			};

			const dryRun = input.dryRun ?? false;
			const commands = input.commands ?? ['prune', 'reindex', 'ensure-all'];
			const currentConfigPath = getConfigPath();

			const results: { command: string; success: boolean; output: string }[] = [];

			for (const cmd of commands) {
				let cliArgs: string[];
				let description: string;

				switch (cmd) {
					case 'prune':
						cliArgs = ['vector', 'prune'];
						if (dryRun) cliArgs.push('--dry-run');
						description = 'Vector Prune';
						break;
					case 'reindex':
						cliArgs = ['vector', 'reindex'];
						description = 'Vector Reindex';
						break;
					case 'ensure-all':
						cliArgs = ['layers', 'ensure-all'];
						description = 'Layers Ensure-All';
						break;
					default:
						continue;
				}

				api.logger.info(`[maintenance] Running: ${description}`);

				try {
					const result = await executeCliCommand(
						cliArgs,
						currentConfigPath,
						tenantId,
						300000 // 5 minute timeout for maintenance
					);

					results.push({
						command: description,
						success: result.success,
						output: result.stdout || result.stderr
					});

					if (!result.success) {
						api.logger.warn(`[memclaw] [maintenance] ${description} failed: ${result.stderr}`);
					}
				} catch (error) {
					const message = error instanceof Error ? error.message : String(error);
					results.push({
						command: description,
						success: false,
						output: message
					});
					api.logger.error(`[maintenance] ${description} error: ${message}`);
				}
			}

			const summary = results.map((r) => `${r.command}: ${r.success ? 'OK' : 'FAILED'}`).join('\n');

			const successCount = results.filter((r) => r.success).length;

			return {
				content: `Maintenance ${dryRun ? '(dry run) ' : ''}completed:\n${summary}\n\n${successCount}/${results.length} commands succeeded.`,
				dryRun,
				results,
				success: successCount === results.length
			};
		}
	});

	log('MemClaw plugin initialized');

	return {
		id: 'memclaw',
		name: 'MemClaw',
		version: '0.1.0'
	};
}