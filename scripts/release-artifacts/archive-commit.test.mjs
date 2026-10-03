import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { archiveCommit, sourceArchive } from './archives.mjs';
import { fixture, git } from './test-support.mjs';

test('commit extraction consumes only the Git header from a large source archive', () => {
  const f = fixture();
  writeFileSync(join(f.root, 'large.txt'), Buffer.alloc(8 * 1024 * 1024, 0x61));
  git(f.root, ['add', 'large.txt']);
  git(f.root, ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'large archive']);
  const revision = git(f.root, ['rev-parse', 'HEAD']).trim();
  const archive = join(f.parent, 'large-source.tar.gz');
  sourceArchive(f.root, revision, 'large-source', archive);
  assert.equal(archiveCommit(archive), revision);
});

test('commit extraction rejects a tree archive with no commit marker', () => {
  const f = fixture();
  const archive = join(f.parent, 'tree-source.tar.gz');
  sourceArchive(f.root, git(f.root, ['rev-parse', 'HEAD^{tree}']).trim(), 'tree-source', archive);
  assert.throws(() => archiveCommit(archive));
});

test('commit extraction still rejects incomplete gzip content', () => {
  const f = fixture();
  const archive = join(f.parent, 'truncated-source.tar.gz');
  sourceArchive(f.root, git(f.root, ['rev-parse', 'HEAD']).trim(), 'truncated-source', archive);
  const gzip = readFileSync(archive);
  writeFileSync(archive, gzip.subarray(0, gzip.length - 8));
  assert.throws(() => archiveCommit(archive));
});
