import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, chmodSync, symlinkSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { readNativeTlsFile } from './native-tls-io.mjs';

test('private inputs reject oversized, public-permission and symlink files', () => {
  const root = mkdtempSync(`${tmpdir()}/rom-native-tls-io-`);
  try {
    const path = `${root}/private.json`; writeFileSync(path, 'abc', { mode: 0o600 });
    assert.equal(readNativeTlsFile(path, 3, true).toString(), 'abc');
    assert.throws(() => readNativeTlsFile(path, 2, true));
    chmodSync(path, 0o644); assert.throws(() => readNativeTlsFile(path, 3, true));
    chmodSync(path, 0o600); symlinkSync(path, `${root}/link`);
    assert.throws(() => readNativeTlsFile(`${root}/link`, 3, true));
  } finally { rmSync(root, { recursive: true }); }
});
