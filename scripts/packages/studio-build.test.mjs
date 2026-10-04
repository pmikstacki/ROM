import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { frontendFixture, fixtureRunner } from './studio-test-support.mjs';
import { prepareStudioAssets } from './studio-build.mjs';
import { requireStudioAssets } from './studio-assets.mjs';

function stage() {
  const directory = mkdtempSync(join(tmpdir(), 'rom-studio-build-test-'));
  mkdirSync(join(directory, 'evidence'));
  return directory;
}
test('fresh offline build produces an exclusively created immutable asset extraction', async () => {
  const f = frontendFixture(), directory = stage(), commands = [];
  try {
    const artifact = { path: 'studio.tar.gz', prefix: 'studio-assets' };
    const prepared = await prepareStudioAssets(f.root, directory, artifact, async (program, args, options) => {
      commands.push([program, args, options.cwd]);
      assert.notEqual(options.cwd, join(f.root, 'studio'));
      return fixtureRunner(program, args, options);
    });
    assert.deepEqual(commands.map(([program, args]) => [program, args]), [
      ['npm', ['ci', '--offline', '--no-audit', '--no-fund']], ['npm', ['run', 'build']],
    ]);
    assert.equal(prepared.source.package_version, '0.0.2');
    assert.equal(prepared.package_commands.length, 2);
    assert.ok(existsSync(join(directory, artifact.path)));
    requireStudioAssets(prepared.asset_directory, prepared.assets);
    assert.equal(existsSync(join(prepared.asset_directory, 'node_modules')), false);
    assert.equal(existsSync(join(f.root, 'studio/dist')), false);
    prepared.cleanup();
    assert.equal(existsSync(prepared.asset_directory), false);
  } finally { rmSync(f.root, { recursive: true, force: true }); rmSync(directory, { recursive: true, force: true }); }
});
test('npm failures and changed copied lock prevent acceptance and retain command evidence', async () => {
  for (const mutation of ['failure', 'lock']) {
    const f = frontendFixture(), directory = stage();
    try {
      await assert.rejects(prepareStudioAssets(f.root, directory, { path: 'studio.tar.gz', prefix: 'studio-assets' }, async (program, args, options) => {
        if (mutation === 'failure') return { code: 7, stdout: 'fixture failure', stderr: 'failed', timedOut: false };
        if (args[0] === 'ci') writeFileSync(join(options.cwd, 'package-lock.json'), 'changed');
        return fixtureRunner(program, args, options);
      }), /Studio (build|source)/);
      assert.ok(existsSync(join(directory, 'evidence/studio-npm-1.stdout.log')));
      if (mutation === 'failure') assert.equal(readFileSync(join(directory, 'evidence/studio-npm-1.stdout.log'), 'utf8'), 'fixture failure');
    } finally { rmSync(f.root, { recursive: true, force: true }); rmSync(directory, { recursive: true, force: true }); }
  }
});
