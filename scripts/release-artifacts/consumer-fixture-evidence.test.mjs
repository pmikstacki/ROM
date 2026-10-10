import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { hash } from '../skills/files.mjs';
import * as evidence from './consumer-fixture-evidence.mjs';
import { recoveryFixtureEvidence } from './consumer-fixture.mjs';

function fixture() {
  const parent = mkdtempSync(join(tmpdir(), 'rom-fixture-identity-')), root = join(parent, 'source'), directory = join(parent, 'evidence');
  return { lease: { root, fence() {} }, directory, record: recoveryFixtureEvidence(root, directory) };
}

test('recovery fixture identity binds native source, consumer inputs, retained generated outputs and verifier', () => {
  const f = fixture();
  assert.equal(evidence.requireRecoveryFixture(f.lease, f.directory, f.record), f.record);
});

test('changed fixture identities or missing inventory entries fail admission', () => {
  for (const mutate of [
    r => { r.verifier_sha256 = '0'.repeat(64); },
    r => { r.fixture_source_sha256['tests/recovery-host/src/main.rs'] = '0'.repeat(64); },
    r => { delete r.fixture_source_sha256['tests/recovery-host/Cargo.toml']; },
    r => { delete r.consumer_fixture_sha256['consumer/dist/bundle.js']; },
    r => { r.consumer_fixture_sha256['consumer/extra'] = '0'.repeat(64); },
  ]) {
    const f = fixture(); mutate(f.record);
    assert.throws(() => evidence.requireRecoveryFixture(f.lease, f.directory, f.record), /fixture evidence/);
  }
});

test('a rewritten output inventory cannot hide substituted fixture input or runtime source', () => {
  for (const path of ['consumer/src/index.ts', 'runtime/proxy.mjs', 'consumer/package.json']) {
    const f = fixture(); writeFileSync(join(f.directory, path), 'substituted');
    f.record.consumer_fixture_sha256[path] = hash(join(f.directory, path));
    assert.throws(() => evidence.requireRecoveryFixture(f.lease, f.directory, f.record), /fixture evidence/);
  }
});

test('linked generated output and source lease drift are rejected', () => {
  const f = fixture(), path = join(f.directory, 'consumer/dist/bundle.js');
  rmSync(path); symlinkSync(join(f.directory, 'consumer/src/index.ts'), path);
  assert.throws(() => evidence.requireRecoveryFixture(f.lease, f.directory, f.record), /fixture evidence/);
  const fresh = fixture(); let calls = 0;
  fresh.lease.fence = () => { if (++calls === 2) throw Error('source drift'); };
  assert.throws(() => evidence.requireRecoveryFixture(fresh.lease, fresh.directory, fresh.record), /fixture evidence/);
});
