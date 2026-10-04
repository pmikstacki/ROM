import test from 'node:test';
import assert from 'node:assert/strict';
import { gates, requireCompleteGate } from './commands.mjs';

function results(commands, root) {
  return commands.map(([program, args]) => ({ program, args, cwd: root, exit_code: 0, bounded_abort: false, spawn_failed: false }));
}
test('Studio gate profile requires both source frontend and exact extracted host acceptance', () => {
  const root = '/accepted/source', assets = '/private/stage/extracted/studio';
  const commands = gates(root, assets);
  assert.equal(commands.length, 8);
  assert.deepEqual(commands[6], ['./scripts/studio-browser-runtime-check', []]);
  assert.deepEqual(commands[7], ['./demo/verify-studio', ['--assets-dir', assets]]);
  const accepted = results(commands, root);
  requireCompleteGate(accepted, 'rom-studio-v2', assets);
  for (const mutation of ['profile', 'assets', 'missing', 'failed']) {
    const changed = structuredClone(accepted);
    if (mutation === 'assets') changed[7].args[1] = '/other/assets';
    else if (mutation === 'missing') changed.splice(6, 1);
    else if (mutation === 'failed') changed[7].exit_code = 7;
    assert.throws(() => requireCompleteGate(changed, mutation === 'profile' ? 'unknown-profile' : 'rom-studio-v2', assets), /verification gate|profile/);
  }
});
test('historical v1 profile retains exactly the original six command identities', () => {
  const root = '/historical/source';
  const commands = [
    ['./scripts/check', []], ['./scripts/build', ['--release']], ['./demo/verify', []],
    ['./demo/verify-provider', []], ['./scripts/check-skills', []], ['node', ['scripts/check-packages.mjs', root]],
  ];
  requireCompleteGate(results(commands, root), 'native-source-v1');
  assert.throws(() => requireCompleteGate(results(commands, root), 'rom-studio-v2', '/extracted/assets'), /verification gate/);
});
