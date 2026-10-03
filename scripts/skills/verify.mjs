#!/usr/bin/env node
import { verify } from './admission.mjs';
import { execute } from './execution.mjs';
const args = process.argv.slice(2);
if (args.length < 3 || args.length > 4 || (args.length === 4 && args[3] !== '--run')) {
  console.error('Usage: node verify.mjs BUNDLE_DIR ROM_ROOT resource|native|operator|release [--run]');
  process.exitCode = 1;
} else {
  try {
    const context = verify(args[0], args[1], args[2]);
    console.log(`Preflight passed: ${context.workflow}. No workflow ran during preflight.`);
    if (args[3] === '--run') await execute(context);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
