import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { reserveOutput, assertLockedGraph, requireAdmission } from './verification.mjs';

test('a reused evidence directory is rejected before existing evidence changes', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-controls-evidence-'));
  try {
    writeFileSync(join(root, 'result.json'), 'historical-success');
    assert.throws(() => reserveOutput(root), /already exists/);
    assert.equal(readFileSync(join(root, 'result.json'), 'utf8'), 'historical-success');
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('the verifier refuses an occupied output before overwriting an archive or successful result', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-controls-cli-evidence-'));
  try {
    mkdirSync(join(root, 'extracted'));
    writeFileSync(join(root, 'result.json'), 'historical-success');
    writeFileSync(join(root, 'studio-source.tar.gz'), 'historical-archive');
    const result = spawnSync(process.execPath, [fileURLToPath(new URL('./verify.mjs', import.meta.url)), root], { encoding: 'utf8' });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /already exists/);
    assert.equal(readFileSync(join(root, 'result.json'), 'utf8'), 'historical-success');
    assert.equal(readFileSync(join(root, 'studio-source.tar.gz'), 'utf8'), 'historical-archive');
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('locked admission rejects a changed dependency resolution', () => {
  const frozen = { packages: { '': { name: 'consumer' }, 'node_modules/svelte': { version: '5.57.1', integrity: 'fixed' } } };
  assert.doesNotThrow(() => assertLockedGraph(frozen, structuredClone(frozen)));
  for (const mutation of [
    { version: '5.58.0', integrity: 'fixed' },
    { version: '5.57.1', integrity: 'changed' },
  ]) {
    const changed = structuredClone(frozen);
    changed.packages['node_modules/svelte'] = mutation;
    assert.throws(() => assertLockedGraph(frozen, changed), /dependency graph drift/);
  }
});

const report = projects => ({ suites: [{ specs: [{ tests: projects.map(projectName => ({ projectName, results: [{ status: 'passed' }] })) }] }] });
const admitted = { completed: true, dependency_bootstrap: 'locked_npm_ci', source_mode: 'release_artifact', browser_executed: true, installed_source_matches: true };

test('release admission rejects skipped browsers, one engine and copied dependencies', () => {
  assert.throws(() => requireAdmission({ ...admitted, browser_executed: false }, report(['chromium', 'webkit'])), /browser/);
  assert.throws(() => requireAdmission(admitted, report(['chromium'])), /webkit/);
  assert.throws(() => requireAdmission({ ...admitted, dependency_bootstrap: 'physical_copy_of_existing_graph' }, report(['chromium', 'webkit'])), /locked/);
  assert.throws(() => requireAdmission({ ...admitted, source_mode: 'authoring_checkout' }, report(['chromium', 'webkit'])), /artifact/);
  assert.doesNotThrow(() => requireAdmission(admitted, report(['chromium', 'webkit'])));
});

test('release admission rejects failed engine results and unmatched installed sources', () => {
  const failed = report(['chromium', 'webkit']);
  failed.suites[0].specs[0].tests[1].results[0].status = 'failed';
  assert.throws(() => requireAdmission(admitted, failed), /failed/);
  assert.throws(() => requireAdmission({ ...admitted, installed_source_matches: false }, report(['chromium', 'webkit'])), /source/);
});
