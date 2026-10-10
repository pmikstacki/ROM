import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { extract } from './archives.mjs';
import { produce } from './produce.mjs';
import { verifyArtifacts } from './verification.mjs';
import { fixture, passed, mutateManifest } from './test-support.mjs';

test('archive traversal or links are rejected before extraction', () => {
  for (const link of [false, true]) {
    const parent = mkdtempSync(join(tmpdir(), 'rom-artifact-malformed-'));
    mkdirSync(join(parent, 'input')); mkdirSync(join(parent, 'input', 'root'));
    if (link) symlinkSync('../../outside', join(parent, 'input', 'root', 'link'));
    else writeFileSync(join(parent, 'input', 'root', 'file'), 'value');
    const archive = join(parent, 'invalid.tar.gz');
    const args = ['-czf', archive, '-C', join(parent, 'input')];
    if (!link) args.push('--transform=s@root/file@root/../../outside@');
    execFileSync('tar', [...args, 'root']);
    assert.throws(() => extract(archive, parent, 'root'), /archive (path|entry)/);
  }
});

test('changed declared archive identity fails even with refreshed manifest checksum', async () => {
  const f = fixture(); await produce({ ...f, runner: passed });
  mutateManifest(f.output, manifest => { manifest.source.files[0].sha256 = '0'.repeat(64); });
  await assert.rejects(verifyArtifacts(f.output), /archive source identity/);
});

test('artifact verification rejects extra unchecksummed files', async () => {
  const f = fixture(); await produce({ ...f, runner: passed });
  writeFileSync(join(f.output, 'unexpected'), 'extra');
  await assert.rejects(verifyArtifacts(f.output), /incomplete artifact checksums/);
});

test('verification checks archive commit and complete gate metadata', async () => {
  for (const mutation of ['revision', 'tree', 'commands']) {
    const f = fixture(); await produce({ ...f, runner: passed });
    mutateManifest(f.output, manifest => {
      if (mutation === 'revision') manifest.source.revision = '0'.repeat(40);
      else if (mutation === 'tree') manifest.source.tree = '0'.repeat(40);
      else manifest.command_results.pop();
    });
    await assert.rejects(verifyArtifacts(f.output), /archive commit|archive source tree|complete verification gate/);
  }
});

test('verification notices unchecksummed cache directories', async () => {
  const f = fixture(); await produce({ ...f, runner: passed });
  mkdirSync(join(f.output, 'target')); writeFileSync(join(f.output, 'target', 'extra'), 'hidden');
  await assert.rejects(verifyArtifacts(f.output), /incomplete artifact checksums/);
});

test('verification binds complete declared profile and package version to the extracted profile', async () => {
  for (const mutation of ['format', 'feature', 'version']) {
    const f = fixture(); await produce({ ...f, runner: passed });
    mutateManifest(f.output, manifest => {
      if (mutation === 'format') manifest.source.profile.native_storage_format += 1;
      else if (mutation === 'feature') manifest.source.profile.features.pop();
      else manifest.source.package_version = '0.0.4';
    });
    await assert.rejects(verifyArtifacts(f.output), /archive release profile identity/);
  }
});

test('verification binds the complete declared selected file inventory', async () => {
  const f = fixture(); await produce({ ...f, runner: passed });
  mutateManifest(f.output, manifest => { manifest.source.selected.files.pop(); });
  await assert.rejects(verifyArtifacts(f.output), /archive selected source identity/);
});
