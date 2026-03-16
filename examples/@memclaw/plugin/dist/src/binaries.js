"use strict";
/**
 * Binary management for MemClaw
 *
 * Binaries are bundled in platform-specific npm packages:
 * - @memclaw/bin-darwin-arm64 (macOS Apple Silicon)
 * - @memclaw/bin-win-x64 (Windows x64)
 *
 * The correct package is installed automatically via optionalDependencies.
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
exports.getPlatform = getPlatform;
exports.isPlatformSupported = isPlatformSupported;
exports.getUnsupportedPlatformMessage = getUnsupportedPlatformMessage;
exports.getBinaryPath = getBinaryPath;
exports.isBinaryAvailable = isBinaryAvailable;
exports.isPlatformPackageInstalled = isPlatformPackageInstalled;
exports.getInstallInstructions = getInstallInstructions;
exports.checkServiceStatus = checkServiceStatus;
exports.startQdrant = startQdrant;
exports.startCortexMemService = startCortexMemService;
exports.stopAllServices = stopAllServices;
exports.ensureAllServices = ensureAllServices;
exports.getCliPath = getCliPath;
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const child_process_1 = require("child_process");
const config_js_1 = require("./config.js");
// Platform detection
function getPlatform() {
    const platform = process.platform;
    const arch = process.arch;
    if (platform === "darwin" && arch === "arm64") {
        return "darwin-arm64";
    }
    else if (platform === "win32" && arch === "x64") {
        return "win-x64";
    }
    return null;
}
// Check if current platform is supported
function isPlatformSupported() {
    return getPlatform() !== null;
}
// Get unsupported platform message
function getUnsupportedPlatformMessage() {
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
function getBinaryFileName(binary) {
    return process.platform === "win32" ? `${binary}.exe` : binary;
}
// Get the path to the platform-specific npm package
function getPlatformPackagePath() {
    const platform = getPlatform();
    if (!platform) {
        return null;
    }
    const packageName = `@memclaw/bin-${platform}`;
    try {
        // Try to resolve the package path
        const packageJsonPath = require.resolve(`${packageName}/package.json`);
        return path.dirname(packageJsonPath);
    }
    catch {
        return null;
    }
}
// Get binary path from npm package
function getBinaryPath(binary) {
    const packagePath = getPlatformPackagePath();
    if (!packagePath) {
        return null;
    }
    const binaryFileName = getBinaryFileName(binary);
    const binaryPath = path.join(packagePath, "bin", binaryFileName);
    if (fs.existsSync(binaryPath)) {
        return binaryPath;
    }
    return null;
}
// Check if binary is available
function isBinaryAvailable(binary) {
    return getBinaryPath(binary) !== null;
}
// Check if platform package is installed
function isPlatformPackageInstalled() {
    return getPlatformPackagePath() !== null;
}
// Get installation instructions for missing platform package
function getInstallInstructions() {
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
async function checkServiceStatus() {
    const qdrant = await isPortOpen(6333);
    const cortexMemService = await isPortOpen(8085);
    return { qdrant, cortexMemService };
}
async function isPortOpen(port) {
    try {
        const response = await fetch(`http://127.0.0.1:${port}/health`, {
            method: "GET",
            signal: AbortSignal.timeout(2000),
        });
        return response.ok;
    }
    catch {
        // Try alternate endpoints for Qdrant
        try {
            if (port === 6333 || port === 6334) {
                const response = await fetch(`http://127.0.0.1:${port}`, {
                    method: "GET",
                    signal: AbortSignal.timeout(2000),
                });
                return response.ok;
            }
        }
        catch { }
        return false;
    }
}
// Running processes
const runningProcesses = new Map();
async function startQdrant(log) {
    const status = await checkServiceStatus();
    if (status.qdrant) {
        log?.("Qdrant is already running");
        return;
    }
    const binaryPath = getBinaryPath("qdrant");
    if (!binaryPath) {
        throw new Error(`Qdrant binary not found. ${getInstallInstructions()}`);
    }
    const dataDir = (0, config_js_1.getDataDir)();
    const storagePath = path.join(dataDir, "qdrant-storage");
    if (!fs.existsSync(storagePath)) {
        fs.mkdirSync(storagePath, { recursive: true });
    }
    log?.(`Starting Qdrant with storage at ${storagePath}...`);
    const proc = (0, child_process_1.spawn)(binaryPath, [
        "--storage-path",
        storagePath,
        "--http-port",
        "6333",
        "--grpc-port",
        "6334",
    ], {
        stdio: ["ignore", "pipe", "pipe"],
        detached: true,
    });
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
async function startCortexMemService(log) {
    const status = await checkServiceStatus();
    if (status.cortexMemService) {
        log?.("cortex-mem-service is already running");
        return;
    }
    const binaryPath = getBinaryPath("cortex-mem-service");
    if (!binaryPath) {
        throw new Error(`cortex-mem-service binary not found. ${getInstallInstructions()}`);
    }
    const dataDir = (0, config_js_1.getDataDir)();
    log?.(`Starting cortex-mem-service with data-dir ${dataDir}...`);
    // cortex-mem-service automatically reads config.toml from --data-dir
    const proc = (0, child_process_1.spawn)(binaryPath, ["--data-dir", dataDir], {
        stdio: ["ignore", "pipe", "pipe"],
        detached: true,
    });
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
function stopAllServices() {
    for (const [name, proc] of runningProcesses) {
        try {
            proc.kill();
            console.log(`Stopped ${name}`);
        }
        catch (err) {
            console.error(`Failed to stop ${name}:`, err);
        }
    }
    runningProcesses.clear();
}
async function ensureAllServices(log) {
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
function getCliPath() {
    return getBinaryPath("cortex-mem-cli");
}
//# sourceMappingURL=binaries.js.map