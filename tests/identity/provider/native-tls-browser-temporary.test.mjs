import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, realpathSync, renameSync, mkdirSync, chmodSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as preparation from './native-tls-prepare.mjs';
import * as storage from './native-tls-storage.mjs';
const fixture = () => mkdtempSync(realpathSync(tmpdir()) + '/rom-native-tmp-');

test('native temporary admission rejects both actual overlong failed paths', () => {
  for (const path of ['/var/tmp/rom-010-authentik-20261007/run/volume/private/native-tls-e0cacd5c927ce40a007ac798/tmp', '/var/tmp/rom-010-authentik-20261007/run/volume/private/native-tls-browser-launch-ab19320fd80025e4a31a2bac/tmp']) {
    assert.throws(() => preparation.requireNativeTlsBrowserTemporary({ path }), /browser TMPDIR exceeds Unix socket path budget/);
  }
});

test('fresh short directory is private, canonical and identity-fenced', () => {
  const root = fixture(), witness = preparation.createNativeTlsBrowserTemporary(root);
  assert.match(witness.path, new RegExp('^' + root + '/t-[a-f0-9]{8}$'));
  assert.equal(witness.mode, 0o700);
  assert.equal(witness.uid, process.getuid());
  assert.equal(realpathSync(witness.path), witness.path);
  assert.doesNotThrow(() => preparation.requireNativeTlsBrowserTemporary(witness, root));
});

test('same-path directory replacement cannot pass the original identity fence', () => {
  const root = fixture(), witness = preparation.createNativeTlsBrowserTemporary(root);
  renameSync(witness.path, witness.path + '-preserved'); mkdirSync(witness.path, { mode: 0o700 });
  assert.throws(() => preparation.requireNativeTlsBrowserTemporary(witness, root), /temporary identity changed/);
});

test('public mode and symlink substitution are rejected', () => {
  const root = fixture(), witness = preparation.createNativeTlsBrowserTemporary(root);
  chmodSync(witness.path, 0o755);
  assert.throws(() => preparation.requireNativeTlsBrowserTemporary(witness, root), /temporary identity changed/);
  chmodSync(witness.path, 0o700); renameSync(witness.path, witness.path + '-preserved');
  symlinkSync(witness.path + '-preserved', witness.path);
  assert.throws(() => preparation.requireNativeTlsBrowserTemporary(witness, root), /temporary identity changed/);
});

test('shared allocation walker retains link and budget rejection', () => {
  const root = fixture(); writeFileSync(root + '/data', 'payload');
  assert.throws(() => storage.nativeTlsOwnedDirectoryAllocation(root, 1), /planned budget/);
  assert.throws(() => storage.nativeTlsOwnedDirectoryAllocation('/dev/null'), /unexpected file type/);
  symlinkSync('data', root + '/link');
  assert.throws(() => storage.nativeTlsOwnedDirectoryAllocation(root), /unexpected file type/);
});

test('private wrapper and combined case-plus-temporary budget remain enforced', () => {
  assert.throws(() => storage.nativeTlsAllocation(fixture()), /owned native TLS private path/);
  const caseAllocation = { allocated_bytes: 600, logical_bytes: 400 }, temporaryAllocation = { allocated_bytes: 500, logical_bytes: 400 };
  assert.throws(() => storage.requireNativeTlsCombinedAllocation(caseAllocation, temporaryAllocation, 1000), /combined native TLS allocation/);
  assert.deepEqual(storage.requireNativeTlsCombinedAllocation(caseAllocation, temporaryAllocation, 1100), { allocated_bytes: 1100, logical_bytes: 800, planned_budget_bytes: 1100, aggregate_hard_quota: false });
});
