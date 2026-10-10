// The production gate is fixed; only the directly imported test API injects execution.
import { execute, recordedCommand } from './command-result.mjs';
import { isAbsolute, resolve, join } from 'node:path';
import { requireConsumerContext } from './consumer-context.mjs';
export { execute } from './command-result.mjs';

export const STUDIO_PROFILE = 'rom-studio-v2';
export const NATIVE_PROFILE = 'native-source-v1';
export const PUBLIC_CONSUMERS_PROFILE = 'rom-public-consumers-v3';
export function gates(root, assets, profile = STUDIO_PROFILE, context) {
  const native = [
    ['./scripts/check', []], ['./scripts/build', ['--release']], ['./demo/verify', []],
    ['./demo/verify-provider', []], ['./scripts/check-skills', []], ['node', ['scripts/check-packages.mjs', root]],
  ];
  if (profile === NATIVE_PROFILE) return native;
  if (![STUDIO_PROFILE, PUBLIC_CONSUMERS_PROFILE].includes(profile)) throw Error('unsupported verification profile');
  const studio = [...native, ['./scripts/studio-browser-runtime-check', []], ['./demo/verify-studio', ['--assets-dir', assets]]];
  if (profile === STUDIO_PROFILE) return studio;
  const input = requireConsumerContext(context, root);
  const extracted = input.extracted_root, evidence = input.evidence_root;
  return [...studio,
    ['node', [join(extracted, 'studio/tests/public-controls/verify.mjs'), join(evidence, 'controls'), '--source-dir', join(extracted, 'studio'), '--admit']],
    ...['sqlite', 'redb'].map((adapter, index) => ['node', [join(extracted, 'studio/tests/mutation-recovery/verify.mjs'), join(evidence, `recovery-${adapter}`), '--source-dir', join(extracted, 'studio'), '--adapter', adapter, '--host-binary', input.host_binary, '--port', String(43282 + index), '--admit']]),
  ];
}
export function gatePhase(root, assets, profile, context, phase) {
  const expected = gates(root, assets, profile, context);
  if (profile !== PUBLIC_CONSUMERS_PROFILE || !['prepare', 'assets', 'consumers'].includes(phase)) throw Error('invalid verification gate phase');
  if (phase === 'prepare') return expected.slice(0, 7);
  if (phase === 'assets') return expected.slice(7, 8);
  return expected.slice(8);
}
export async function verifyCommands(root, stage, results, runner = execute, assets, options = {}) {
  if (options.fence !== undefined && typeof options.fence !== 'function') throw Error('invalid verification fence');
  const profile = options.profile ?? STUDIO_PROFILE;
  const expected = gates(root, assets, profile, options.context);
  const selected = profile === PUBLIC_CONSUMERS_PROFILE
    ? gatePhase(root, assets, profile, options.context, options.phase)
    : profile === NATIVE_PROFILE ? expected
    : assets === undefined ? expected.slice(0, -1) : expected.slice(-1);
  for (const [program, args] of selected) {
    const index = results.length + 1;
    if (JSON.stringify([program, args]) !== JSON.stringify(expected[index - 1])) throw Error('out-of-order verification gate');
    options.fence?.();
    console.log(`Release gate ${index}/${expected.length}: ${program} ${args.join(' ')}`);
    const result = await recordedCommand(program, args, root, stage, `gate-${index}`, runner);
    results.push(result);
    options.fence?.();
    if (result.exit_code !== 0 || result.bounded_abort || result.spawn_failed) throw Error(`release gate failed: ${program}`);
  }
}
export function requireCompleteGate(results, profile = NATIVE_PROFILE, assets, context) {
  const root = results?.[0]?.cwd;
  if (typeof root !== 'string' || !root.startsWith('/') || !Array.isArray(results)) throw Error('incomplete verification gate');
  if ([STUDIO_PROFILE, PUBLIC_CONSUMERS_PROFILE].includes(profile) && (typeof assets !== 'string' || !isAbsolute(assets) || resolve(assets) !== assets)) throw Error('invalid verification gate assets');
  const expected = gates(root, assets, profile, context);
  if (results.length !== expected.length) throw Error('incomplete verification gate');
  if (results.some((result, index) => result.exit_code !== 0 || result.bounded_abort !== false || result.spawn_failed !== false || result.cwd !== root || JSON.stringify([result.program, result.args]) !== JSON.stringify(expected[index]))) throw Error('incomplete verification gate');
}
