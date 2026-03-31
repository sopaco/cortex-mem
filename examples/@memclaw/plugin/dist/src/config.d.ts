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
        embedding_dim?: number;
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
        cors_origins?: string[];
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
/**
 * Parse TOML config file using smol-toml library
 * Supports full TOML syntax including arrays, nested tables, etc.
 */
export declare function parseConfig(configPath: string): MemClawConfig;
export declare function validateConfig(config: MemClawConfig): {
    valid: boolean;
    errors: string[];
};
/**
 * Configuration provided by OpenClaw plugin config
 * These values will be synced to config.toml if provided
 */
export interface PluginProvidedConfig {
    llmApiBaseUrl?: string;
    llmApiKey?: string;
    llmModel?: string;
    embeddingApiBaseUrl?: string;
    embeddingApiKey?: string;
    embeddingModel?: string;
}
/**
 * Update config.toml with values from OpenClaw plugin config
 * Uses smol-toml for proper TOML serialization
 */
export declare function updateConfigFromPlugin(pluginConfig: PluginProvidedConfig): {
    updated: boolean;
    path: string;
};
/**
 * Merge plugin config with file config, preferring plugin config values
 */
export declare function mergeConfigWithPlugin(fileConfig: MemClawConfig, pluginConfig: PluginProvidedConfig): MemClawConfig;
//# sourceMappingURL=config.d.ts.map