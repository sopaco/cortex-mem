/**
 * Configuration management for MemClaw
 *
 * Handles platform-specific config paths, config file generation,
 * and auto-opening config files for user editing.
 */

import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { spawn } from "child_process";

// Platform-specific paths
export function getConfigDir(): string {
  const platform = process.platform;

  if (platform === "win32") {
    return path.join(
      process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming"),
      "memclaw",
    );
  } else if (platform === "darwin") {
    return path.join(os.homedir(), "Library", "Application Support", "memclaw");
  } else {
    return path.join(os.homedir(), ".config", "memclaw");
  }
}

export function getDataDir(): string {
  const platform = process.platform;

  if (platform === "win32") {
    return path.join(
      process.env.LOCALAPPDATA || path.join(os.homedir(), "AppData", "Local"),
      "memclaw",
      "data",
    );
  } else if (platform === "darwin") {
    return path.join(
      os.homedir(),
      "Library",
      "Application Support",
      "memclaw",
      "data",
    );
  } else {
    return path.join(os.homedir(), ".local", "share", "memclaw", "data");
  }
}

export function getConfigPath(): string {
  return path.join(getConfigDir(), "config.toml");
}

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
  cortex: {
    data_dir: string;
    enable_intent_analysis: boolean;
  };
}

export function generateConfigTemplate(): string {
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

export function ensureConfigExists(): { created: boolean; path: string } {
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

export function openConfigFile(configPath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const platform = process.platform;
    let command: string;
    let args: string[] = [];

    if (platform === "win32") {
      command = "cmd";
      args = ["/c", "start", '""', configPath];
    } else if (platform === "darwin") {
      command = "open";
      args = [configPath];
    } else {
      command = "xdg-open";
      args = [configPath];
    }

    const proc = spawn(command, args, { detached: true, stdio: "ignore" });
    proc.on("error", (err) => {
      reject(err);
    });
    proc.unref();
    resolve();
  });
}

export function parseConfig(configPath: string): MemClawConfig {
  const content = fs.readFileSync(configPath, "utf-8");
  const config: Partial<MemClawConfig> = {};

  let currentSection = "";

  for (const line of content.split("\n")) {
    const trimmed = line.trim();

    // Skip comments and empty lines
    if (trimmed.startsWith("#") || trimmed === "") continue;

    // Section header
    const sectionMatch = trimmed.match(/^\[(\w+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      config[currentSection as keyof MemClawConfig] = {} as any;
      continue;
    }

    // Key-value pair
    const kvMatch =
      trimmed.match(/^(\w+)\s*=\s*"([^"]*)"(?:\s*$|\s*#)/) ||
      trimmed.match(/^(\w+)\s*=\s*(\d+(?:\.\d+)?)(?:\s*$|\s*#)/) ||
      trimmed.match(/^(\w+)\s*=\s*(true|false)(?:\s*$|\s*#)/);

    if (kvMatch && currentSection) {
      const key = kvMatch[1];
      let value: string | number | boolean = kvMatch[2];

      // Convert to appropriate type
      if (value === "true") value = true;
      else if (value === "false") value = false;
      else if (/^\d+$/.test(value)) value = parseInt(value, 10);
      else if (/^\d+\.\d+$/.test(value)) value = parseFloat(value);

      (config as any)[currentSection] = (config as any)[currentSection] || {};
      (config as any)[currentSection][key] = value;
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

export function validateConfig(config: MemClawConfig): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  if (!config.llm.api_key || config.llm.api_key === "") {
    errors.push("llm.api_key is required");
  }

  if (!config.embedding.api_key || config.embedding.api_key === "") {
    // Allow using llm.api_key for embedding if not specified
    if (config.llm.api_key && config.llm.api_key !== "") {
      config.embedding.api_key = config.llm.api_key;
    } else {
      errors.push("embedding.api_key is required");
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}
