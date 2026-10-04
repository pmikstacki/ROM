import test from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { produce } from './produce.mjs';
import { verifyArtifacts } from './verification.mjs';
import { contents } from './contents.mjs';
import { hash } from '../skills/files.mjs';
import { fixture, git, mutateManifest, passed } from './test-support.mjs';

test('legacy version-one source-only manifest retains its six-gate contract', async () => {
  const f = fixture(), result = await produce({ ...f, runner: passed });
  rmSync(join(f.output, result.artifacts.studio.path));
  delete result.artifacts.studio;
  delete result.archive_verification.studio;
  delete result.verification_profile;
  result.command_results = result.command_results.slice(0, 6);
  result.manifest_version = 1;
  result.distribution = 'source-only';
  writeFileSync(join(f.output, 'manifest.json'), JSON.stringify(result, null, 2) + '\n');
  writeFileSync(join(f.output, 'SHA256SUMS'), contents(f.output).filter(path => path !== 'SHA256SUMS')
    .map(path => `${hash(join(f.output, path))}  ${path}`).join('\n') + '\n');
  const admitted = await verifyArtifacts(f.output);
  assert.equal(admitted.manifest_version, 1);
  assert.equal(admitted.command_results.length, 6);
  assert.equal(admitted.archive_verification.studio, undefined);
});

test('frontend source, npm commands, asset inventory and profile are bound to their records', async () => {
  const f = fixture(); await produce({ ...f, runner: passed });
  const original = readFileSync(join(f.output, 'manifest.json'), 'utf8');
  const mutations = [
    document => { document.archive_verification.studio.source.package_lock_sha256 = '0'.repeat(64); },
    document => { document.archive_verification.studio.source.files = {}; },
    document => { document.archive_verification.studio.assets.files = {}; },
    document => { document.archive_verification.studio.assets.sha256 = '0'.repeat(64); },
    document => { document.archive_verification.studio.package_commands[0].args = ['install']; },
    document => { document.verification_profile = 'native-source-v1'; },
    document => {
      document.manifest_version = 1; document.distribution = 'source-only';
      document.command_results = document.command_results.slice(0, 6);
    },
  ];
  for (const mutate of mutations) {
    writeFileSync(join(f.output, 'manifest.json'), original);
    mutateManifest(f.output, mutate);
    await assert.rejects(verifyArtifacts(f.output));
  }
});

test('committed frontend generated caches are rejected before the first gate', async () => {
  for (const cache of ['dist', '.component-dist', '.demo-dist', 'test-results', 'playwright-report']) {
    const f = fixture(), directory = join(f.root, 'studio', cache);
    mkdirSync(directory); writeFileSync(join(directory, 'generated.js'), 'generated');
    git(f.root, ['add', '-f', '.']);
    git(f.root, ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'generated cache']);
    let calls = 0;
    await assert.rejects(produce({ ...f, runner: async (...command) => { calls++; return passed(...command); } }), /excluded source path/);
    assert.equal(calls, 0);
  }
});

test('incompatible frontend release version is rejected before gates or npm execution', async () => {
  const f = fixture();
  for (const name of ['package.json', 'package-lock.json']) {
    const path = join(f.root, 'studio', name), document = JSON.parse(readFileSync(path));
    document.version = '9.9.9';
    if (document.packages) document.packages[''].version = '9.9.9';
    writeFileSync(path, JSON.stringify(document));
  }
  git(f.root, ['add', '.']);
  git(f.root, ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'version mismatch']);
  let calls = 0;
  await assert.rejects(produce({ ...f, runner: async (...command) => { calls++; return passed(...command); } }), /Studio release version/);
  assert.equal(calls, 0);
});

test('host gate must consume unchanged extracted assets and succeed before publication', async () => {
  for (const failure of ['changed-assets', 'host-failure']) {
    const f = fixture();
    await assert.rejects(produce({ ...f, runner: async (program, args, options) => {
      if (program === './demo/verify-studio') {
        if (failure === 'host-failure') return { code: 8, stdout: '', stderr: 'fixture host failure', timedOut: false };
        writeFileSync(join(args[1], 'assets/main.js'), 'changed during acceptance');
      }
      return passed(program, args, options);
    } }), /Studio asset identity|release gate failed/);
  }
});
