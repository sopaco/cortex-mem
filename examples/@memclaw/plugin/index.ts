/**
 * MemClaw - Layered Semantic Memory for OpenClaw
 *
 * Provides:
 * - L0/L1/L2 tiered memory retrieval
 * - Automatic service startup (Qdrant + cortex-mem-service)
 * - Migration from OpenClaw native memory
 *
 * Installation:
 *   openclaw plugins install memclaw
 *
 * Configuration (in openclaw.json):
 *   {
 *     "plugins": {
 *       "entries": {
 *         "memclaw": {
 *           "enabled": true,
 *           "config": {
 *             "serviceUrl": "http://localhost:8085",
 *             "tenantId": "tenant_claw",
 *             "autoStartServices": true
 *           }
 *         }
 *       }
 *     }
 *   }
 */

import { createPlugin } from './plugin-impl.js';

// Re-export types
export type { CortexMemClient } from './src/client.js';
export type { MemClawConfig } from './src/config.js';

// OpenClaw Plugin API types
interface PluginLogger {
	debug?: (msg: string, ...args: unknown[]) => void;
	info: (msg: string, ...args: unknown[]) => void;
	warn: (msg: string, ...args: unknown[]) => void;
	error: (msg: string, ...args: unknown[]) => void;
}

interface ToolDefinition {
	name: string;
	description: string;
	parameters: object;
	execute: (_id: string, params: Record<string, unknown>) => Promise<unknown>;
	optional?: boolean;
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

// Default export - main plugin function
export default function memclawPlugin(api: PluginAPI) {
	return createPlugin(api);
}

// Named export - object style registration
export const plugin = {
	id: 'memclaw',
	name: 'MemClaw',
	version: '0.9.10',
	configSchema: {
		type: 'object',
		properties: {
			serviceUrl: { type: 'string', default: 'http://localhost:8085' },
			defaultSessionId: { type: 'string', default: 'default' },
			searchLimit: { type: 'integer', default: 10 },
			minScore: { type: 'number', default: 0.6 },
			tenantId: { type: 'string', default: 'tenant_claw' },
			autoStartServices: { type: 'boolean', default: true }
		},
		required: []
	},
	register(api: PluginAPI) {
		return createPlugin(api);
	}
};
