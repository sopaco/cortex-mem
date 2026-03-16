/**
 * MemClaw Plugin Implementation
 *
 * Provides layered semantic memory for OpenClaw with:
 * - Automatic service startup
 * - Memory tools (search, recall, add, list, close)
 * - Migration from OpenClaw native memory
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
export declare function createPlugin(api: PluginAPI): {
    id: string;
    name: string;
    version: string;
};
export {};
//# sourceMappingURL=plugin-impl.d.ts.map