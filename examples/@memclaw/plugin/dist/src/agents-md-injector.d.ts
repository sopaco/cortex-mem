/**
 * AGENTS.md Auto-Enhancement for MemClaw
 *
 * Automatically detects and enhances OpenClaw's AGENTS.md with MemClaw
 * memory usage guidelines when legacy memory patterns are found.
 */
export interface InjectionResult {
    injected: boolean;
    reason: 'success' | 'already_injected' | 'no_agents_md' | 'no_legacy_patterns' | 'disabled' | 'error';
    path?: string;
    error?: string;
}
export interface PluginLogger {
    debug?: (msg: string, ...args: unknown[]) => void;
    info: (msg: string, ...args: unknown[]) => void;
    warn: (msg: string, ...args: unknown[]) => void;
    error: (msg: string, ...args: unknown[]) => void;
}
/**
 * Get OpenClaw home directory
 */
export declare function getOpenClawHome(): string;
/**
 * Find OpenClaw workspace path
 * Priority: ENV > openclaw.json config > default
 */
export declare function findOpenClawWorkspace(): string | null;
/**
 * Find AGENTS.md file in workspace
 */
export declare function findAgentsMd(workspacePath: string): string | null;
/**
 * Check if content already has MemClaw injection
 */
export declare function hasMemClawInjection(content: string): boolean;
/**
 * Check if content has legacy memory patterns
 */
export declare function hasLegacyPatterns(content: string): boolean;
/**
 * Inject MemClaw section into AGENTS.md content
 */
export declare function injectMemClawSection(content: string): string;
/**
 * Main entry point: Ensure AGENTS.md is enhanced with MemClaw
 */
export declare function ensureAgentsMdEnhanced(logger: PluginLogger, enabled?: boolean): InjectionResult;
//# sourceMappingURL=agents-md-injector.d.ts.map