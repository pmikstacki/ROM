import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { frontendFixture, fixtureRunner } from './studio-test-support.mjs';
import { verifyStudioPackage } from './studio-consumer.mjs';

test('packaged consumer must build locked inputs and run the actual-host command on the extraction', async () => {
  const f = frontendFixture(), parent = mkdtempSync(join(tmpdir(), 'rom-studio-consumer-test-')), output = join(parent, 'package');
  try {
    const commands = [];
    const result = await verifyStudioPackage({ root: f.root, output, runner: async (program, args, options) => {
      commands.push([program, args]);
      return fixtureRunner(program, args, options);
    } });
    assert.deepEqual(commands.slice(0, 2), [['npm', ['ci', '--offline', '--no-audit', '--no-fund']], ['npm', ['run', 'build']]]);
    assert.deepEqual(commands[2], ['./demo/verify-studio', ['--assets-dir', result.asset_directory]]);
    assert.equal(result.completed, true);
    assert.equal(existsSync(join(output, 'studio-assets.tar.gz')), true);
    assert.equal(existsSync(result.asset_directory), false);
    assert.equal(JSON.parse(readFileSync(join(output, 'studio-package.json'))).completed, true);
    let calls = 0;
    await assert.rejects(verifyStudioPackage({ root: f.root, output, runner: async () => { calls++; } }), /EEXIST/);
    assert.equal(calls, 0);
  } finally { rmSync(parent, { recursive: true, force: true }); rmSync(f.root, { recursive: true, force: true }); }
});

test('packaged consumer retains incomplete evidence if host fails or changes accepted assets', async () => {
  for (const kind of ['host-failure', 'changed-assets']) {
    const f = frontendFixture(), parent = mkdtempSync(join(tmpdir(), 'rom-studio-consumer-negative-')), output = join(parent, 'package');
    try {
      await assert.rejects(verifyStudioPackage({ root: f.root, output, runner: async (program, args, options) => {
        if (program === './demo/verify-studio') {
          if (kind === 'host-failure') return { code: 8, stdout: '', stderr: 'fixture host failure', timedOut: false };
          writeFileSync(join(args[1], 'assets/main.js'), 'changed');
        }
        return fixtureRunner(program, args, options);
      } }), /Studio (host acceptance|asset identity)/);
      assert.equal(JSON.parse(readFileSync(join(output, 'studio-package-failure.json'))).completed, false);
      assert.equal(existsSync(join(output, 'studio-package.json')), false);
    } finally { rmSync(parent, { recursive: true, force: true }); rmSync(f.root, { recursive: true, force: true }); }
  }
});
