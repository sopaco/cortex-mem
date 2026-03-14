/**
 * Cortex Memory Plugin for OpenClaw
 *
 * Provides layered semantic memory with L0/L1/L2 tiered retrieval.
 *
 * Installation:
 *   openclaw plugins install @cortex-mem-openclaw/openclaw-plugin
 *
 * Configuration (in openclaw.json):
 *   {
 *     "plugins": {
 *       "entries": {
 *         "cortex-mem-openclaw": {
 *           "enabled": true,
 *           "config": {
 *             "serviceUrl": "http://127.0.0.1:8085",
 *             "tenantId": "tenant_claw",
 *             "defaultSessionId": "default",
 *             "searchLimit": 10,
 *             "minScore": 0.6
 *           }
 *         }
 *       }
 *     }
 *   }
 */
interface PluginLogger {
    debug?: (msg: string, ...args: unknown[]) => void;
    info: (msg: string, ...args: unknown[]) => void;
    warn: (msg: string, ...args: unknown[]) => void;
    error: (msg: string, ...args: unknown[]) => void;
}
interface ToolDefinition {
    name: string;
    description: string;
    /**
     * JSON Schema for tool inputs.
     * OpenClaw uses 'parameters', NOT 'inputSchema'.
     */
    parameters: object;
    /**
     * Tool execution function.
     * OpenClaw uses 'execute(_id, params)', NOT 'handler(args)'.
     */
    execute: (_id: string, params: Record<string, unknown>) => Promise<unknown>;
    /** Optional: mark tool as opt-in (not auto-enabled) */
    optional?: boolean;
}
interface PluginAPI {
    /**
     * Plugin-specific configuration from openclaw.json
     * Access via api.pluginConfig, NOT api.getConfig()
     */
    pluginConfig?: Record<string, unknown>;
    registerTool(tool: ToolDefinition, opts?: {
        optional?: boolean;
    }): void;
    logger: PluginLogger;
}
export default function cortexMemPlugin(api: PluginAPI): {
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
        };
        required: string[];
    };
    register(api: PluginAPI): {
        id: string;
        name: string;
        version: string;
    };
};
export {};
//# sourceMappingURL=index.d.ts.map