import test from 'node:test';
import assert from 'node:assert/strict';
import { requireAdmission, writeEvidence } from './evidence.mjs';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const cases = ['login', 'csrf'];
function valid() {
  return {
    schema: 'rom-identity-provider-v1', source_mode: 'verified_extracted_source',
    source_sha256: 'a'.repeat(64), final_source_sha256: 'a'.repeat(64),
    dependency_mode: 'locked_npm_ci', provider_admitted: true, tls_verified: true,
    processes: [{ pid: 99, exit_code: 0, signal: null, reason: 'completed', drained: true }],
    cases: ['sqlite', 'redb'].flatMap(adapter => ['chromium', 'webkit'].flatMap(engine =>
      cases.map(id => ({ id, adapter, engine, status: 'passed', elapsed_ms: 5 })))),
  };
}

test('admission requires the exact complete adapter and actual engine scenario matrix', () => {
  assert.doesNotThrow(() => requireAdmission(valid(), cases));
  for (const mutate of [
    r => { r.cases = r.cases.filter(c => c.engine !== 'webkit'); },
    r => { r.cases = []; },
    r => { r.cases[0].status = 'skipped'; },
    r => { r.cases.push(r.cases[0]); },
    r => { r.cases[0].engine = 'mock'; },
    r => { r.provider_admitted = false; },
    r => { r.tls_verified = false; },
    r => { r.source_mode = 'authoring_checkout'; },
    r => { r.final_source_sha256 = 'b'.repeat(64); },
    r => { r.dependency_mode = 'npm_install'; },
    r => { r.processes[0].drained = false; },
    r => { r.processes[0].reason = 'deadline'; },
  ]) { const report = valid(); mutate(report); assert.throws(() => requireAdmission(report, cases), /admission/); }
});

test('strict evidence schema rejects credentials, raw tokens and arbitrary diagnostic fields', () => {
  for (const mutate of [
    r => { r.access_token = 'private'; },
    r => { r.cases[0].cookie = 'private'; },
    r => { r.processes[0].stderr = 'authorization: bearer private'; },
    r => { r.cases[0].id = 'authorization_code=private'; },
  ]) { const report = valid(); mutate(report); assert.throws(() => requireAdmission(report, cases), /admission/); }
});

test('evidence writes are bounded and never replace an earlier report', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-identity-evidence-'));
  writeEvidence(root, 'source-identity.json', { sha256: 'a'.repeat(64) });
  assert.throws(() => writeEvidence(root, 'source-identity.json', {}), /already exists/);
  assert.equal(JSON.parse(readFileSync(join(root, 'source-identity.json'))).sha256, 'a'.repeat(64));
  assert.throws(() => writeEvidence(root, '../outside.json', {}), /evidence name/);
  assert.throws(() => writeEvidence(root, 'oversize.json', { value: 'x'.repeat(100) }, 20), /evidence limit/);
});

test('evidence writer rejects private fields and symlinked output roots before creating a file', async () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-identity-private-'));
  assert.throws(() => writeEvidence(root, 'private.json', { nested: { client_secret: 'private' } }), /private evidence/);
  const { symlinkSync, existsSync } = await import('node:fs');
  const link = join(root, 'shortcut'); symlinkSync(root, link);
  assert.throws(() => writeEvidence(link, 'linked.json', {}), /evidence root/);
  assert.equal(existsSync(join(root, 'private.json')), false);
  assert.equal(existsSync(join(root, 'linked.json')), false);
});

test('owned graceful provider stop is admitted while unexpected signals remain failures', () => {
  const report = valid(); report.processes[0] = { pid: 99, exit_code: null, signal: 'SIGTERM', reason: 'shutdown', drained: true };
  assert.doesNotThrow(() => requireAdmission(report, cases));
  report.processes[0].reason = 'completed';
  assert.throws(() => requireAdmission(report, cases), /admission/);
});

test('malformed process records and coercible engine names fail the closed schema', () => {
  for (const mutate of [
    r => { r.processes[0] = null; },
    r => { r.cases[0] = null; },
    r => { r.cases[0].engine = ['chromium']; },
    r => { r.cases[0].adapter = ['sqlite']; },
    r => { const digest = ['a'.repeat(64)]; r.source_sha256 = digest; r.final_source_sha256 = digest; },
  ]) { const report = valid(); mutate(report); assert.throws(() => requireAdmission(report, cases), /admission/); }
});
