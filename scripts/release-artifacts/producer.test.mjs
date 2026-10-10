import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { produce } from './produce.mjs';
import { verifyArtifacts } from './verification.mjs';
import { fixture, git, passed } from './test-support.mjs';

test('dirty tracked or untracked source refuses gates and artifacts', async () => {
  for (const tracked of [true, false]) {
    const f = fixture();
    writeFileSync(join(f.root, tracked ? 'Cargo.lock' : 'unreviewed.txt'), 'dirty');
    let calls = 0;
    await assert.rejects(produce({ ...f, runner: async () => { calls++; return passed(); } }), /clean source/);
    assert.equal(calls, 0);
    assert.equal(existsSync(f.output), false);
  }
});

test('existing output retains its bytes and refuses gates', async () => {
  const f = fixture();
  mkdirSync(f.output);
  writeFileSync(join(f.output, 'keep'), 'original');
  let calls = 0;
  await assert.rejects(produce({ ...f, runner: async () => { calls++; return passed(); } }), /output exists/);
  assert.equal(calls, 0);
  assert.equal(readFileSync(join(f.output, 'keep'), 'utf8'), 'original');
});

test('committed Rust workspace version mismatch refuses every release gate', async () => {
  const f = fixture();
  const path = join(f.root, 'Cargo.toml');
  const input = readFileSync(path, 'utf8');
  const version = JSON.parse(readFileSync(join(f.root, 'extensions/native-alpha-v1.json'))).package_version;
  writeFileSync(path, input.replace(`version = "${version}"`, 'version = "9.9.9"'));
  git(f.root, ['add', '.']);
  git(f.root, ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'mismatched workspace']);
  let calls = 0;
  await assert.rejects(produce({ ...f, runner: async (...args) => { calls++; return passed(...args); } }), /workspace package version mismatch/);
  assert.equal(calls, 0);
  assert.equal(existsSync(f.output), false);
});

test('missing Git metadata or obsolete profile refuses gates', async () => {
  for (const profile of [false, true]) {
    const f = fixture();
    if (profile) {
      const path = join(f.root, 'extensions/native-alpha-v1.json');
      const document = JSON.parse(readFileSync(path)); document.profile_version = 0;
      writeFileSync(path, JSON.stringify(document));
      git(f.root, ['add', '.']); git(f.root, ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'obsolete']);
    } else {
      const { rmSync } = await import('node:fs'); rmSync(join(f.root, '.git'), { recursive: true });
    }
    let calls = 0;
    await assert.rejects(produce({ ...f, runner: async () => { calls++; return passed(); } }), /Git checkout|release profile/);
    assert.equal(calls, 0); assert.equal(existsSync(f.output), false);
  }
});

test('late destination collision retains complete losing stage and existing output', async () => {
  const f = fixture(); let calls = 0;
  await assert.rejects(produce({ ...f, runner: async (...command) => {
    if (++calls === 6) { mkdirSync(f.output); writeFileSync(join(f.output, 'keep'), 'winner'); }
    return passed(...command);
  } }));
  assert.equal(readFileSync(join(f.output, 'keep'), 'utf8'), 'winner');
  assert.deepEqual(readdirSync(f.output), ['keep']);
  const stage = readdirSync(f.parent).find(name => name.startsWith('.rom-release-stage-'));
  assert.ok(existsSync(join(f.parent, stage, 'manifest.json')));
  assert.equal(JSON.parse(readFileSync(join(f.parent, stage, 'failure.json'))).completed, false);
});

test('failed gate retains incomplete evidence and exposes no final output', async () => {
  const f = fixture();
  await assert.rejects(produce({ ...f, runner: async () => ({ code: 7, stdout: 'fixture', stderr: 'failure', timedOut: false }) }), /gate failed/);
  assert.equal(existsSync(f.output), false);
  const stages = readdirSync(f.parent).filter(name => name.startsWith('.rom-release-stage-'));
  assert.equal(stages.length, 1);
  const failure = JSON.parse(readFileSync(join(f.parent, stages[0], 'failure.json')));
  assert.equal(failure.completed, false);
  assert.equal(failure.command_results[0].exit_code, 7);
});

test('post-gate lock change or new committed revision prevents publication', async () => {
  for (const commit of [false, true]) {
    const f = fixture();
    let calls = 0;
    await assert.rejects(produce({ ...f, runner: async (...command) => {
      if (calls++ === 0) {
        writeFileSync(join(f.root, 'Cargo.lock'), 'changed');
        if (commit) { git(f.root, ['add', '.']); git(f.root, ['-c', 'user.name=fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'changed']); }
      }
      return passed(...command);
    } }), /source (changed|must remain clean)/);
    assert.equal(existsSync(f.output), false);
  }
});

test('successful fixture creates complete checksummed artifacts with matching extraction admission', async () => {
  const f = fixture();
  const commands = [];
  const result = await produce({ ...f, runner: async (program, args, options) => { commands.push([program, ...args]); return passed(program, args, options); } });
  assert.equal(commands.length, 10);
  assert.deepEqual(commands.map(command => command[0]), ['./scripts/check', './scripts/build', './demo/verify', './demo/verify-provider', './scripts/check-skills', 'node', './scripts/studio-browser-runtime-check', 'npm', 'npm', './demo/verify-studio']);
  assert.deepEqual(commands[5], ['node', 'scripts/check-packages.mjs', f.root]);
  assert.equal(result.publication_enabled, false);
  assert.equal(result.manifest_version, 2);
  assert.equal(result.verification_profile, 'rom-studio-v2');
  assert.equal(result.distribution, 'source-and-studio-assets');
  assert.equal(result.source.revision, git(f.root, ['rev-parse', 'HEAD']).trim());
  assert.deepEqual(result.archive_verification.workflows, ['resource', 'native', 'operator', 'release']);
  assert.equal(result.archive_verification.examples_executed, false);
  assert.equal(result.complete, true);
  assert.ok(existsSync(join(f.output, 'SHA256SUMS')));
  await verifyArtifacts(f.output);
  const archive = result.artifacts.source.path;
  writeFileSync(join(f.output, archive), 'tampered');
  await assert.rejects(verifyArtifacts(f.output), /checksum/);
});

test('current production version cannot finish the old eight gates without complete operational evidence', async () => {
 const f=fixture('0.1.0');let calls=0;
 await assert.rejects(produce({...f,runner:async(...args)=>{calls++;return passed(...args);}}),/release requirement evidence required/);
 assert.equal(calls,0);assert.equal(existsSync(f.output),false);
});
