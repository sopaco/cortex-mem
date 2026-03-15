/**
 * Binary management for MemClaw
 *
 * Binaries are bundled in platform-specific npm packages:
 * - @memclaw/bin-darwin-arm64 (macOS Apple Silicon)
 * - @memclaw/bin-win-x64 (Windows x64)
 *
 * The correct package is installed automatically via optionalDependencies.
 */
type SupportedPlatform = "darwin-arm64" | "win-x64";
export declare function getPlatform(): SupportedPlatform | null;
export declare function isPlatformSupported(): boolean;
export declare function getUnsupportedPlatformMessage(): string;
export declare function getBinaryPath(binary: string): string | null;
export declare function isBinaryAvailable(binary: string): boolean;
export declare function isPlatformPackageInstalled(): boolean;
export declare function getInstallInstructions(): string;
export interface ServiceStatus {
    qdrant: boolean;
    cortexMemService: boolean;
}
export declare function checkServiceStatus(): Promise<ServiceStatus>;
export declare function startQdrant(log?: (msg: string) => void): Promise<void>;
export declare function startCortexMemService(log?: (msg: string) => void): Promise<void>;
export declare function stopAllServices(): void;
export declare function ensureAllServices(log?: (msg: string) => void): Promise<ServiceStatus>;
export declare function getCliPath(): string | null;
export {};
//# sourceMappingURL=binaries.d.ts.map