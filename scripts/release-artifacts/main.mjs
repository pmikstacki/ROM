#!/usr/bin/env node
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { produce } from './produce.mjs';

if (process.argv.length > 3 || process.argv[2]?.startsWith('-')) {
  console.error('Usage: scripts/release [OUTPUT_DIR]');
  process.exitCode = 2;
} else {
  try { await produce({ root: resolve(dirname(fileURLToPath(import.meta.url)), '../..'), output: process.argv[2] }); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
