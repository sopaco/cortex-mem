#!/usr/bin/env node

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// Get project root directory (parent of scripts directory)
const PROJECT_ROOT = path.resolve(__dirname, '..');

// ANSI color codes for terminal output
const colors = {
	reset: '\x1b[0m',
	bright: '\x1b[1m',
	red: '\x1b[31m',
	green: '\x1b[32m',
	yellow: '\x1b[33m',
	blue: '\x1b[34m',
	magenta: '\x1b[35m',
	cyan: '\x1b[36m'
};

// Helper function to colorize output
function colorize(text, color) {
	return `${colors[color]}${text}${colors.reset}`;
}

// Crates to publish in dependency order (excluding cortex-mem-insights which is a web project)
// Order based on dependencies: config -> core -> tools -> rig -> (service, cli, mcp)
// Note: cortex-mem-tars is excluded as it's an example project
const CRATES_TO_PUBLISH = [
	{ name: 'cortex-mem-config', path: 'cortex-mem-config', deps: [] },
	{ name: 'cortex-mem-core', path: 'cortex-mem-core', deps: ['cortex-mem-config'] },
	{ name: 'cortex-mem-tools', path: 'cortex-mem-tools', deps: ['cortex-mem-core'] },
	{ name: 'cortex-mem-rig', path: 'cortex-mem-rig', deps: ['cortex-mem-core', 'cortex-mem-tools'] },
	{ name: 'cortex-mem-service', path: 'cortex-mem-service', deps: ['cortex-mem-core', 'cortex-mem-config'] },
	{ name: 'cortex-mem-cli', path: 'cortex-mem-cli', deps: ['cortex-mem-core', 'cortex-mem-tools', 'cortex-mem-config'] },
	{ name: 'cortex-mem-mcp', path: 'cortex-mem-mcp', deps: ['cortex-mem-core', 'cortex-mem-tools', 'cortex-mem-config'] }
];

// Get workspace version from root Cargo.toml
function getWorkspaceVersion() {
	const workspaceCargoPath = path.join(PROJECT_ROOT, 'Cargo.toml');
	const content = fs.readFileSync(workspaceCargoPath, 'utf8');
	// Look for version in [workspace.package] section
	const match = content.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
	if (match) return match[1];
	// Fallback: look for standalone version
	const fallbackMatch = content.match(/^version\s*=\s*"([^"]+)"/m);
	return fallbackMatch ? fallbackMatch[1] : null;
}

// Cached workspace version
const WORKSPACE_VERSION = getWorkspaceVersion();

// Get version from Cargo.toml (supports both standalone and workspace version)
function getVersion(cratePath) {
	const cargoTomlPath = path.join(PROJECT_ROOT, cratePath, 'Cargo.toml');
	const content = fs.readFileSync(cargoTomlPath, 'utf8');
	
	// Check if using workspace version
	if (/version\.workspace\s*=\s*true/.test(content)) {
		return WORKSPACE_VERSION;
	}
	
	// Look for standalone version
	const match = content.match(/^version\s*=\s*"([^"]+)"/m);
	return match ? match[1] : null;
}

// Check if crate has path dependencies
function hasPathDependencies(cratePath) {
	const cargoTomlPath = path.join(PROJECT_ROOT, cratePath, 'Cargo.toml');
	const content = fs.readFileSync(cargoTomlPath, 'utf8');
	return /path\s*=\s*"[^"]*"/.test(content);
}

// Get version of a dependency by its name
function getDependencyVersion(dependencyName) {
	for (const crate of CRATES_TO_PUBLISH) {
		if (crate.name === dependencyName) {
			return getVersion(crate.path);
		}
	}
	return null;
}

// Convert path dependencies to version dependencies for publishing
function prepareForPublishing(cratePath) {
	const cargoTomlPath = path.join(PROJECT_ROOT, cratePath, 'Cargo.toml');
	const content = fs.readFileSync(cargoTomlPath, 'utf8');
	const lines = content.split('\n');
	const modifiedLines = [];
	let hasChanges = false;

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		const modifiedLine = line.replace(/\{\s*path\s*=\s*"([^"]+)"\s*\}/g, (match, pathValue) => {
			// Extract dependency name from the line before this match
			const depNameMatch = lines[i].match(/^(\S+)\s*=/);
			if (!depNameMatch) return match;

			const depName = depNameMatch[1].trim();
			const version = getDependencyVersion(depName);

			if (version) {
				hasChanges = true;
				console.log(
					colorize(
						`    Converting: ${depName} = { path = "${pathValue}" } -> { version = "${version}" }`,
						'yellow'
					)
				);
				return `{ version = "${version}" }`;
			}
			return match;
		});

		modifiedLines.push(modifiedLine);
	}

	if (hasChanges) {
		// Backup original file
		fs.copyFileSync(cargoTomlPath, cargoTomlPath + '.bak');
		// Write modified content
		fs.writeFileSync(cargoTomlPath, modifiedLines.join('\n'), 'utf8');
		return true;
	}

	return false;
}

// Restore original Cargo.toml
function restoreCargoToml(cratePath) {
	const cargoTomlPath = path.join(PROJECT_ROOT, cratePath, 'Cargo.toml');
	const backupPath = cargoTomlPath + '.bak';

	if (fs.existsSync(backupPath)) {
		fs.copyFileSync(backupPath, cargoTomlPath);
		fs.unlinkSync(backupPath);
		console.log(colorize('    Restored original Cargo.toml', 'blue'));
	}
}

// Run cargo publish with retry on network errors
function publishCrate(cratePath, dryRun = false, maxRetries = 3) {
	let command = dryRun ? 'cargo publish --dry-run --allow-dirty' : 'cargo publish --allow-dirty';

	for (let attempt = 1; attempt <= maxRetries; attempt++) {
		try {
			execSync(command, {
				cwd: path.join(PROJECT_ROOT, cratePath),
				stdio: 'inherit'
			});
			return { success: true, isNetworkError: false };
		} catch (error) {
			const errorMsg = error.message || '';
			const isNetworkError = /SSL|network|connection|timeout|reset/i.test(errorMsg);
			
			if (!dryRun && isNetworkError && attempt < maxRetries) {
				console.log(
					colorize(
						`    ⚠️  Network error detected, retrying (${attempt}/${maxRetries})...`,
						'yellow'
					)
				);
				// Wait a bit before retrying
				const waitTime = attempt * 5000;
				console.log(colorize(`    Waiting ${waitTime / 1000}s before retry...`, 'yellow'));
				execSync(`timeout /t ${waitTime / 1000} /nobreak`, { stdio: 'inherit' });
				continue;
			}
			
			return { success: false, isNetworkError };
		}
	}
	
	return { success: false, isNetworkError: true };
}

// Helper: Make HTTP GET request (cross-platform, no curl dependency)
function httpGet(url) {
	return new Promise((resolve, reject) => {
		const https = require('https');
		https
			.get(url, (res) => {
				let data = '';
				res.on('data', (chunk) => (data += chunk));
				res.on('end', () => {
					if (res.statusCode >= 200 && res.statusCode < 300) {
						resolve(data);
					} else {
						reject(new Error(`HTTP ${res.statusCode}`));
					}
				});
			})
			.on('error', reject);
	});
}

// Wait for a crate to be available on crates.io
function waitForCrateAvailability(crateName, targetVersion, maxWaitSeconds = 180) {
	console.log(
		colorize(
			`    Waiting for ${crateName} v${targetVersion} to be available on crates.io...`,
			'cyan'
		)
	);

	const startTime = Date.now();
	const checkInterval = 5000; // Check every 5 seconds

	return new Promise((resolve, reject) => {
		const checkAvailability = async () => {
			try {
				// Use native HTTPS instead of curl for cross-platform compatibility
				const result = await httpGet(`https://crates.io/api/v1/crates/${crateName}`);
				const data = JSON.parse(result);
				const versions = data.versions || [];
				const targetVersionExists = versions.some(v => v.num === targetVersion);
				
				if (targetVersionExists) {
					console.log(
						colorize(`    ✓ ${crateName} v${targetVersion} is now available on crates.io`, 'green')
					);
					resolve();
				} else {
					throw new Error('Version not yet available');
				}
			} catch (error) {
				const elapsed = (Date.now() - startTime) / 1000;
				if (elapsed >= maxWaitSeconds) {
					reject(new Error(`Timeout waiting for ${crateName} v${targetVersion} to be available`));
				} else {
					setTimeout(checkAvailability, checkInterval);
				}
			}
		};

		checkAvailability();
	});
}

// Check if a specific version of a crate is already published on crates.io (async)
async function isVersionPublished(crateName, version) {
	try {
		const result = await httpGet(`https://crates.io/api/v1/crates/${crateName}`);
		const data = JSON.parse(result);
		const versions = data.versions || [];
		return versions.some(v => v.num === version);
	} catch (error) {
		return false;
	}
}

// Main function
async function main() {
	console.log(colorize('='.repeat(60), 'cyan'));
	console.log(colorize('Cortex Mem Crates Publishing Tool', 'bright'));
	console.log(colorize('='.repeat(60), 'cyan'));

	const args = process.argv.slice(2);
	const dryRun = args.includes('--dry-run');
	const skipWait = args.includes('--skip-wait');
	const force = args.includes('--force'); // New flag to force republish
	const autoConfirm = args.includes('--yes') || args.includes('-y'); // Auto-confirm flag

	if (dryRun) {
		console.log(colorize('\n⚠️  DRY RUN MODE - No actual publishing will occur', 'yellow'));
		console.log(colorize('   Note: --allow-dirty flag is automatically added', 'yellow'));
	}

	if (force) {
		console.log(colorize('\n⚠️  FORCE MODE - Will attempt to republish all crates', 'yellow'));
	}

	console.log(colorize('\n📦 Crates to publish (in dependency order):', 'blue'));
	for (let index = 0; index < CRATES_TO_PUBLISH.length; index++) {
		const crate = CRATES_TO_PUBLISH[index];
		const version = getVersion(crate.path);
		const published = await isVersionPublished(crate.name, version);
		console.log(
			`  ${index + 1}. ${colorize(crate.name, published ? 'yellow' : 'green')} v${version} ${published ? '(already published)' : ''}`
		);
	}

	console.log(colorize('\n' + '='.repeat(60), 'cyan'));

	// Ask for confirmation
	if (!dryRun && !autoConfirm) {
		console.log(colorize('\n⚠️  This will publish the above crates to crates.io', 'yellow'));
		console.log(colorize('Press Ctrl+C to cancel, or press Enter to continue...', 'yellow'));
		await new Promise((resolve) => {
			process.stdin.once('data', resolve);
		});
	}

	let successCount = 0;
	let failCount = 0;
	let skippedCount = 0;
	
	// Track successfully published crates for dependency checking
	const publishedCrates = new Set();

	for (let i = 0; i < CRATES_TO_PUBLISH.length; i++) {
		const crate = CRATES_TO_PUBLISH[i];
		const version = getVersion(crate.path);

		// Check if all dependencies have been successfully published
		const failedDeps = crate.deps.filter(dep => !publishedCrates.has(dep));
		if (failedDeps.length > 0 && !force) {
			console.log(
				colorize(
					`\n⏭️  [${i + 1}/${CRATES_TO_PUBLISH.length}] Skipping ${crate.name} v${version}`,
					'yellow'
				)
			);
			console.log(
				colorize(
					`    Reason: Dependencies not published: ${failedDeps.join(', ')}`,
					'red'
				)
			);
			skippedCount++;
			continue;
		}

		// Skip if already published (unless force mode)
		if (!force && (await isVersionPublished(crate.name, version))) {
			console.log(
				colorize(
					`\n⏭️  [${i + 1}/${CRATES_TO_PUBLISH.length}] Skipping ${crate.name} v${version} - already published`,
					'yellow'
				)
			);
			publishedCrates.add(crate.name);
			skippedCount++;
			continue;
		}

		console.log(
			colorize(
				`\n📦 [${i + 1}/${CRATES_TO_PUBLISH.length}] Publishing ${crate.name} v${version}`,
				'bright'
			)
		);

		// Check if crate has path dependencies
		if (hasPathDependencies(crate.path)) {
			console.log(
				colorize('    ⚠️  Found path dependencies, converting for publishing...', 'yellow')
			);
			const prepared = prepareForPublishing(crate.path);
			if (prepared) {
				console.log(colorize('    ✓ Dependencies converted', 'green'));
			}
		}

		// Dry run first
		console.log(colorize('    🔍 Running dry-run check...', 'cyan'));
		const dryRunResult = publishCrate(crate.path, true);

		if (!dryRunResult.success) {
			console.log(colorize('    ✗ Dry run failed, skipping publish', 'red'));
			restoreCargoToml(crate.path);
			failCount++;
			continue;
		}

		console.log(colorize('    ✓ Dry run passed', 'green'));

		// Actual publish
		if (!dryRun) {
			console.log(colorize('    🚀 Publishing to crates.io...', 'cyan'));
			const publishResult = publishCrate(crate.path, false, 3); // 3 retries

			if (publishResult.success) {
				console.log(colorize(`    ✓ ${crate.name} v${version} published successfully!`, 'green'));
				successCount++;
				publishedCrates.add(crate.name);

				// Wait for crate to be available on crates.io (unless it's the last crate)
				if (i < CRATES_TO_PUBLISH.length - 1 && !skipWait) {
					try {
						await waitForCrateAvailability(crate.name, version, 180);
					} catch (error) {
						console.log(colorize(`    ⚠️  Warning: ${error.message}`, 'yellow'));
						console.log(colorize('    Continuing anyway...', 'yellow'));
					}
				}
			} else {
				console.log(colorize(`    ✗ Failed to publish ${crate.name}`, 'red'));
				if (publishResult.isNetworkError) {
					console.log(
						colorize(
							'    💡 Tip: Check your network connection, firewall, or proxy settings.',
							'yellow'
						)
					);
				}
				failCount++;
			}
		} else {
			console.log(colorize(`    ✓ ${crate.name} v${version} ready for publishing`, 'green'));
			successCount++;
			publishedCrates.add(crate.name);
		}

		// Restore original Cargo.toml
		restoreCargoToml(crate.path);
	}

	// Summary
	console.log(colorize('\n' + '='.repeat(60), 'cyan'));
	console.log(colorize('Publish Summary:', 'bright'));
	console.log(
		`  ${colorize('✓', 'green')} ${successCount} crates ${dryRun ? 'ready' : 'published'} successfully`
	);
	if (skippedCount > 0) {
		console.log(`  ${colorize('⏭️', 'yellow')} ${skippedCount} crates skipped`);
	}
	if (failCount > 0) {
		console.log(`  ${colorize('✗', 'red')} ${failCount} crates failed`);
	}
	console.log(colorize('='.repeat(60), 'cyan'));

	if (failCount > 0) {
		console.log(
			colorize('\n⚠️  Some crates failed to publish. Please check the errors above.', 'yellow')
		);
		process.exit(1);
	} else if (!dryRun && successCount > 0) {
		console.log(colorize('\n🎉 All crates published successfully!', 'green'));
		console.log(colorize('You can now install them from crates.io', 'blue'));
		process.exit(0);
	} else if (dryRun) {
		console.log(colorize('\n✅ Dry run completed successfully!', 'green'));
		console.log(colorize('Run without --dry-run to actually publish the crates.', 'yellow'));
		process.exit(0);
	} else {
		console.log(colorize('\nℹ️  No crates needed to be published.', 'blue'));
		process.exit(0);
	}
}

// Run the script
main().catch((error) => {
	console.error(colorize('\n❌ Error:', 'red'), error.message);
	process.exit(1);
});