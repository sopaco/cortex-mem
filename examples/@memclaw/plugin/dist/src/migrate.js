"use strict";
/**
 * Migration script for OpenClaw native memory to MemClaw
 *
 * Migrates:
 * - memory/YYYY-MM-DD.md → session timeline files
 * - MEMORY.md → users/{tenant}/preferences.md
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
exports.migrateFromOpenClaw = migrateFromOpenClaw;
exports.canMigrate = canMigrate;
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const os = __importStar(require("os"));
const child_process_1 = require("child_process");
const util_1 = require("util");
const glob_1 = require("glob");
const config_js_1 = require("./config.js");
const binaries_js_1 = require("./binaries.js");
const execAsync = (0, util_1.promisify)(child_process_1.exec);
// Migration configuration
const MIGRATION_TENANT = 'tenant_claw';
const DEFAULT_ROLE = 'user';
/**
 * Detect OpenClaw workspace directory
 */
function findOpenClawWorkspace() {
    const homeDir = os.homedir();
    const workspacePath = path.join(homeDir, '.openclaw', 'workspace');
    if (fs.existsSync(workspacePath)) {
        return workspacePath;
    }
    return null;
}
/**
 * Split content into paragraphs (not just lines)
 */
function splitIntoParagraphs(content) {
    return content
        .split(/\n\s*\n/)
        .map(p => p.trim())
        .filter(p => p.length > 0 && !p.startsWith('#')); // Skip headers
}
/**
 * Generate L2 content for a single paragraph
 */
function generateL2Content(paragraph, sessionId, date, index) {
    // Generate a deterministic timestamp based on index
    const hour = String(index % 24).padStart(2, '0');
    const minute = String((index * 7) % 60).padStart(2, '0');
    const second = String((index * 13) % 60).padStart(2, '0');
    const timestamp = `${date}T${hour}:${minute}:${second}Z`;
    const msgId = `${date.replace(/-/g, '')}_${String(index).padStart(4, '0')}`;
    return `---
id: "${msgId}"
role: "${DEFAULT_ROLE}"
timestamp: "${timestamp}"
thread_id: "${sessionId}"
---
${paragraph}`;
}
/**
 * Migrate daily logs from OpenClaw native memory to MemClaw
 */
async function migrateDailyLogs(ocWorkspace, dataDir, log) {
    const memoryDir = path.join(ocWorkspace, 'memory');
    const count = 0;
    const sessions = [];
    const errors = [];
    if (!fs.existsSync(memoryDir)) {
        log?.('No memory directory found in OpenClaw workspace');
        return { count: 0, sessions: [], errors: [] };
    }
    // Find all daily log files
    const dailyLogPattern = path.join(memoryDir, '*.md').replace(/\\/g, '/');
    const files = await (0, glob_1.glob)(dailyLogPattern);
    const dailyLogs = files.filter(f => {
        const basename = path.basename(f);
        return /^\d{4}-\d{2}-\d{2}\.md$/.test(basename);
    });
    log?.(`Found ${dailyLogs.length} daily log files to migrate`);
    for (const logPath of dailyLogs) {
        try {
            const date = path.basename(logPath, '.md'); // 2026-03-13
            const [year, month, day] = date.split('-');
            const sessionId = `migrated-oc-${date}`;
            const timelineDir = path.join(dataDir, 'sessions', MIGRATION_TENANT, sessionId, 'timeline', year, month, day);
            // Create directory
            fs.mkdirSync(timelineDir, { recursive: true });
            // Read and split content
            const content = fs.readFileSync(logPath, 'utf-8');
            const paragraphs = splitIntoParagraphs(content);
            // Write each paragraph as L2 file
            for (let i = 0; i < paragraphs.length; i++) {
                const para = paragraphs[i];
                const hour = String(i % 24).padStart(2, '0');
                const minute = String((i * 7) % 60).padStart(2, '0');
                const second = String((i * 13) % 60).padStart(2, '0');
                const msgId = `${date.replace(/-/g, '')}_${String(i).padStart(4, '0')}`;
                const l2Content = generateL2Content(para, sessionId, date, i);
                const filename = `${hour}_${minute}_${second}_${msgId}.md`;
                fs.writeFileSync(path.join(timelineDir, filename), l2Content, 'utf-8');
            }
            sessions.push(sessionId);
            log?.(`Migrated ${date}: ${paragraphs.length} messages`);
        }
        catch (err) {
            const errorMsg = `Failed to migrate ${logPath}: ${err}`;
            errors.push(errorMsg);
            log?.(`Error: ${errorMsg}`);
        }
    }
    return {
        count: dailyLogs.length,
        sessions,
        errors,
    };
}
/**
 * Migrate MEMORY.md to user preferences
 */
async function migrateMemoryMd(ocWorkspace, dataDir, log) {
    const memoryMdPath = path.join(ocWorkspace, 'MEMORY.md');
    if (!fs.existsSync(memoryMdPath)) {
        log?.('No MEMORY.md found');
        return { migrated: false };
    }
    try {
        const userDir = path.join(dataDir, 'users', MIGRATION_TENANT);
        fs.mkdirSync(userDir, { recursive: true });
        const content = fs.readFileSync(memoryMdPath, 'utf-8');
        const targetPath = path.join(userDir, 'preferences.md');
        // Add header to indicate migration source
        const migratedContent = `<!--
Migrated from OpenClaw native MEMORY.md
Original path: ${memoryMdPath}
Migration date: ${new Date().toISOString()}
-->
${content}`;
        fs.writeFileSync(targetPath, migratedContent, 'utf-8');
        log?.('Migrated MEMORY.md to user preferences');
        return { migrated: true };
    }
    catch (err) {
        const error = `Failed to migrate MEMORY.md: ${err}`;
        log?.(`Error: ${error}`);
        return { migrated: false, error };
    }
}
/**
 * Generate L0/L1 layers using cortex-mem-cli
 */
async function generateLayers(configPath, tenant, log) {
    log?.('Generating L0/L1 layers...');
    const cliPath = (0, binaries_js_1.getCliPath)();
    if (!cliPath) {
        log?.('cortex-mem-cli not found, skipping layer generation');
        return;
    }
    try {
        const { stdout, stderr } = await execAsync(`"${cliPath}" --config "${configPath}" --tenant ${tenant} layers ensure-all`, { timeout: 300000 } // 5 minutes
        );
        if (stdout)
            log?.(stdout);
        if (stderr)
            log?.(stderr);
        log?.('Layer generation completed');
    }
    catch (err) {
        log?.(`Layer generation warning: ${err}`);
        // Don't throw - this is not critical for migration
    }
}
/**
 * Generate vector index using cortex-mem-cli
 */
async function generateVectorIndex(configPath, tenant, log) {
    log?.('Generating vector index...');
    const cliPath = (0, binaries_js_1.getCliPath)();
    if (!cliPath) {
        log?.('cortex-mem-cli not found, skipping vector index generation');
        return;
    }
    try {
        const { stdout, stderr } = await execAsync(`"${cliPath}" --config "${configPath}" --tenant ${tenant} vector reindex`, { timeout: 600000 } // 10 minutes
        );
        if (stdout)
            log?.(stdout);
        if (stderr)
            log?.(stderr);
        log?.('Vector index generation completed');
    }
    catch (err) {
        log?.(`Vector index warning: ${err}`);
        // Don't throw - this is not critical for migration
    }
}
/**
 * Main migration function
 */
async function migrateFromOpenClaw(log) {
    const result = {
        dailyLogsMigrated: 0,
        memoryMdMigrated: false,
        sessionsCreated: [],
        errors: [],
    };
    log?.('Starting OpenClaw memory migration...');
    // Find OpenClaw workspace
    const ocWorkspace = findOpenClawWorkspace();
    if (!ocWorkspace) {
        const error = 'OpenClaw workspace not found at ~/.openclaw/workspace';
        result.errors.push(error);
        log?.(error);
        return result;
    }
    log?.(`Found OpenClaw workspace: ${ocWorkspace}`);
    const dataDir = (0, config_js_1.getDataDir)();
    const configPath = (0, config_js_1.getConfigPath)();
    // Migrate daily logs
    const dailyResult = await migrateDailyLogs(ocWorkspace, dataDir, log);
    result.dailyLogsMigrated = dailyResult.count;
    result.sessionsCreated = dailyResult.sessions;
    result.errors.push(...dailyResult.errors);
    // Migrate MEMORY.md
    const memoryMdResult = await migrateMemoryMd(ocWorkspace, dataDir, log);
    result.memoryMdMigrated = memoryMdResult.migrated;
    if (memoryMdResult.error) {
        result.errors.push(memoryMdResult.error);
    }
    // Generate layers and index
    if (result.dailyLogsMigrated > 0 || result.memoryMdMigrated) {
        await generateLayers(configPath, MIGRATION_TENANT, log);
        await generateVectorIndex(configPath, MIGRATION_TENANT, log);
    }
    log?.(`Migration completed: ${result.dailyLogsMigrated} daily logs, MEMORY.md: ${result.memoryMdMigrated}`);
    return result;
}
/**
 * Check if migration is possible
 */
function canMigrate() {
    const ocWorkspace = findOpenClawWorkspace();
    if (!ocWorkspace) {
        return { possible: false, reason: 'OpenClaw workspace not found' };
    }
    const memoryDir = path.join(ocWorkspace, 'memory');
    const memoryMd = path.join(ocWorkspace, 'MEMORY.md');
    if (!fs.existsSync(memoryDir) && !fs.existsSync(memoryMd)) {
        return { possible: false, reason: 'No memory files found in OpenClaw workspace' };
    }
    return { possible: true, reason: 'OpenClaw memory found and ready for migration' };
}
//# sourceMappingURL=migrate.js.map