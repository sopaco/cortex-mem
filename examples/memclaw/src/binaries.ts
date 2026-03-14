/**
 * Binary management for MemClaw
 * 
 * Binaries are bundled in platform-specific npm packages:
 * - @memclaw/bin-darwin-arm64 (macOS Apple Silicon)
 * - @memclaw/bin-win-x64 (Windows x64)
 * 
 * The correct package is installed automatically via optionalDependencies.
 */

import * as fs from "fs";
import * as path from "path";
import { spawn, ChildProcess } from "child_process";
import { getDataDir, getConfigPath } from "./config.js";

// Binary names type
type BinaryName = "qdrant" | "cortex-mem-service" | "cortex-mem-cli";

// Supported platforms
type SupportedPlatform = "darwin-arm64" | "win-x64";

// Platform detection
export function getPlatform(): SupportedPlatform | null {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "darwin" && arch === "arm64") {
    return "darwin-arm64";
  } else if (platform === "win32" && arch === "x64") {
    return "win-x64";
  }
  
  return null;
}

// Check if current platform is supported
export function isPlatformSupported(): boolean {
  return getPlatform() !== null;
}

// Get unsupported platform message
export function getUnsupportedPlatformMessage(): string {
  const platform = process.platform;
  const arch = process.arch;
  
  return `
MemClaw is only supported on:
  - macOS Apple Silicon (darwin-arm64)
  - Windows x64 (win-x64)

Current platform: ${platform}-${arch} is not supported.
`;
}

// Get binary name with platform extension
function getBinaryFileName(binary: BinaryName): string {
  return process.platform === "win32" ? `${binary}.exe` : binary;
}

// Get the path to the platform-specific npm package
function getPlatformPackagePath(): string | null {
  const platform = getPlatform();
  if (!platform) {
    return null;
  }
  
  const packageName = `@memclaw/bin-${platform}`;

  try {
    // Try to resolve the package path
    const packageJsonPath = require.resolve(`${packageName}/package.json`);
    return path.dirname(packageJsonPath);
  } catch {
    return null;
  }
}

// Get binary path from npm package
export function getBinaryPath(binary: string): string | null {
  const packagePath = getPlatformPackagePath();

  if (!packagePath) {
    return null;
  }

  const binaryFileName = getBinaryFileName(binary as BinaryName);
  const binaryPath = path.join(packagePath, "bin", binaryFileName);

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  return null;
}

// Check if binary is available
export function isBinaryAvailable(binary: string): boolean {
  return getBinaryPath(binary as BinaryName) !== null;
}

// Check if platform package is installed
export function isPlatformPackageInstalled(): boolean {
  return getPlatformPackagePath() !== null;
}

// Get installation instructions for missing platform package
export function getInstallInstructions(): string {
  const platform = getPlatform();
  
  if (!platform) {
    return getUnsupportedPlatformMessage();
  }
  
  const packageName = `@memclaw/bin-${platform}`;
  
  return `
Platform binaries not found for ${platform}.

Try running: npm install ${packageName}

Or reinstall memclaw: npm install memclaw
`;
}

export interface ServiceStatus {
  qdrant: boolean;
  cortexMemService: boolean;
}

export async function checkServiceStatus(): Promise<ServiceStatus> {
  const qdrant = await isPortOpen(6333);
  const cortexMemService = await isPortOpen(8085);

  return { qdrant, cortexMemService };
}

async function isPortOpen(port: number): Promise<boolean> {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/health`, {
      method: "GET",
      signal: AbortSignal.timeout(2000),
    });
    return response.ok;
  } catch {
    // Try alternate endpoints for Qdrant
    try {
      if (port === 6333 || port === 6334) {
        const response = await fetch(`http://127.0.0.1:${port}`, {
          method: "GET",
          signal: AbortSignal.timeout(2000),
        });
        return response.ok;
      }
    } catch {}
    return false;
  }
}

// Running processes
const runningProcesses: Map<string, ChildProcess> = new Map();

export async function startQdrant(log?: (msg: string) => void): Promise<void> {
  const status = await checkServiceStatus();
  if (status.qdrant) {
    log?.("Qdrant is already running");
    return;
  }

  const binaryPath = getBinaryPath("qdrant");
  if (!binaryPath) {
    throw new Error(
      `Qdrant binary not found. ${getInstallInstructions()}`
    );
  }

  const dataDir = getDataDir();
  const storagePath = path.join(dataDir, "qdrant-storage");

  if (!fs.existsSync(storagePath)) {
    fs.mkdirSync(storagePath, { recursive: true });
  }

  log?.(`Starting Qdrant with storage at ${storagePath}...`);

  const proc = spawn(
    binaryPath,
    [
      "--storage-path",
      storagePath,
      "--http-port",
      "6333",
      "--grpc-port",
      "6334",
    ],
    {
      stdio: ["ignore", "pipe", "pipe"],
      detached: true,
    },
  );

  proc.on("error", (err) => {
    log?.(`Qdrant error: ${err.message}`);
  });

  proc.unref();
  runningProcesses.set("qdrant", proc);

  // Wait for Qdrant to start
  let retries = 30;
  while (retries > 0) {
    const status = await checkServiceStatus();
    if (status.qdrant) {
      log?.("Qdrant started successfully");
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
    retries--;
  }

  throw new Error("Qdrant failed to start within 15 seconds");
}

export async function startCortexMemService(
  log?: (msg: string) => void,
): Promise<void> {
  const status = await checkServiceStatus();
  if (status.cortexMemService) {
    log?.("cortex-mem-service is already running");
    return;
  }

  const binaryPath = getBinaryPath("cortex-mem-service");
  if (!binaryPath) {
    throw new Error(
      `cortex-mem-service binary not found. ${getInstallInstructions()}`
    );
  }

  const configPath = getConfigPath();
  const dataDir = getDataDir();

  log?.(`Starting cortex-mem-service with config ${configPath}...`);

  const proc = spawn(
    binaryPath,
    ["--config", configPath, "--data-dir", dataDir],
    {
      stdio: ["ignore", "pipe", "pipe"],
      detached: true,
    },
  );

  proc.on("error", (err) => {
    log?.(`cortex-mem-service error: ${err.message}`);
  });

  proc.unref();
  runningProcesses.set("cortex-mem-service", proc);

  // Wait for service to start
  let retries = 30;
  while (retries > 0) {
    const status = await checkServiceStatus();
    if (status.cortexMemService) {
      log?.("cortex-mem-service started successfully");
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
    retries--;
  }

  throw new Error("cortex-mem-service failed to start within 15 seconds");
}

export function stopAllServices(): void {
  for (const [name, proc] of runningProcesses) {
    try {
      proc.kill();
      console.log(`Stopped ${name}`);
    } catch (err) {
      console.error(`Failed to stop ${name}:`, err);
    }
  }
  runningProcesses.clear();
}

export async function ensureAllServices(
  log?: (msg: string) => void,
): Promise<ServiceStatus> {
  // Check if platform is supported
  if (!isPlatformSupported()) {
    log?.(getUnsupportedPlatformMessage());
    return { qdrant: false, cortexMemService: false };
  }

  // Check if platform package is installed
  if (!isPlatformPackageInstalled()) {
    log?.(`Warning: Platform binaries not installed. ${getInstallInstructions()}`);
    return { qdrant: false, cortexMemService: false };
  }

  const status = await checkServiceStatus();

  if (!status.qdrant) {
    await startQdrant(log);
  }

  if (!status.cortexMemService) {
    await startCortexMemService(log);
  }

  return checkServiceStatus();
}

// Get CLI binary path for external commands (like migration)
export function getCliPath(): string | null {
  return getBinaryPath("cortex-mem-cli");
}