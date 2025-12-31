#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const glob = require('glob');

const VERSION = '1.0.0';
const CARGO_TOML_PATTERN = '**/Cargo.toml';
const EXCLUDE_PATTERNS = ['**/target/**', '**/node_modules/**', '**/.git/**'];

// ANSI color codes for terminal output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
};

// Helper function to colorize output
function colorize(text, color) {
  return `${colors[color]}${text}${colors.reset}`;
}

// Function to find all Cargo.toml files
function findCargoTomlFiles() {
  console.log(colorize('Scanning for Cargo.toml files...', 'cyan'));

  try {
    const files = glob.sync(CARGO_TOML_PATTERN, {
      ignore: EXCLUDE_PATTERNS,
      cwd: process.cwd(),
      absolute: true
    });

    console.log(colorize(`Found ${files.length} Cargo.toml files`, 'green'));
    return files;
  } catch (error) {
    console.error(colorize('Error finding Cargo.toml files:', 'red'), error);
    process.exit(1);
  }
}

// Function to update version in a Cargo.toml file
function updateVersionInCargoToml(filePath) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n');
    let versionFound = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Look for version in package section
      if (line.trim().startsWith('version = ')) {
        lines[i] = `version = "${VERSION}"`;
        versionFound = true;
        console.log(colorize(`  Updated version in ${path.relative(process.cwd(), filePath)}`, 'green'));
        break;
      }
    }

    if (!versionFound) {
      console.log(colorize(`  No version found in ${path.relative(process.cwd(), filePath)}`, 'yellow'));
      return false;
    }

    fs.writeFileSync(filePath, lines.join('\n'), 'utf8');
    return true;
  } catch (error) {
    console.error(colorize(`Error processing ${filePath}:`, 'red'), error);
    return false;
  }
}

// Function to update internal dependencies
function updateInternalDependencies(filePath) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n');
    let updated = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Look for internal dependencies
      if (line.includes('path = ')) {
        // Check if it's an internal dependency by checking if it has a path to a local crate
        if (line.includes('cortex-mem-') || line.includes('../')) {
          // Update version for internal dependencies
          const versionMatch = line.match(/version\s*=\s*"([^"]+)"/);
          if (versionMatch) {
            lines[i] = line.replace(/version\s*=\s*"[^"]+"/, `version = "${VERSION}"`);
            updated = true;
          }
        }
      }
    }

    if (updated) {
      fs.writeFileSync(filePath, lines.join('\n'), 'utf8');
      console.log(colorize(`  Updated internal dependencies in ${path.relative(process.cwd(), filePath)}`, 'blue'));
    }

    return updated;
  } catch (error) {
    console.error(colorize(`Error updating dependencies in ${filePath}:`, 'red'), error);
    return false;
  }
}

// Main function
function main() {
  console.log(colorize('='.repeat(50), 'cyan'));
  console.log(colorize('Cargo.toml Version Updater', 'bright'));
  console.log(colorize(`Updating all versions to ${VERSION}`, 'bright'));
  console.log(colorize('='.repeat(50), 'cyan'));

  const files = findCargoTomlFiles();
  let updatedFiles = 0;
  let updatedDependencies = 0;

  // First pass: update package versions
  console.log(colorize('\nUpdating package versions...', 'cyan'));
  for (const file of files) {
    if (updateVersionInCargoToml(file)) {
      updatedFiles++;
    }
  }

  // Second pass: update internal dependencies
  console.log(colorize('\nUpdating internal dependencies...', 'cyan'));
  for (const file of files) {
    if (updateInternalDependencies(file)) {
      updatedDependencies++;
    }
  }

  // Summary
  console.log(colorize('\n' + '='.repeat(50), 'cyan'));
  console.log(colorize('Update Summary:', 'bright'));
  console.log(`  ${colorize(updatedFiles.toString(), 'green')} package versions updated`);
  console.log(`  ${colorize(updatedDependencies.toString(), 'blue')} dependency references updated`);
  console.log(colorize('='.repeat(50), 'cyan'));

  if (updatedFiles > 0) {
    console.log(colorize('\nVersion update completed successfully!', 'green'));
    console.log(colorize('You may want to run "cargo check" to verify all changes.', 'yellow'));
  } else {
    console.log(colorize('\nNo files were updated.', 'yellow'));
  }
}

// Check if glob module is available
try {
  require.resolve('glob');
} catch (e) {
  console.error(colorize('Error: The "glob" package is required but not installed.', 'red'));
  console.error(colorize('Please install it with: npm install glob', 'yellow'));
  process.exit(1);
}

// Run the script
main();
