import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync, linkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { hash } from '../skills/files.mjs';
import * as admission from './requirements.mjs';

function fixture() {
  const directory = mkdtempSync(join(tmpdir(), 'rom-requirements-'));
  mkdirSync(join(directory, 'evidence'));
  writeFileSync(join(directory, 'evidence/result.json'), 'executed fixture report');
  writeFileSync(join(directory, 'evidence/review.json'), 'independent fixture review');
  const ref = (path, kind) => ({ path, kind, bytes: Buffer.byteLength(path.endsWith('result.json') ? 'executed fixture report' : 'independent fixture review'), sha256: hash(join(directory, path)) });
  const source = 'a'.repeat(64);
  const record = { schema_version: 1, source_identity: source, requirements: Array.from({ length: 14 }, (_, i) => ({ id: `R${i + 1}`, state: 'accepted', source_identity: source, evidence: [ref('evidence/result.json', 'executed')], measured_scope: 'Synthetic fixture; not real release acceptance.', limitations: 'Static consistency only.', review: ref('evidence/review.json', 'review') })) };
  return { directory, source, record };
}

test('release requirement ledger validates all fourteen source-bound records without executing reports', () => {
  const { directory, source, record } = fixture();
  assert.deepEqual(admission.requireReleaseRequirements(record, source, directory), { complete: true, accepted: 14, pending: [], failed: [] });
});

test('pending or failed requirements remain diagnostic records but block explicit release admission', () => {
  const { directory, source, record } = fixture();
  record.requirements[3] = { ...record.requirements[3], state: 'pending', evidence: [], measured_scope: '', review: null };
  record.requirements[7].state = 'failed';
  assert.deepEqual(admission.requireReleaseRequirements(record, source, directory), { complete: false, accepted: 12, pending: ['R4'], failed: ['R8'] });
  assert.throws(() => admission.requireReleaseRequirements(record, source, directory, { requireComplete: true }), /incomplete release requirements/);
});

test('requirement ledger rejects missing, duplicate, unknown, source-substituted or candidate acceptance', () => {
  const { directory, source, record } = fixture();
  for (const mutate of [
    r => r.requirements.pop(), r => { r.requirements[13].id = 'R1'; },
    r => { r.requirements[13].id = 'R15'; }, r => { r.source_identity = 'b'.repeat(64); },
    r => { r.requirements[0].source_identity = 'b'.repeat(64); },
    r => { r.requirements[0].evidence[0].kind = 'candidate'; },
    r => { r.requirements[0].evidence[0].kind = 'inspected'; },
    r => { r.requirements[0].review = null; }, r => { r.requirements[0].measured_scope = ''; },
    r => { r.requirements[0].state = 'deferred'; }, r => { r.extra = true; },
    r => { r.requirements[0].extra = true; },
    r => { r.requirements[0].evidence.push(structuredClone(r.requirements[0].evidence[0])); },
  ]) {
    const changed = structuredClone(record); mutate(changed);
    assert.throws(() => admission.requireReleaseRequirements(changed, source, directory), /release requirement evidence/);
  }
});

test('requirement evidence checks retained bytes and rejects links and escaped paths', () => {
  const { directory, source, record } = fixture();
  symlinkSync('result.json', join(directory, 'evidence/symbolic.json'));
  assert.equal(admission.requireReleaseRequirements(record, source, directory).complete, true);
  for (const mutate of [
    r => { r.requirements[0].evidence[0].path = '../outside'; },
    r => { r.requirements[0].evidence[0].path = 'evidence/symbolic.json'; },
    r => { r.requirements[0].evidence[0].sha256 = 'b'.repeat(64); },
    r => { r.requirements[0].evidence[0].bytes++; },
    r => { r.requirements[0].evidence[0].bytes = 32 * 1024 * 1024 + 1; },
  ]) {
    const changed = structuredClone(record); mutate(changed);
    assert.throws(() => admission.requireReleaseRequirements(changed, source, directory), /release requirement evidence/);
  }
  linkSync(join(directory, 'evidence/review.json'), join(directory, 'evidence/hard.json'));
  assert.throws(() => admission.requireReleaseRequirements(record, source, directory), /release requirement evidence/);
});

test('requirement evidence detects later report mutation independently of link rejection', () => {
  const { directory, source, record } = fixture();
  assert.equal(admission.requireReleaseRequirements(record, source, directory).complete, true);
  writeFileSync(join(directory, 'evidence/result.json'), 'changed bytes');
  assert.throws(() => admission.requireReleaseRequirements(record, source, directory), /release requirement evidence/);
});
