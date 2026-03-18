"use strict";
/**
 * Configuration management for MemClaw
 *
 * Handles platform-specific config paths, config file generation,
 * and auto-opening config files for user editing.
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.getDataDir = getDataDir;
exports.getConfigPath = getConfigPath;
exports.generateConfigTemplate = generateConfigTemplate;
exports.ensureConfigExists = ensureConfigExists;
exports.openConfigFile = openConfigFile;
exports.parseConfig = parseConfig;
exports.validateConfig = validateConfig;
exports.updateConfigFromPlugin = updateConfigFromPlugin;
exports.mergeConfigWithPlugin = mergeConfigWithPlugin;
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const os = __importStar(require("os"));
const child_process_1 = require("child_process");
// Platform-specific paths
function getDataDir() {
    const platform = process.platform;
    if (platform === 'win32') {
        return path.join(process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local'), 'memclaw');
    }
    else if (platform === 'darwin') {
        return path.join(os.homedir(), 'Library', 'Application Support', 'memclaw');
    }
    else {
        return path.join(os.homedir(), '.local', 'share', 'memclaw');
    }
}
function getConfigPath() {
    return path.join(getDataDir(), 'config.toml');
}
function generateConfigTemplate() {
    return `# MemClaw Configuration
#
# This file was auto-generated. Please fill in the required values below.
# All sections are required - missing sections will cause config to be ignored.

# Qdrant Vector Database Configuration
[qdrant]
url = "http://localhost:6334"
collection_name = "memclaw"
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
function ensureConfigExists() {
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
function openConfigFile(configPath) {
    return new Promise((resolve, reject) => {
        const platform = process.platform;
        let command;
        let args = [];
        if (platform === 'win32') {
            command = 'cmd';
            args = ['/c', 'start', '""', configPath];
        }
        else if (platform === 'darwin') {
            command = 'open';
            args = [configPath];
        }
        else {
            command = 'xdg-open';
            args = [configPath];
        }
        const proc = (0, child_process_1.spawn)(command, args, { detached: true, stdio: 'ignore' });
        proc.on('error', (err) => {
            reject(err);
        });
        proc.unref();
        resolve();
    });
}
function parseConfig(configPath) {
    const content = fs.readFileSync(configPath, 'utf-8');
    const config = {};
    let currentSection = '';
    for (const line of content.split('\n')) {
        const trimmed = line.trim();
        // Skip comments and empty lines
        if (trimmed.startsWith('#') || trimmed === '')
            continue;
        // Section header
        const sectionMatch = trimmed.match(/^\[(\w+)\]$/);
        if (sectionMatch) {
            currentSection = sectionMatch[1];
            config[currentSection] = {};
            continue;
        }
        // Key-value pair
        const kvMatch = trimmed.match(/^(\w+)\s*=\s*"([^"]*)"(?:\s*$|\s*#)/) ||
            trimmed.match(/^(\w+)\s*=\s*(\d+(?:\.\d+)?)(?:\s*$|\s*#)/) ||
            trimmed.match(/^(\w+)\s*=\s*(true|false)(?:\s*$|\s*#)/);
        if (kvMatch && currentSection) {
            const key = kvMatch[1];
            let value = kvMatch[2];
            // Convert to appropriate type
            if (value === 'true')
                value = true;
            else if (value === 'false')
                value = false;
            else if (/^\d+$/.test(value))
                value = parseInt(value, 10);
            else if (/^\d+\.\d+$/.test(value))
                value = parseFloat(value);
            config[currentSection] = config[currentSection] || {};
            config[currentSection][key] = value;
        }
    }
    // Apply defaults
    return {
        qdrant: {
            url: 'http://localhost:6334',
            collection_name: 'memclaw',
            timeout_secs: 30,
            ...(config.qdrant || {})
        },
        llm: {
            api_base_url: 'https://api.openai.com/v1',
            api_key: '',
            model_efficient: 'gpt-5-mini',
            temperature: 0.1,
            max_tokens: 4096,
            ...(config.llm || {})
        },
        embedding: {
            api_base_url: 'https://api.openai.com/v1',
            api_key: '',
            model_name: 'text-embedding-3-small',
            batch_size: 10,
            timeout_secs: 30,
            ...(config.embedding || {})
        },
        server: {
            host: 'localhost',
            port: 8085,
            ...(config.server || {})
        },
        logging: {
            enabled: false,
            log_directory: 'logs',
            level: 'info',
            ...(config.logging || {})
        },
        cortex: {
            enable_intent_analysis: false,
            ...(config.cortex || {})
        }
    };
}
function validateConfig(config) {
    const errors = [];
    if (!config.llm.api_key || config.llm.api_key === '') {
        errors.push('llm.api_key is required');
    }
    if (!config.embedding.api_key || config.embedding.api_key === '') {
        // Allow using llm.api_key for embedding if not specified
        if (config.llm.api_key && config.llm.api_key !== '') {
            config.embedding.api_key = config.llm.api_key;
        }
        else {
            errors.push('embedding.api_key is required');
        }
    }
    return {
        valid: errors.length === 0,
        errors
    };
}
/**
 * Update config.toml with values from OpenClaw plugin config
 * Only updates fields that are provided (non-empty) in pluginConfig
 */
function updateConfigFromPlugin(pluginConfig) {
    const configPath = getConfigPath();
    // Ensure config file exists
    ensureConfigExists();
    // Parse existing config
    const existingConfig = parseConfig(configPath);
    // Track if any changes were made
    let updated = false;
    // Build updated config sections
    const updates = [];
    // LLM config updates
    if (pluginConfig.llmApiKey && pluginConfig.llmApiKey !== '') {
        updates.push({ section: 'llm', key: 'api_key', value: pluginConfig.llmApiKey });
        updated = true;
    }
    if (pluginConfig.llmApiBaseUrl && pluginConfig.llmApiBaseUrl !== '') {
        updates.push({ section: 'llm', key: 'api_base_url', value: pluginConfig.llmApiBaseUrl });
        updated = true;
    }
    if (pluginConfig.llmModel && pluginConfig.llmModel !== '') {
        updates.push({ section: 'llm', key: 'model_efficient', value: pluginConfig.llmModel });
        updated = true;
    }
    // Embedding config updates
    if (pluginConfig.embeddingApiKey && pluginConfig.embeddingApiKey !== '') {
        updates.push({ section: 'embedding', key: 'api_key', value: pluginConfig.embeddingApiKey });
        updated = true;
    }
    if (pluginConfig.embeddingApiBaseUrl && pluginConfig.embeddingApiBaseUrl !== '') {
        updates.push({
            section: 'embedding',
            key: 'api_base_url',
            value: pluginConfig.embeddingApiBaseUrl
        });
        updated = true;
    }
    if (pluginConfig.embeddingModel && pluginConfig.embeddingModel !== '') {
        updates.push({ section: 'embedding', key: 'model_name', value: pluginConfig.embeddingModel });
        updated = true;
    }
    if (!updated) {
        return { updated: false, path: configPath };
    }
    // Read current content
    let content = fs.readFileSync(configPath, 'utf-8');
    // Apply each update
    for (const { section, key, value } of updates) {
        // Escape value for TOML string
        const escapedValue = value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
        // Pattern to match the key in the correct section
        // This handles both existing keys and missing keys
        const sectionPattern = new RegExp(`(\\[${section}\\][^\\[]*?)(${key}\\s*=\\s*)"[^"]*"`, 's');
        const keyExistsInSection = sectionPattern.test(content);
        if (keyExistsInSection) {
            // Update existing key
            content = content.replace(sectionPattern, `$1$2"${escapedValue}"`);
        }
        else {
            // Add key to section
            const sectionStartPattern = new RegExp(`(\\[${section}\\]\\n)`, '');
            if (sectionStartPattern.test(content)) {
                content = content.replace(sectionStartPattern, `$1${key} = "${escapedValue}"\n`);
            }
        }
    }
    // Write updated content
    fs.writeFileSync(configPath, content, 'utf-8');
    return { updated: true, path: configPath };
}
/**
 * Merge plugin config with file config, preferring plugin config values
 */
function mergeConfigWithPlugin(fileConfig, pluginConfig) {
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
//# sourceMappingURL=config.js.map