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
exports.getConfigDir = getConfigDir;
exports.getDataDir = getDataDir;
exports.getConfigPath = getConfigPath;
exports.generateConfigTemplate = generateConfigTemplate;
exports.ensureConfigExists = ensureConfigExists;
exports.openConfigFile = openConfigFile;
exports.parseConfig = parseConfig;
exports.validateConfig = validateConfig;
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const os = __importStar(require("os"));
const child_process_1 = require("child_process");
// Platform-specific paths
function getConfigDir() {
    const platform = process.platform;
    if (platform === "win32") {
        return path.join(process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming"), "memclaw");
    }
    else if (platform === "darwin") {
        return path.join(os.homedir(), "Library", "Application Support", "memclaw");
    }
    else {
        return path.join(os.homedir(), ".config", "memclaw");
    }
}
function getDataDir() {
    const platform = process.platform;
    if (platform === "win32") {
        return path.join(process.env.LOCALAPPDATA || path.join(os.homedir(), "AppData", "Local"), "memclaw", "data");
    }
    else if (platform === "darwin") {
        return path.join(os.homedir(), "Library", "Application Support", "memclaw", "data");
    }
    else {
        return path.join(os.homedir(), ".local", "share", "memclaw", "data");
    }
}
function getConfigPath() {
    return path.join(getConfigDir(), "config.toml");
}
function generateConfigTemplate() {
    const dataDir = getDataDir().replace(/\\/g, "/");
    return `# MemClaw Configuration
#
# This file was auto-generated. Please fill in the required values below.
# Required fields are marked with [REQUIRED]

# Qdrant Vector Database Configuration
[qdrant]
url = "http://localhost:6334"
collection_name = "memclaw"
timeout_secs = 30

# LLM Configuration [REQUIRED for memory processing]
[llm]
# Your LLM API endpoint (OpenAI-compatible)
api_base_url = "https://api.openai.com/v1"
# Your API key [REQUIRED]
api_key = ""
# Model for memory extraction and layer generation
model_efficient = "gpt-4o-mini"
temperature = 0.1
max_tokens = 4096

# Embedding Configuration [REQUIRED for vector search]
[embedding]
# Your embedding API endpoint (OpenAI-compatible)
api_base_url = "https://api.openai.com/v1"
# Your API key [REQUIRED - can be same as llm.api_key]
api_key = ""
model_name = "text-embedding-3-small"
batch_size = 10
timeout_secs = 30

# Service Configuration
[server]
host = "localhost"
port = 8085

# Cortex Memory Settings
[cortex]
data_dir = "${dataDir}"
enable_intent_analysis = false
`;
}
function ensureConfigExists() {
    const configDir = getConfigDir();
    const configPath = getConfigPath();
    if (!fs.existsSync(configDir)) {
        fs.mkdirSync(configDir, { recursive: true });
    }
    if (!fs.existsSync(configPath)) {
        const template = generateConfigTemplate();
        fs.writeFileSync(configPath, template, "utf-8");
        return { created: true, path: configPath };
    }
    return { created: false, path: configPath };
}
function openConfigFile(configPath) {
    return new Promise((resolve, reject) => {
        const platform = process.platform;
        let command;
        let args = [];
        if (platform === "win32") {
            command = "cmd";
            args = ["/c", "start", '""', configPath];
        }
        else if (platform === "darwin") {
            command = "open";
            args = [configPath];
        }
        else {
            command = "xdg-open";
            args = [configPath];
        }
        const proc = (0, child_process_1.spawn)(command, args, { detached: true, stdio: "ignore" });
        proc.on("error", (err) => {
            reject(err);
        });
        proc.unref();
        resolve();
    });
}
function parseConfig(configPath) {
    const content = fs.readFileSync(configPath, "utf-8");
    const config = {};
    let currentSection = "";
    for (const line of content.split("\n")) {
        const trimmed = line.trim();
        // Skip comments and empty lines
        if (trimmed.startsWith("#") || trimmed === "")
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
            if (value === "true")
                value = true;
            else if (value === "false")
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
    const dataDir = getDataDir();
    return {
        qdrant: {
            url: "http://localhost:6334",
            collection_name: "memclaw",
            timeout_secs: 30,
            ...(config.qdrant || {}),
        },
        llm: {
            api_base_url: "https://api.openai.com/v1",
            api_key: "",
            model_efficient: "gpt-5-mini",
            temperature: 0.1,
            max_tokens: 4096,
            ...(config.llm || {}),
        },
        embedding: {
            api_base_url: "https://api.openai.com/v1",
            api_key: "",
            model_name: "text-embedding-3-small",
            batch_size: 10,
            timeout_secs: 30,
            ...(config.embedding || {}),
        },
        server: {
            host: "localhost",
            port: 8085,
            ...(config.server || {}),
        },
        cortex: {
            data_dir: dataDir,
            enable_intent_analysis: false,
            ...(config.cortex || {}),
        },
    };
}
function validateConfig(config) {
    const errors = [];
    if (!config.llm.api_key || config.llm.api_key === "") {
        errors.push("llm.api_key is required");
    }
    if (!config.embedding.api_key || config.embedding.api_key === "") {
        // Allow using llm.api_key for embedding if not specified
        if (config.llm.api_key && config.llm.api_key !== "") {
            config.embedding.api_key = config.llm.api_key;
        }
        else {
            errors.push("embedding.api_key is required");
        }
    }
    return {
        valid: errors.length === 0,
        errors,
    };
}
//# sourceMappingURL=config.js.map