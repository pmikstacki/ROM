import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, symlinkSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { reserveEvidence, requireFrozenLock, verifySource, requireProviderIdentity } from './admission.mjs';

const scratch = () => mkdtempSync(join(tmpdir(), 'rom-identity-admission-'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

test('evidence reservation creates an exclusive directory and preserves existing evidence', () => {
  const root = scratch(); const output = join(root, 'result');
  assert.equal(reserveEvidence(output), output);
  assert.ok(existsSync(output));
  writeFileSync(join(output, 'retained.json'), 'retained');
  assert.throws(() => reserveEvidence(output), /already exists/);
  assert.throws(() => reserveEvidence(root), /already exists/);
});

test('consumer lock admission compares the full graph and rejects bootstrap', () => {
  const frozen = { lockfileVersion: 3, packages: { '': { name: 'fixture' }, 'node_modules/example': { version: '1', integrity: 'sha512-x' } } };
  assert.doesNotThrow(() => requireFrozenLock(frozen, structuredClone(frozen), 'locked_npm_ci'));
  const drift = structuredClone(frozen); drift.packages['node_modules/example'].integrity = 'sha512-y';
  assert.throws(() => requireFrozenLock(frozen, drift, 'locked_npm_ci'), /drift/);
  assert.throws(() => requireFrozenLock(frozen, frozen, 'npm_install'), /locked/);
});

test('source fence reads admitted regular files and rejects changed files, traversal and symlinks', () => {
  const root = scratch(); writeFileSync(join(root, 'Cargo.toml'), 'candidate');
  const expected = { 'Cargo.toml': hash('candidate') };
  assert.deepEqual(verifySource(root, expected), expected);
  writeFileSync(join(root, 'Cargo.toml'), 'changed');
  assert.throws(() => verifySource(root, expected), /source mismatch/);
  assert.throws(() => verifySource(root, { '../outside': hash('candidate') }), /source path/);
  symlinkSync('Cargo.toml', join(root, 'shortcut'));
  assert.throws(() => verifySource(root, { shortcut: hash('changed') }), /symbolic link/);
  assert.throws(() => verifySource(root, {}), /manifest/);
});

test('provider admission requires exact image digest, platform, version, and source identity', () => {
  const provider = { provider: 'authentik', version: '2026.8.3', image: `ghcr.io/goauthentik/server@sha256:${'a'.repeat(64)}`, platform: 'linux/amd64', source_revision: 'b'.repeat(40) };
  assert.doesNotThrow(() => requireProviderIdentity(provider, structuredClone(provider)));
  for (const field of ['image', 'platform', 'version', 'source_revision']) {
    assert.throws(() => requireProviderIdentity(provider, { ...provider, [field]: 'unknown' }), /provider identity/);
  }
  assert.throws(() => requireProviderIdentity(provider, { ...provider, credentials: 'secret' }), /provider identity/);
});

test('source admission rejects unlisted files rather than fencing only a selected subset', () => {
  const root = scratch(); writeFileSync(join(root, 'Cargo.toml'), 'candidate'); writeFileSync(join(root, 'injected.rs'), 'extra');
  assert.throws(() => verifySource(root, { 'Cargo.toml': hash('candidate') }), /source inventory/);
});
