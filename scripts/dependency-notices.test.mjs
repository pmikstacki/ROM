import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { additionalNotices } from './dependency-notices.mjs';

test('offline supplements bind crate version, source revision and exact bytes', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-notice-test-'));
  try {
    const archive = join(root, 'archive');
    mkdirSync(archive);
    const revision = 'a'.repeat(40);
    const text = 'Fixture attribution and terms\n';
    const hash = createHash('sha256').update(text).digest('hex');
    const pkg = { name: 'fixture', version: '1.0.0', manifest_path: join(archive, 'Cargo.toml') };
    writeFileSync(join(archive, '.cargo_vcs_info.json'), JSON.stringify({ git: { sha1: revision } }));
    writeFileSync(join(root, 'notice.txt'), text);
    writeFileSync(join(root, 'manifest.json'), JSON.stringify({ archiveFiles: [], supplements: [{ name: pkg.name, version: pkg.version, revision, url: `https://example.invalid/${revision}/LICENSE`, sha256: hash, file: 'notice.txt', license: 'MIT', reason: 'fixture' }] }));
    assert.equal(additionalNotices(pkg, root)[0].text, text);
    assert.deepEqual(additionalNotices({ ...pkg, version: '2.0.0' }, root), []);
    writeFileSync(join(root, 'notice.txt'), `${text}tampered`);
    assert.throws(() => additionalNotices(pkg, root), /checksum mismatch/);
    writeFileSync(join(root, 'notice.txt'), text);
    writeFileSync(join(archive, '.cargo_vcs_info.json'), JSON.stringify({ git: { sha1: 'b'.repeat(40) } }));
    assert.throws(() => additionalNotices(pkg, root), /source revision mismatch/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('explicit archive paths retain AUTHORS and nested notices; missing paths fail', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-notice-test-'));
  try {
    const pkg = { name: 'fixture', version: '1.0.0', manifest_path: join(root, 'Cargo.toml') };
    writeFileSync(join(root, 'AUTHORS'), 'Attribution and license');
    writeFileSync(join(root, 'manifest.json'), JSON.stringify({ supplements: [], archiveFiles: [{ name: pkg.name, version: pkg.version, files: ['AUTHORS'] }] }));
    assert.equal(additionalNotices(pkg, root)[0].text, 'Attribution and license');
    rmSync(join(root, 'AUTHORS'));
    assert.throws(() => additionalNotices(pkg, root), /ENOENT/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
