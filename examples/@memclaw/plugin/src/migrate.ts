/**
 * Migration script for OpenClaw native memory to MemClaw
 * 
 * Migrates:
 * - memory/YYYY-MM-DD.md → session timeline files
 * - MEMORY.md → users/{tenant}/preferences.md
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { exec } from 'child_process';
import { promisify } from 'util';
import { glob } from 'glob';
import { getDataDir, getConfigPath } from './config.js';
import { getCliPath } from './binaries.js';

const execAsync = promisify(exec);

// Migration configuration
const MIGRATION_TENANT = 'tenant_claw';
const DEFAULT_ROLE = 'user';

interface MigrationResult {
  dailyLogsMigrated: number;
  memoryMdMigrated: boolean;
  sessionsCreated: string[];
  errors: string[];
}

/**
 * Detect OpenClaw workspace directory
 */
function findOpenClawWorkspace(): string | null {
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
function splitIntoParagraphs(content: string): string[] {
  return content
    .split(/\n\s*\n/)
    .map(p => p.trim())
    .filter(p => p.length > 0 && !p.startsWith('#')); // Skip headers
}

/**
 * Generate L2 content for a single paragraph
 */
function generateL2Content(
  paragraph: string,
  sessionId: string,
  date: string,
  index: number
): string {
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
async function migrateDailyLogs(
  ocWorkspace: string,
  dataDir: string,
  log?: (msg: string) => void
): Promise<{ count: number; sessions: string[]; errors: string[] }> {
  const memoryDir = path.join(ocWorkspace, 'memory');
  const count = 0;
  const sessions: string[] = [];
  const errors: string[] = [];
  
  if (!fs.existsSync(memoryDir)) {
    log?.('No memory directory found in OpenClaw workspace');
    return { count: 0, sessions: [], errors: [] };
  }
  
  // Find all daily log files
  const dailyLogPattern = path.join(memoryDir, '*.md').replace(/\\/g, '/');
  const files = await glob(dailyLogPattern);
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
      
      const timelineDir = path.join(
        dataDir,
        'sessions',
        MIGRATION_TENANT,
        sessionId,
        'timeline',
        year,
        month,
        day
      );
      
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
      
    } catch (err) {
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
async function migrateMemoryMd(
  ocWorkspace: string,
  dataDir: string,
  log?: (msg: string) => void
): Promise<{ migrated: boolean; error?: string }> {
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
  } catch (err) {
    const error = `Failed to migrate MEMORY.md: ${err}`;
    log?.(`Error: ${error}`);
    return { migrated: false, error };
  }
}

/**
 * Generate L0/L1 layers using cortex-mem-cli
 */
async function generateLayers(
  configPath: string,
  tenant: string,
  log?: (msg: string) => void
): Promise<void> {
  log?.('Generating L0/L1 layers...');
  
  const cliPath = getCliPath();
  if (!cliPath) {
    log?.('cortex-mem-cli not found, skipping layer generation');
    return;
  }
  
  try {
    const { stdout, stderr } = await execAsync(
      `"${cliPath}" --config "${configPath}" --tenant ${tenant} layers ensure-all`,
      { timeout: 300000 } // 5 minutes
    );
    
    if (stdout) log?.(stdout);
    if (stderr) log?.(stderr);
    
    log?.('Layer generation completed');
  } catch (err) {
    log?.(`Layer generation warning: ${err}`);
    // Don't throw - this is not critical for migration
  }
}

/**
 * Generate vector index using cortex-mem-cli
 */
async function generateVectorIndex(
  configPath: string,
  tenant: string,
  log?: (msg: string) => void
): Promise<void> {
  log?.('Generating vector index...');
  
  const cliPath = getCliPath();
  if (!cliPath) {
    log?.('cortex-mem-cli not found, skipping vector index generation');
    return;
  }
  
  try {
    const { stdout, stderr } = await execAsync(
      `"${cliPath}" --config "${configPath}" --tenant ${tenant} vector reindex`,
      { timeout: 600000 } // 10 minutes
    );
    
    if (stdout) log?.(stdout);
    if (stderr) log?.(stderr);
    
    log?.('Vector index generation completed');
  } catch (err) {
    log?.(`Vector index warning: ${err}`);
    // Don't throw - this is not critical for migration
  }
}

/**
 * Main migration function
 */
export async function migrateFromOpenClaw(
  log?: (msg: string) => void
): Promise<MigrationResult> {
  const result: MigrationResult = {
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
  
  const dataDir = getDataDir();
  const configPath = getConfigPath();
  
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
export function canMigrate(): { possible: boolean; reason: string } {
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
