#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const isWindows = process.platform === 'win32';
const binName = isWindows ? 'hcp.exe' : 'hcp';
const binaryPath = path.join(__dirname, binName);

if (!fs.existsSync(binaryPath)) {
  console.error(`\x1b[31m[hypercopy]\x1b[0m Error: Native binary not found at: ${binaryPath}`);
  console.error(`Please reinstall hypercopy via: npm install -g hypercopy`);
  process.exit(1);
}

// Execute the bare-metal Rust executable with streaming stdio
const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: false
});

if (result.error) {
  console.error(`\x1b[31m[hypercopy]\x1b[0m Failed to spawn binary:`, result.error.message);
  process.exit(1);
}

process.exit(result.status !== null ? result.status : 0);
