import test from 'node:test';
import assert from 'node:assert/strict';
import { validateStudioArchiveListing } from './archive-admission.mjs';
const entries = ['rom-studio-source/', 'rom-studio-source/package.json', 'rom-studio-source/package-lock.json', 'rom-studio-source/LICENSE', 'rom-studio-source/THIRD_PARTY_NOTICES.md', 'rom-studio-source/src/', 'rom-studio-source/src/controls.ts'];
const details = entries.map(path => `${path.endsWith('/') ? 'd' : '-'}rw-r--r-- 0/0 100 2026-10-07 00:00:00 ${path}`);

test('source archive admission rejects traversal, unknown members and links before extraction', () => {
  assert.doesNotThrow(() => validateStudioArchiveListing(entries, details));
  for (const path of ['rom-studio-source/../escape', '/absolute', 'rom-studio-source/src/./alias', 'rom-studio-source/src//alias', 'rom-studio-source/src/back\\slash', 'rom-studio-source/node_modules/unadmitted.js']) {
    assert.throws(() => validateStudioArchiveListing([...entries, path], [...details, `-rw-r--r-- 0/0 1 2026-10-07 00:00:00 ${path}`]), /archive/);
  }
  for (const type of ['l', 'h']) {
    const changed = [...details]; changed[1] = type + changed[1].slice(1);
    assert.throws(() => validateStudioArchiveListing(entries, changed), /archive entry/);
  }
});

test('source archive admission bounds entries and bytes and rejects duplicate members', () => {
  const large = [...details]; large[1] = large[1].replace(' 100 ', ' 67108865 ');
  assert.throws(() => validateStudioArchiveListing(entries, large), /archive byte/);
  assert.throws(() => validateStudioArchiveListing([...entries, entries[1]], [...details, details[1]]), /duplicate archive/);
  assert.throws(() => validateStudioArchiveListing(entries, details, { maxEntries: 6, maxBytes: 67108864 }), /archive member/);
});
