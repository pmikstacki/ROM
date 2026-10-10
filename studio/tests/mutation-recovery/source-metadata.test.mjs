import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';

async function metadata() {
  const module = await import('./source-metadata.mjs').catch(() => ({}));
  assert.equal(typeof module.sourceCheckoutHead, 'function', 'source metadata must not require git for supplied release source');
  return module.sourceCheckoutHead;
}
function repository() {
  const root = mkdtempSync(join(tmpdir(), 'rom-recovery-source-meta-'));
  const git = args => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
  git(['init', '-q']); writeFileSync(join(root, 'source'), 'fixture'); git(['add', 'source']);
  git(['-c', 'user.name=ROM fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'source fixture']);
  return { root, git };
}
test('supplied archive and extracted source report no invented checkout revision without git metadata', async () => {
  const head = await metadata(), root = mkdtempSync(join(tmpdir(), 'rom-extracted-source-meta-'));
  assert.equal(head(root, 'release_artifact'), null);
  assert.equal(head(root, 'extracted_release_source'), null);
});
test('authoring evidence records only the exact fixture checkout revision', async () => {
  const head = await metadata(), f = repository();
  assert.equal(head(f.root, 'authoring_checkout'), f.git(['rev-parse', 'HEAD']));
  const child = join(f.root, 'extracted'); mkdirSync(child);
  assert.throws(() => head(child, 'authoring_checkout'), /checkout root/);
  assert.equal(head(child, 'extracted_release_source'), null);
});
test('source metadata rejects unknown modes instead of inventing artifact provenance', async () => {
  const head = await metadata(), root = mkdtempSync(join(tmpdir(), 'rom-invalid-source-meta-'));
  assert.throws(() => head(root, 'verified_production'), /source mode/);
});
