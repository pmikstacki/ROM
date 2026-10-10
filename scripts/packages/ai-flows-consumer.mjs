// Installed AI consumer facade; named modules own source checks and execution.
export { rewriteAiManifest, requireAiResults, requireAiCompiler, validateOptions } from '../../tests/ai-flows-installed/contracts.mjs';
export { auditAiGraph, auditRegistryLock, fenceInputs } from '../../tests/ai-flows-installed/provenance.mjs';
export { runAiConsumer } from '../../tests/ai-flows-installed/run.mjs';

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runAiConsumer } from '../../tests/ai-flows-installed/run.mjs';
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runAiConsumer(JSON.parse(readFileSync(process.argv[2], 'utf8')));
}
