import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { admitBrowserStorage } from './browser-storage.mjs';

const root = mkdtempSync(join(tmpdir(), 'rom-browser-storage-tests-'));
const home = `${root}/home`, temporary = `${root}/tmp`, profile = `${home}/profile`;
for (const path of [home, temporary, profile]) mkdirSync(path, { mode: 0o700 });

test('storage evidence comes from actual owned directories and device identity', () => {
  const evidence = admitBrowserStorage(root, { home, temporary, profile });
  assert.equal(evidence.device, statSync(root).dev);
  assert.equal(evidence.directories.profile.realpath, profile);
  assert.equal(evidence.directories.home.inode, statSync(home).ino);
});
test('sibling prefix and paths outside the hard run tree are rejected', () => {
  assert.throws(() => admitBrowserStorage(root, { home: `${root}-sibling`, temporary }), /browser storage/);
  assert.throws(() => admitBrowserStorage(root, { home: '/tmp', temporary }), /browser storage/);
});
test('symbolic links and non-directory storage are rejected', () => {
  const link = `${root}/link`, file = `${root}/file`;
  symlinkSync(home, link); writeFileSync(file, '', { flag: 'wx', mode: 0o600 });
  for (const path of [link, file]) assert.throws(() => admitBrowserStorage(root, { home: path, temporary }), /browser storage/);
});
test('an uncreated profile or symbolic-link root cannot authorize storage evidence', () => {
  assert.throws(() => admitBrowserStorage(root, { home, temporary, profile: `${home}/missing` }));
  const rootLink = `${root}/root-link`; symlinkSync(root, rootLink);
  assert.throws(() => admitBrowserStorage(rootLink, { home, temporary }), /browser storage/);
});
