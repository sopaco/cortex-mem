/**
 * Configuration management for MemClaw
 *
 * Handles platform-specific config paths, config file generation,
 * and auto-opening config files for user editing.
 */
export declare function getDataDir(): string;
export declare function getConfigPath(): string;
export interface MemClawConfig {
    qdrant: {
        url: string;
        collection_name: string;
        timeout_secs: number;
    };
    llm: {
        api_base_url: string;
        api_key: string;
        model_efficient: string;
        temperature: number;
        max_tokens: number;
    };
    embedding: {
        api_base_url: string;
        api_key: string;
        model_name: string;
        batch_size: number;
        timeout_secs: number;
    };
    server: {
        host: string;
        port: number;
    };
    logging: {
        enabled: boolean;
        log_directory: string;
        level: string;
    };
    cortex: {
        enable_intent_analysis: boolean;
    };
}
export declare function generateConfigTemplate(): string;
export declare function ensureConfigExists(): {
    created: boolean;
    path: string;
};
export declare function openConfigFile(configPath: string): Promise<void>;
export declare function parseConfig(configPath: string): MemClawConfig;
export declare function validateConfig(config: MemClawConfig): {
    valid: boolean;
    errors: string[];
};
//# sourceMappingURL=config.d.ts.map