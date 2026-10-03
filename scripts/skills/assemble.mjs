#!/usr/bin/env node
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { assemble } from './assembly.mjs';
const args = process.argv.slice(2);
if (args.length !== 1) { console.error('Usage: node assemble.mjs OUTPUT_DIR'); process.exitCode = 1; }
else {
  try { assemble(resolve(dirname(fileURLToPath(import.meta.url)), '../..'), resolve(args[0])); console.log('ROM instruction bundle assembled.'); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
