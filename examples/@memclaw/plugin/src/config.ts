/**
 * Configuration management for MemClaw
 *
 * Handles platform-specific config paths, config file generation,
 * and auto-opening config files for user editing.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { spawn } from 'child_process';
import TOML from 'smol-toml';

// Platform-specific paths
export function getDataDir(): string {
	const platform = process.platform;

	if (platform === 'win32') {
		return path.join(
			process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local'),
			'memclaw'
		);
	} else if (platform === 'darwin') {
		return path.join(os.homedir(), 'Library', 'Application Support', 'memclaw');
	} else {
		return path.join(os.homedir(), '.local', 'share', 'memclaw');
	}
}

export function getConfigPath(): string {
	return path.join(getDataDir(), 'config.toml');
}

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

export function generateConfigTemplate(): string {
	return `# MemClaw Configuration
#
# This file was auto-generated. Please fill in the required values below.
# All sections are required - missing sections will cause config to be ignored.

# Qdrant Vector Database Configuration
[qdrant]
url = "http://localhost:6334"
collection_name = "memclaw"
# Embedding dimension (optional, auto-detected if not set)
# For text-embedding-3-small: 1536
# For text-embedding-3-large: 3072
# embedding_dim = 1536
timeout_secs = 30

# LLM Configuration [REQUIRED for memory processing]
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = ""
model_efficient = "gpt-5-mini"
temperature = 0.1
max_tokens = 65536

# Embedding Configuration [REQUIRED for vector search]
[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = ""
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

# Service Configuration
[server]
host = "localhost"
port = 8085
cors_origins = ["*"]

# Logging Configuration
[logging]
enabled = false
log_directory = "logs"
level = "info"

# Cortex Memory Settings
[cortex]
enable_intent_analysis = false
`;
}

export function ensureConfigExists(): { created: boolean; path: string } {
	const dataDir = getDataDir();
	const configPath = getConfigPath();

	if (!fs.existsSync(dataDir)) {
		fs.mkdirSync(dataDir, { recursive: true });
	}

	if (!fs.existsSync(configPath)) {
		const template = generateConfigTemplate();
		fs.writeFileSync(configPath, template, 'utf-8');
		return { created: true, path: configPath };
	}

	return { created: false, path: configPath };
}

export function openConfigFile(configPath: string): Promise<void> {
	return new Promise((resolve, reject) => {
		const platform = process.platform;
		let command: string;
		let args: string[] = [];

		if (platform === 'win32') {
			command = 'cmd';
			args = ['/c', 'start', '""', configPath];
		} else if (platform === 'darwin') {
			command = 'open';
			args = [configPath];
		} else {
			command = 'xdg-open';
			args = [configPath];
		}

		const proc = spawn(command, args, { detached: true, stdio: 'ignore' });
		proc.on('error', (err) => {
			reject(err);
		});
		proc.unref();
		resolve();
	});
}

/**
 * Parse TOML config file using smol-toml library
 * Supports full TOML syntax including arrays, nested tables, etc.
 */
export function parseConfig(configPath: string): MemClawConfig {
	const content = fs.readFileSync(configPath, 'utf-8');
	
	// Parse using smol-toml
	let parsed: Record<string, unknown>;
	try {
		parsed = TOML.parse(content) as Record<string, unknown>;
	} catch (error) {
		// If parsing fails, return defaults
		console.error('Failed to parse config.toml:', error);
		parsed = {};
	}

	// Apply defaults for missing sections
	return {
		qdrant: {
			url: 'http://localhost:6334',
			collection_name: 'memclaw',
			timeout_secs: 30,
			...((parsed.qdrant as Record<string, unknown>) || {})
		},
		llm: {
			api_base_url: 'https://api.openai.com/v1',
			api_key: '',
			model_efficient: 'gpt-5-mini',
			temperature: 0.1,
			max_tokens: 4096,
			...((parsed.llm as Record<string, unknown>) || {})
		},
		embedding: {
			api_base_url: 'https://api.openai.com/v1',
			api_key: '',
			model_name: 'text-embedding-3-small',
			batch_size: 10,
			timeout_secs: 30,
			...((parsed.embedding as Record<string, unknown>) || {})
		},
		server: {
			host: 'localhost',
			port: 8085,
			...((parsed.server as Record<string, unknown>) || {})
		},
		logging: {
			enabled: false,
			log_directory: 'logs',
			level: 'info',
			...((parsed.logging as Record<string, unknown>) || {})
		},
		cortex: {
			enable_intent_analysis: false,
			...((parsed.cortex as Record<string, unknown>) || {})
		}
	};
}

export function validateConfig(config: MemClawConfig): {
	valid: boolean;
	errors: string[];
} {
	const errors: string[] = [];

	if (!config.llm.api_key || config.llm.api_key === '') {
		errors.push('llm.api_key is required');
	}

	if (!config.embedding.api_key || config.embedding.api_key === '') {
		// Allow using llm.api_key for embedding if not specified
		if (config.llm.api_key && config.llm.api_key !== '') {
			config.embedding.api_key = config.llm.api_key;
		} else {
			errors.push('embedding.api_key is required');
		}
	}

	return {
		valid: errors.length === 0,
		errors
	};
}

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
export function updateConfigFromPlugin(pluginConfig: PluginProvidedConfig): {
	updated: boolean;
	path: string;
} {
	const configPath = getConfigPath();

	// Ensure config file exists
	ensureConfigExists();

	// Parse existing config
	const existingConfig = parseConfig(configPath);

	// Build updated config
	const updatedConfig: MemClawConfig = {
		qdrant: existingConfig.qdrant,
		llm: {
			...existingConfig.llm,
			api_base_url: pluginConfig.llmApiBaseUrl || existingConfig.llm.api_base_url,
			api_key: pluginConfig.llmApiKey || existingConfig.llm.api_key,
			model_efficient: pluginConfig.llmModel || existingConfig.llm.model_efficient
		},
		embedding: {
			...existingConfig.embedding,
			api_base_url: pluginConfig.embeddingApiBaseUrl || existingConfig.embedding.api_base_url,
			api_key: pluginConfig.embeddingApiKey || existingConfig.embedding.api_key,
			model_name: pluginConfig.embeddingModel || existingConfig.embedding.model_name
		},
		server: existingConfig.server,
		logging: existingConfig.logging,
		cortex: existingConfig.cortex
	};

	// Check if any changes were made
	const hasChanges = 
		(pluginConfig.llmApiKey && pluginConfig.llmApiKey !== existingConfig.llm.api_key) ||
		(pluginConfig.llmApiBaseUrl && pluginConfig.llmApiBaseUrl !== existingConfig.llm.api_base_url) ||
		(pluginConfig.llmModel && pluginConfig.llmModel !== existingConfig.llm.model_efficient) ||
		(pluginConfig.embeddingApiKey && pluginConfig.embeddingApiKey !== existingConfig.embedding.api_key) ||
		(pluginConfig.embeddingApiBaseUrl && pluginConfig.embeddingApiBaseUrl !== existingConfig.embedding.api_base_url) ||
		(pluginConfig.embeddingModel && pluginConfig.embeddingModel !== existingConfig.embedding.model_name);

	if (!hasChanges) {
		return { updated: false, path: configPath };
	}

	// Serialize and write using smol-toml
	const tomlContent = TOML.stringify(updatedConfig as unknown as Record<string, unknown>);
	fs.writeFileSync(configPath, tomlContent, 'utf-8');

	return { updated: true, path: configPath };
}

/**
 * Merge plugin config with file config, preferring plugin config values
 */
export function mergeConfigWithPlugin(
	fileConfig: MemClawConfig,
	pluginConfig: PluginProvidedConfig
): MemClawConfig {
	return {
		...fileConfig,
		llm: {
			...fileConfig.llm,
			api_base_url: pluginConfig.llmApiBaseUrl || fileConfig.llm.api_base_url,
			api_key: pluginConfig.llmApiKey || fileConfig.llm.api_key,
			model_efficient: pluginConfig.llmModel || fileConfig.llm.model_efficient
		},
		embedding: {
			...fileConfig.embedding,
			api_base_url: pluginConfig.embeddingApiBaseUrl || fileConfig.embedding.api_base_url,
			api_key: pluginConfig.embeddingApiKey || fileConfig.embedding.api_key,
			model_name: pluginConfig.embeddingModel || fileConfig.embedding.model_name
		}
	};
}
