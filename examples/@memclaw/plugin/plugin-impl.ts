/**
 * MemClaw Plugin Implementation
 *
 * Provides layered semantic memory for OpenClaw with:
 * - Automatic service startup
 * - Memory tools (search, recall, add, list, close)
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
Returns relevant memories ranked by relevance score.

Use this tool when you need to:
- Find past conversations or decisions
- Search for specific information across all sessions
- Discover related memories by semantic similarity`,
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
				}
			},
			required: ['query']
		}
	},

	cortex_recall: {
		name: 'cortex_recall',
		description: `Recall memories using L0/L1/L2 tiered retrieval.

The search engine internally performs tiered retrieval:
- L0 (Abstract): Quick filtering by summary
- L1 (Overview): Context refinement
- L2 (Full): Precise matching with full content

Returns results with snippet (summary) and content (if available).

Use this when you need memories with more context than a simple search.`,
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
				}
			},
			required: ['content']
		}
	},

	cortex_list_sessions: {
		name: 'cortex_list_sessions',
		description: `List all memory sessions with their status.
Shows session IDs, message counts, and creation/update times.`,
		inputSchema: {
			type: 'object',
			properties: {}
		}
	},

	cortex_close_session: {
		name: 'cortex_close_session',
		description: `Trigger memory extraction and archival for accumulated conversation content.

**IMPORTANT - Call this tool proactively and periodically, NOT just at conversation end.**

This triggers the complete memory processing pipeline:
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
					description: 'Session/thread ID to process (uses default if not specified)'
				}
			}
		}
	},

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
				servicesStarted = true;

				// Switch tenant
				await client.switchTenant(tenantId);
				log(`Switched to tenant: ${tenantId}`);

				log('MemClaw services started successfully');

				// Start maintenance timer (runs every 3 hours)
				maintenanceTimer = setInterval(async () => {
					try {
						log('Running scheduled maintenance...');
						const configPath = getConfigPath();

						// Run maintenance commands
						const commands = [
							['vector', 'prune'],
							['vector', 'reindex'],
							['layers', 'ensure-all']
						];

						for (const cmd of commands) {
							const result = await executeCliCommand(cmd, configPath, tenantId, 300000);
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

	// Register tools

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
			};

			try {
				await ensureServicesReady();

				const results = await client.search({
					query: input.query,
					thread: input.scope,
					limit: input.limit ?? searchLimit,
					min_score: input.min_score ?? minScore
				});

				const formatted = results
					.map((r, i) => `${i + 1}. [Score: ${r.score.toFixed(2)}] ${r.snippet}\n   URI: ${r.uri}`)
					.join('\n\n');

				return {
					content: `Found ${results.length} results for "${input.query}":\n\n${formatted}`,
					results: results.map((r) => ({
						uri: r.uri,
						score: r.score,
						snippet: r.snippet
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
			};

			try {
				await ensureServicesReady();

				const sessionId = input.session_id ?? defaultSessionId;
				const result = await client.addMessage(sessionId, {
					role: (input.role ?? 'user') as 'user' | 'assistant' | 'system',
					content: input.content
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

	// cortex_list_sessions
	api.registerTool({
		name: toolSchemas.cortex_list_sessions.name,
		description: toolSchemas.cortex_list_sessions.description,
		parameters: toolSchemas.cortex_list_sessions.inputSchema,
		execute: async (_id, _params) => {
			try {
				await ensureServicesReady();

				const sessions = await client.listSessions();

				if (sessions.length === 0) {
					return { content: 'No sessions found.' };
				}

				const formatted = sessions
					.map((s, i) => {
						const created = new Date(s.created_at).toLocaleDateString();
						return `${i + 1}. ${s.thread_id} (${s.status}, ${s.message_count} messages, created ${created})`;
					})
					.join('\n');

				return {
					content: `Found ${sessions.length} sessions:\n\n${formatted}`,
					sessions: sessions.map((s) => ({
						thread_id: s.thread_id,
						status: s.status,
						message_count: s.message_count,
						created_at: s.created_at
					}))
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_list_sessions failed: ${message}`);
				return { error: `Failed to list sessions: ${message}` };
			}
		}
	});

	// cortex_close_session
	api.registerTool({
		name: toolSchemas.cortex_close_session.name,
		description: toolSchemas.cortex_close_session.description,
		parameters: toolSchemas.cortex_close_session.inputSchema,
		execute: async (_id, params) => {
			const input = params as { session_id?: string };

			try {
				await ensureServicesReady();

				const sessionId = input.session_id ?? defaultSessionId;
				const result = await client.closeSession(sessionId);

				return {
					content: `Session "${sessionId}" closed successfully.\nStatus: ${result.status}, Messages: ${result.message_count}\n\nMemory extraction pipeline triggered.`,
					success: true,
					session: {
						thread_id: result.thread_id,
						status: result.status,
						message_count: result.message_count
					}
				};
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				api.logger.error(`[memclaw] cortex_close_session failed: ${message}`);
				return { error: `Failed to close session: ${message}` };
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
