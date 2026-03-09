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
	{ name: 'cortex-mem-config', path: 'cortex-mem-config' },
	{ name: 'cortex-mem-core', path: 'cortex-mem-core' },
	{ name: 'cortex-mem-tools', path: 'cortex-mem-tools' },
	{ name: 'cortex-mem-rig', path: 'cortex-mem-rig' },
	{ name: 'cortex-mem-service', path: 'cortex-mem-service' },
	{ name: 'cortex-mem-cli', path: 'cortex-mem-cli' },
	{ name: 'cortex-mem-mcp', path: 'cortex-mem-mcp' }
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

// Run cargo publish
function publishCrate(cratePath, dryRun = false) {
	try {
		let command = dryRun ? 'cargo publish --dry-run --allow-dirty' : 'cargo publish --allow-dirty';

		execSync(command, {
			cwd: path.join(PROJECT_ROOT, cratePath),
			stdio: 'inherit'
		});
		return true;
	} catch (error) {
		return false;
	}
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
function waitForCrateAvailability(crateName, maxWaitSeconds = 120) {
	console.log(colorize(`    Waiting for ${crateName} to be available on crates.io...`, 'cyan'));

	const startTime = Date.now();
	const checkInterval = 5000; // Check every 5 seconds

	return new Promise((resolve, reject) => {
		const checkAvailability = async () => {
			try {
				// Use native HTTPS instead of curl for cross-platform compatibility
				await httpGet(`https://crates.io/api/v1/crates/${crateName}`);
				console.log(colorize(`    ✓ ${crateName} is now available on crates.io`, 'green'));
				resolve();
			} catch (error) {
				const elapsed = (Date.now() - startTime) / 1000;
				if (elapsed >= maxWaitSeconds) {
					reject(new Error(`Timeout waiting for ${crateName} to be available`));
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
		const newestVersion = data.crate?.newest_version || data.crate?.max_version;
		return newestVersion === version;
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

	for (let i = 0; i < CRATES_TO_PUBLISH.length; i++) {
		const crate = CRATES_TO_PUBLISH[i];
		const version = getVersion(crate.path);

		// Skip if already published (unless force mode)
		if (!force && (await isVersionPublished(crate.name, version))) {
			console.log(
				colorize(
					`\n⏭️  [${i + 1}/${CRATES_TO_PUBLISH.length}] Skipping ${crate.name} v${version} - already published`,
					'yellow'
				)
			);
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
		const dryRunSuccess = publishCrate(crate.path, true);

		if (!dryRunSuccess) {
			console.log(colorize('    ✗ Dry run failed, skipping publish', 'red'));
			restoreCargoToml(crate.path);
			failCount++;
			continue;
		}

		console.log(colorize('    ✓ Dry run passed', 'green'));

		// Actual publish
		if (!dryRun) {
			console.log(colorize('    🚀 Publishing to crates.io...', 'cyan'));
			const publishSuccess = publishCrate(crate.path, false);

			if (publishSuccess) {
				console.log(colorize(`    ✓ ${crate.name} v${version} published successfully!`, 'green'));
				successCount++;

				// Wait for crate to be available on crates.io (unless it's the last crate)
				if (i < CRATES_TO_PUBLISH.length - 1 && !skipWait) {
					try {
						await waitForCrateAvailability(crate.name, 120);
					} catch (error) {
						console.log(colorize(`    ⚠️  Warning: ${error.message}`, 'yellow'));
						console.log(colorize('    Continuing anyway...', 'yellow'));
					}
				}
			} else {
				console.log(colorize(`    ✗ Failed to publish ${crate.name}`, 'red'));
				failCount++;
			}
		} else {
			console.log(colorize(`    ✓ ${crate.name} v${version} ready for publishing`, 'green'));
			successCount++;
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
		console.log(`  ${colorize('⏭️', 'yellow')} ${skippedCount} crates skipped (already published)`);
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
	} else if (!dryRun) {
		console.log(colorize('\n🎉 All crates published successfully!', 'green'));
		console.log(colorize('You can now install them from crates.io', 'blue'));
		process.exit(0);
	} else {
		console.log(colorize('\n✅ Dry run completed successfully!', 'green'));
		console.log(colorize('Run without --dry-run to actually publish the crates.', 'yellow'));
		process.exit(0);
	}
}

// Run the script
main().catch((error) => {
	console.error(colorize('\n❌ Error:', 'red'), error.message);
	process.exit(1);
});
