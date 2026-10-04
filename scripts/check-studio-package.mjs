#!/usr/bin/env node
// Standalone finite consumer; release production retains its complete fixed gate.
import { verifyStudioPackage } from './packages/studio-consumer.mjs';

if (process.argv.length !== 4) {
  console.error('usage: node scripts/check-studio-package.mjs ROM_ROOT OUTPUT_DIR');
  process.exitCode = 2;
} else {
  try {
    const result = await verifyStudioPackage({ root: process.argv[2], output: process.argv[3] });
    console.log(`Studio package accepted: ${result.artifact.sha256}`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
