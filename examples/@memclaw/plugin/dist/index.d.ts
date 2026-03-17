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
export type { CortexMemClient } from "./src/client.js";
export type { MemClawConfig } from "./src/config.js";
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
    registerTool(tool: ToolDefinition, opts?: {
        optional?: boolean;
    }): void;
    registerService(service: {
        id: string;
        start: () => Promise<void>;
        stop: () => Promise<void>;
    }): void;
    logger: PluginLogger;
}
export default function memclawPlugin(api: PluginAPI): {
    id: string;
    name: string;
    version: string;
};
export declare const plugin: {
    id: string;
    name: string;
    version: string;
    configSchema: {
        type: string;
        properties: {
            serviceUrl: {
                type: string;
                default: string;
            };
            defaultSessionId: {
                type: string;
                default: string;
            };
            searchLimit: {
                type: string;
                default: number;
            };
            minScore: {
                type: string;
                default: number;
            };
            tenantId: {
                type: string;
                default: string;
            };
            autoStartServices: {
                type: string;
                default: boolean;
            };
        };
        required: never[];
    };
    register(api: PluginAPI): {
        id: string;
        name: string;
        version: string;
    };
};
//# sourceMappingURL=index.d.ts.map