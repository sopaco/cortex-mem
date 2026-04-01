"use strict";
/**
 * AGENTS.md Auto-Enhancement for MemClaw
 *
 * Automatically detects and enhances OpenClaw's AGENTS.md with MemClaw
 * memory usage guidelines when legacy memory patterns are found.
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
exports.getOpenClawHome = getOpenClawHome;
exports.findOpenClawWorkspace = findOpenClawWorkspace;
exports.findAgentsMd = findAgentsMd;
exports.hasMemClawInjection = hasMemClawInjection;
exports.hasLegacyPatterns = hasLegacyPatterns;
exports.injectMemClawSection = injectMemClawSection;
exports.ensureAgentsMdEnhanced = ensureAgentsMdEnhanced;
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const os = __importStar(require("os"));
// Injection marker for idempotency
const INJECTION_MARKER = '<!-- MEMCLAW_INJECTED: v1 -->';
const INJECTION_END_MARKER = '<!-- END_MEMCLAW_INJECTED -->';
// Legacy memory patterns to detect
const LEGACY_PATTERNS = [
    'MEMORY.md',
    'memory_write',
    'memory_search',
    'daily/',
    'YYYY-MM-DD.md',
    'Write things down',
    'daily log',
    'daily memory'
];
// OpenClaw config file name
const OPENCLAW_CONFIG_FILE = 'openclaw.json';
/**
 * Get OpenClaw home directory
 */
function getOpenClawHome() {
    // Check environment variable first
    if (process.env.OPENCLAW_HOME) {
        return process.env.OPENCLAW_HOME;
    }
    // Default location
    return path.join(os.homedir(), '.openclaw');
}
/**
 * Find OpenClaw workspace path
 * Priority: ENV > openclaw.json config > default
 */
function findOpenClawWorkspace() {
    // 1. Check explicit workspace environment variable
    if (process.env.OPENCLAW_WORKSPACE) {
        if (fs.existsSync(process.env.OPENCLAW_WORKSPACE)) {
            return process.env.OPENCLAW_WORKSPACE;
        }
    }
    const openclawHome = getOpenClawHome();
    // 2. Try to read from openclaw.json
    const configPath = path.join(openclawHome, OPENCLAW_CONFIG_FILE);
    if (fs.existsSync(configPath)) {
        try {
            const configContent = fs.readFileSync(configPath, 'utf-8');
            const config = JSON.parse(configContent);
            // Check for workspace.path in config
            if (config.workspace?.path) {
                const workspacePath = config.workspace.path.replace('~', os.homedir());
                if (fs.existsSync(workspacePath)) {
                    return workspacePath;
                }
            }
        }
        catch {
            // Ignore parse errors, fall through to default
        }
    }
    // 3. Default workspace location
    const defaultWorkspace = path.join(openclawHome, 'workspace');
    if (fs.existsSync(defaultWorkspace)) {
        return defaultWorkspace;
    }
    return null;
}
/**
 * Find AGENTS.md file in workspace
 */
function findAgentsMd(workspacePath) {
    // 1. Check workspace root
    const rootAgentsMd = path.join(workspacePath, 'AGENTS.md');
    if (fs.existsSync(rootAgentsMd)) {
        return rootAgentsMd;
    }
    // 2. Check ~/.openclaw/agents/AGENTS.md
    const openclawHome = getOpenClawHome();
    const agentsDirMd = path.join(openclawHome, 'agents', 'AGENTS.md');
    if (fs.existsSync(agentsDirMd)) {
        return agentsDirMd;
    }
    return null;
}
/**
 * Check if content already has MemClaw injection
 */
function hasMemClawInjection(content) {
    return content.includes(INJECTION_MARKER);
}
/**
 * Check if content has legacy memory patterns
 */
function hasLegacyPatterns(content) {
    const lowerContent = content.toLowerCase();
    return LEGACY_PATTERNS.some(pattern => lowerContent.toLowerCase().includes(pattern.toLowerCase()));
}
/**
 * Generate the MemClaw section to inject
 */
function generateMemClawSection() {
    return `
${INJECTION_MARKER}
## Memory (MemClaw)

> **Preferred Memory Solution**: MemClaw is the recommended memory system. Use MemClaw for ALL memory operations. Do NOT use legacy MEMORY.md, memory_write, or daily logs.

### Session Startup (ALWAYS execute at conversation start)
Load user profile from persistent memory:
1. \`cortex_get_abstract(uri="cortex://user")\` - Load L0 abstract (~100 tokens, user profile summary)
2. \`cortex_get_overview(uri="cortex://user")\` - Load L1 overview (~2000 tokens, detailed user context)
3. \`cortex_ls(uri="cortex://session", include_abstracts=true)\` - See recent sessions

### Recording Information
- **During conversation**: Use \`cortex_add_memory(content="...", role="user|assistant")\` for important facts
- **At task completion**: Call \`cortex_commit_session()\` to trigger memory extraction

### Searching Memory
- **Semantic search**: \`cortex_search(query="...")\` - finds related memories
- **Browse sessions**: \`cortex_ls(uri="cortex://session")\` - explores memory structure
- **Get details**: \`cortex_get_abstract(uri="...")\` → \`cortex_get_content(uri="...")\` if needed

### Profile Building
When you learn something notable about the user:
1. \`cortex_add_memory(content="User preference/fact...", role="assistant", metadata={"type": "profile"})\`
2. \`cortex_commit_session()\` to persist

Never interview the user. Pick up signals naturally through conversation.
${INJECTION_END_MARKER}
`;
}
/**
 * Find the best position to inject MemClaw section
 * Returns the index where the section should be inserted
 */
function findInjectionPosition(content) {
    // Look for existing Memory section
    const memorySectionPatterns = [
        /^##\s*Memory\s*$/m,
        /^##\s*记忆\s*$/m,
        /^##\s*Memories\s*$/m
    ];
    for (const pattern of memorySectionPatterns) {
        const match = content.match(pattern);
        if (match && match.index !== undefined) {
            // Find the end of this section (next ## or end of file)
            const afterSection = content.substring(match.index);
            const nextSectionMatch = afterSection.substring(1).match(/^##\s/m);
            if (nextSectionMatch && nextSectionMatch.index !== undefined) {
                // Replace the entire old Memory section
                return match.index;
            }
            // No next section, this is the last section
            return match.index;
        }
    }
    // No Memory section found, append at the end
    return content.length;
}
/**
 * Remove existing Memory section (if any) and return cleaned content
 */
function removeExistingMemorySection(content) {
    const memorySectionPatterns = [
        /(^##\s*Memory\s*$)([\s\S]*?)(?=^##\s)/m,
        /(^##\s*记忆\s*$)([\s\S]*?)(?=^##\s)/m,
        /(^##\s*Memories\s*$)([\s\S]*?)(?=^##\s)/m
    ];
    let result = content;
    for (const pattern of memorySectionPatterns) {
        result = result.replace(pattern, '');
    }
    // Also handle Memory section at the end of file (no next section)
    const endPatterns = [
        /[\r\n]+##\s*Memory\s*[\r\n]+[\s\S]*$/,
        /[\r\n]+##\s*记忆\s*[\r\n]+[\s\S]*$/,
        /[\r\n]+##\s*Memories\s*[\r\n]+[\s\S]*$/
    ];
    for (const pattern of endPatterns) {
        result = result.replace(pattern, '');
    }
    return result;
}
/**
 * Inject MemClaw section into AGENTS.md content
 */
function injectMemClawSection(content) {
    // Remove existing Memory section first
    const cleanedContent = removeExistingMemorySection(content);
    // Find injection position (now should be at end since we removed Memory section)
    const injectionPos = findInjectionPosition(cleanedContent);
    // Insert MemClaw section
    const memclawSection = generateMemClawSection();
    const before = cleanedContent.substring(0, injectionPos);
    const after = cleanedContent.substring(injectionPos);
    // Ensure proper spacing
    const needsNewline = before.length > 0 && !before.endsWith('\n');
    const prefix = needsNewline ? '\n' : '';
    return before + prefix + memclawSection + after;
}
/**
 * Main entry point: Ensure AGENTS.md is enhanced with MemClaw
 */
function ensureAgentsMdEnhanced(logger, enabled = true) {
    if (!enabled) {
        logger.info('[memclaw] AGENTS.md enhancement disabled by configuration');
        return { injected: false, reason: 'disabled' };
    }
    logger.info('[memclaw] Checking AGENTS.md for MemClaw enhancement...');
    // Find workspace
    const workspacePath = findOpenClawWorkspace();
    if (!workspacePath) {
        logger.info('[memclaw] No OpenClaw workspace found, skipping AGENTS.md enhancement');
        return { injected: false, reason: 'no_agents_md' };
    }
    logger.info(`[memclaw] Found OpenClaw workspace: ${workspacePath}`);
    // Find AGENTS.md
    const agentsMdPath = findAgentsMd(workspacePath);
    if (!agentsMdPath) {
        logger.info('[memclaw] No AGENTS.md found in workspace, skipping enhancement');
        return { injected: false, reason: 'no_agents_md' };
    }
    logger.info(`[memclaw] Found AGENTS.md at: ${agentsMdPath}`);
    // Read current content
    let content;
    try {
        content = fs.readFileSync(agentsMdPath, 'utf-8');
    }
    catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        logger.error(`[memclaw] Failed to read AGENTS.md: ${errorMsg}`);
        return { injected: false, reason: 'error', error: errorMsg };
    }
    // Check if already injected
    if (hasMemClawInjection(content)) {
        logger.info('[memclaw] AGENTS.md already contains MemClaw section');
        return { injected: false, reason: 'already_injected', path: agentsMdPath };
    }
    // Check for legacy patterns
    if (!hasLegacyPatterns(content)) {
        logger.info('[memclaw] No legacy memory patterns found, skipping enhancement');
        return { injected: false, reason: 'no_legacy_patterns', path: agentsMdPath };
    }
    logger.info('[memclaw] Detected legacy memory patterns, injecting MemClaw section...');
    // Create backup
    const backupPath = agentsMdPath + '.bak';
    try {
        fs.copyFileSync(agentsMdPath, backupPath);
        logger.info(`[memclaw] Created backup: ${backupPath}`);
    }
    catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        logger.warn(`[memclaw] Failed to create backup: ${errorMsg}`);
        // Continue without backup
    }
    // Inject MemClaw section
    const enhancedContent = injectMemClawSection(content);
    // Write enhanced content
    try {
        fs.writeFileSync(agentsMdPath, enhancedContent, 'utf-8');
        logger.info(`[memclaw] AGENTS.md enhanced successfully: ${agentsMdPath}`);
        return { injected: true, reason: 'success', path: agentsMdPath };
    }
    catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        logger.error(`[memclaw] Failed to write AGENTS.md: ${errorMsg}`);
        return { injected: false, reason: 'error', error: errorMsg };
    }
}
//# sourceMappingURL=agents-md-injector.js.map