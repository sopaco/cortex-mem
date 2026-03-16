/**
 * Binary management for MemClaw
 *
 * Binaries are bundled in platform-specific npm packages:
 * - @memclaw/bin-darwin-arm64 (macOS Apple Silicon)
 * - @memclaw/bin-win-x64 (Windows x64)
 *
 * The correct package is installed automatically via optionalDependencies.
 */

import * as fs from 'fs';
import * as path from 'path';
import { spawn, ChildProcess } from 'child_process';
import { getDataDir } from './config.js';

// Binary names type
type BinaryName = 'qdrant' | 'cortex-mem-service' | 'cortex-mem-cli';

// Supported platforms
type SupportedPlatform = 'darwin-arm64' | 'win-x64';

// Platform detection
export function getPlatform(): SupportedPlatform | null {
	const platform = process.platform;
	const arch = process.arch;

	if (platform === 'darwin' && arch === 'arm64') {
		return 'darwin-arm64';
	} else if (platform === 'win32' && arch === 'x64') {
		return 'win-x64';
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
	return process.platform === 'win32' ? `${binary}.exe` : binary;
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
	const binaryPath = path.join(packagePath, 'bin', binaryFileName);

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
		const response = await fetch(`http://localhost:${port}/health`, {
			method: 'GET',
			signal: AbortSignal.timeout(2000)
		});
		return response.ok;
	} catch {
		// Try alternate endpoints for Qdrant
		try {
			if (port === 6333 || port === 6334) {
				const response = await fetch(`http://localhost:${port}`, {
					method: 'GET',
					signal: AbortSignal.timeout(2000)
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
		log?.('Qdrant is already running');
		return;
	}

	const binaryPath = getBinaryPath('qdrant');
	if (!binaryPath) {
		throw new Error(`Qdrant binary not found. ${getInstallInstructions()}`);
	}

	const dataDir = getDataDir();
	const storagePath = path.join(dataDir, 'qdrant-storage');

	if (!fs.existsSync(storagePath)) {
		fs.mkdirSync(storagePath, { recursive: true });
	}

	log?.(`Starting Qdrant with storage at ${storagePath}...`);
	log?.(`Binary path: ${binaryPath}`);

	const proc = spawn(
		binaryPath,
		['--storage-path', storagePath, '--http-port', '6333', '--grpc-port', '6334'],
		{
			stdio: ['ignore', 'pipe', 'pipe'],
			detached: true
		}
	);

	// Drain stdout/stderr to prevent buffer blocking
	proc.stdout?.on('data', (data) => {
		log?.(`[qdrant stdout] ${data.toString().trim()}`);
	});

	proc.stderr?.on('data', (data) => {
		log?.(`[qdrant stderr] ${data.toString().trim()}`);
	});

	proc.on('error', (err) => {
		log?.(`Qdrant error: ${err.message}`);
	});

	proc.on('exit', (code, signal) => {
		if (code !== null && code !== 0) {
			log?.(`Qdrant exited with code ${code}`);
		}
		if (signal) {
			log?.(`Qdrant killed by signal ${signal}`);
		}
	});

	proc.unref();
	runningProcesses.set('qdrant', proc);

	// Wait for Qdrant to start
	let retries = 30;
	while (retries > 0) {
		const status = await checkServiceStatus();
		if (status.qdrant) {
			log?.('Qdrant started successfully');
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 500));
		retries--;
	}

	throw new Error('Qdrant failed to start within 15 seconds');
}

export async function startCortexMemService(log?: (msg: string) => void): Promise<void> {
	const status = await checkServiceStatus();
	if (status.cortexMemService) {
		log?.('cortex-mem-service is already running');
		return;
	}

	const binaryPath = getBinaryPath('cortex-mem-service');
	if (!binaryPath) {
		throw new Error(`cortex-mem-service binary not found. ${getInstallInstructions()}`);
	}

	const dataDir = getDataDir();

	log?.(`Starting cortex-mem-service with data-dir ${dataDir}...`);
	log?.(`Binary path: ${binaryPath}`);

	// cortex-mem-service automatically reads config.toml from --data-dir
	const proc = spawn(binaryPath, ['--data-dir', dataDir], {
		stdio: ['ignore', 'pipe', 'pipe'],
		detached: true
	});

	// Drain stdout/stderr to prevent buffer blocking
	proc.stdout?.on('data', (data) => {
		log?.(`[cortex-mem-service stdout] ${data.toString().trim()}`);
	});

	proc.stderr?.on('data', (data) => {
		log?.(`[cortex-mem-service stderr] ${data.toString().trim()}`);
	});

	proc.on('error', (err) => {
		log?.(`cortex-mem-service error: ${err.message}`);
	});

	proc.on('exit', (code, signal) => {
		if (code !== null && code !== 0) {
			log?.(`cortex-mem-service exited with code ${code}`);
		}
		if (signal) {
			log?.(`cortex-mem-service killed by signal ${signal}`);
		}
	});

	proc.unref();
	runningProcesses.set('cortex-mem-service', proc);

	// Wait for service to start
	let retries = 30;
	while (retries > 0) {
		const status = await checkServiceStatus();
		if (status.cortexMemService) {
			log?.('cortex-mem-service started successfully');
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 500));
		retries--;
	}

	throw new Error('cortex-mem-service failed to start within 15 seconds');
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

export async function ensureAllServices(log?: (msg: string) => void): Promise<ServiceStatus> {
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
	return getBinaryPath('cortex-mem-cli');
}

// Execute CLI command and return output
export interface CliResult {
	success: boolean;
	stdout: string;
	stderr: string;
	exitCode: number | null;
}

export async function executeCliCommand(
	args: string[],
	configPath: string,
	tenantId: string,
	timeout: number = 120000
): Promise<CliResult> {
	const cliPath = getCliPath();
	if (!cliPath) {
		return {
			success: false,
			stdout: '',
			stderr: 'cortex-mem-cli binary not found',
			exitCode: 1,
		};
	}

	const fullArgs = [
		'--config', configPath,
		'--tenant', tenantId,
		...args
	];

	return new Promise((resolve) => {
		let stdout = '';
		let stderr = '';

		const proc = spawn(cliPath, fullArgs, {
			stdio: ['ignore', 'pipe', 'pipe'],
		});

		proc.stdout?.on('data', (data) => {
			stdout += data.toString();
		});

		proc.stderr?.on('data', (data) => {
			stderr += data.toString();
		});

		const timer = setTimeout(() => {
			proc.kill();
			resolve({
				success: false,
				stdout,
				stderr: stderr + '\nCommand timed out',
				exitCode: null,
			});
		}, timeout);

		proc.on('close', (code) => {
			clearTimeout(timer);
			resolve({
				success: code === 0,
				stdout,
				stderr,
				exitCode: code,
			});
		});

		proc.on('error', (err) => {
			clearTimeout(timer);
			resolve({
				success: false,
				stdout,
				stderr: err.message,
				exitCode: 1,
			});
		});
	});
}
