/**
 * Migration script for OpenClaw native memory to MemClaw
 *
 * Migrates:
 * - memory/YYYY-MM-DD.md → session timeline files
 * - MEMORY.md → users/{tenant}/preferences.md
 */
interface MigrationResult {
    dailyLogsMigrated: number;
    memoryMdMigrated: boolean;
    sessionsCreated: string[];
    errors: string[];
}
/**
 * Main migration function
 */
export declare function migrateFromOpenClaw(log?: (msg: string) => void): Promise<MigrationResult>;
/**
 * Check if migration is possible
 */
export declare function canMigrate(): {
    possible: boolean;
    reason: string;
};
export {};
//# sourceMappingURL=migrate.d.ts.map