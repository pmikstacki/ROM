// The production gate is fixed; only the directly imported test API injects execution.
import { execute, recordedCommand } from './command-result.mjs';
import { isAbsolute, resolve } from 'node:path';
export { execute } from './command-result.mjs';

export const STUDIO_PROFILE = 'rom-studio-v2';
export const NATIVE_PROFILE = 'native-source-v1';
export function gates(root, assets, profile = STUDIO_PROFILE) {
  const native = [
    ['./scripts/check', []], ['./scripts/build', ['--release']], ['./demo/verify', []],
    ['./demo/verify-provider', []], ['./scripts/check-skills', []], ['node', ['scripts/check-packages.mjs', root]],
  ];
  if (profile === NATIVE_PROFILE) return native;
  if (profile !== STUDIO_PROFILE) throw Error('unsupported verification profile');
  return [...native, ['./scripts/studio-browser-runtime-check', []], ['./demo/verify-studio', ['--assets-dir', assets]]];
}
export async function verifyCommands(root, stage, results, runner = execute, assets) {
  const expected = gates(root, assets);
  const selected = assets === undefined ? expected.slice(0, -1) : expected.slice(-1);
  for (const [program, args] of selected) {
    const index = results.length + 1;
    if (JSON.stringify([program, args]) !== JSON.stringify(expected[index - 1])) throw Error('out-of-order verification gate');
    console.log(`Release gate ${index}/${expected.length}: ${program} ${args.join(' ')}`);
    const result = await recordedCommand(program, args, root, stage, `gate-${index}`, runner);
    results.push(result);
    if (result.exit_code !== 0 || result.bounded_abort || result.spawn_failed) throw Error(`release gate failed: ${program}`);
  }
}
export function requireCompleteGate(results, profile = NATIVE_PROFILE, assets) {
  const root = results?.[0]?.cwd;
  if (typeof root !== 'string' || !root.startsWith('/') || !Array.isArray(results)) throw Error('incomplete verification gate');
  if (profile === STUDIO_PROFILE && (typeof assets !== 'string' || !isAbsolute(assets) || resolve(assets) !== assets)) throw Error('invalid verification gate assets');
  const expected = gates(root, assets, profile);
  if (results.length !== expected.length) throw Error('incomplete verification gate');
  if (results.some((result, index) => result.exit_code !== 0 || result.bounded_abort !== false || result.spawn_failed !== false || result.cwd !== root || JSON.stringify([result.program, result.args]) !== JSON.stringify(expected[index]))) throw Error('incomplete verification gate');
}
