import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { produce } from './produce.mjs';
import { fixture, passed } from './test-support.mjs';
import { withVerifiedExtraction } from './consumer-source.mjs';

test('verified extraction remains available through an async consumer and is removed only afterwards', async () => {
  const f = fixture(), manifest = await produce({ ...f, runner: passed });
  let extracted;
  const value = await withVerifiedExtraction(f.output, manifest.source, manifest.artifacts, manifest.archive_verification.studio, async lease => {
    extracted = lease.root;
    assert.equal(existsSync(join(extracted, 'Cargo.lock')), true);
    assert.equal(lease.witness.revision, manifest.source.revision);
    assert.equal(lease.witness.archive_sha256, manifest.artifacts.source.sha256);
    assert.equal(lease.witness.lock_sha256, manifest.source.lock_sha256);
    await new Promise(resolve => setImmediate(resolve));
    lease.fence();
    assert.equal(existsSync(extracted), true);
    return 'consumer-completed';
  });
  assert.equal(value, 'consumer-completed');
  assert.equal(existsSync(extracted), false);
});

test('no consumer executes when source tree identity is not verified', async () => {
  const f = fixture(), manifest = await produce({ ...f, runner: passed });
  const changed = structuredClone(manifest.source); changed.tree = '0'.repeat(changed.tree.length);
  let calls = 0;
  await assert.rejects(withVerifiedExtraction(f.output, changed, manifest.artifacts, manifest.archive_verification.studio, () => { calls++; }), /tree identity/);
  assert.equal(calls, 0);
});

test('consumer extraction requires matching payload digests before executing any fixture', async () => {
  const f = fixture(), manifest = await produce({ ...f, runner: passed });
  for (const mutation of ['missing', 'changed']) {
    const artifacts = structuredClone(manifest.artifacts);
    if (mutation === 'missing') delete artifacts.source.sha256;
    else artifacts.source.sha256 = '0'.repeat(64);
    let calls = 0;
    await assert.rejects(withVerifiedExtraction(f.output, manifest.source, artifacts, manifest.archive_verification.studio, () => { calls++; }), /payload digest/);
    assert.equal(calls, 0);
  }
});

test('a changed extracted source or lock cannot return successful consumer evidence', async () => {
  for (const mutation of ['lock', 'extra']) {
    const f = fixture(), manifest = await produce({ ...f, runner: passed });
    let extracted;
    await assert.rejects(withVerifiedExtraction(f.output, manifest.source, manifest.artifacts, manifest.archive_verification.studio, async lease => {
      extracted = lease.root;
      writeFileSync(join(extracted, mutation === 'lock' ? 'Cargo.lock' : 'new-file'), 'changed');
      return { passed: true };
    }), /source .*identity|source fingerprint/);
    assert.equal(existsSync(extracted), false);
  }
});

test('archive digest changes and callback failures cannot leave a usable extraction lease', async () => {
  const f = fixture(), manifest = await produce({ ...f, runner: passed });
  let extracted;
  const archive = join(f.output, manifest.artifacts.source.path), original = readFileSync(archive);
  await assert.rejects(withVerifiedExtraction(f.output, manifest.source, manifest.artifacts, manifest.archive_verification.studio, lease => {
    extracted = lease.root; writeFileSync(archive, Buffer.concat([original, Buffer.from('changed')]));
  }), /archive fingerprint/);
  assert.equal(existsSync(extracted), false);
  writeFileSync(archive, original);
  await assert.rejects(withVerifiedExtraction(f.output, manifest.source, manifest.artifacts, manifest.archive_verification.studio, async lease => {
    extracted = lease.root; throw Error('consumer failed');
  }), /consumer failed/);
  assert.equal(existsSync(extracted), false);
});
