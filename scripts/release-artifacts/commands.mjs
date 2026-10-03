// The production gate is fixed; only the directly imported test API injects execution.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { runChild } from '../skills/process.mjs';

export function gates(root) {
  return [
    ['./scripts/check', []], ['./scripts/build', ['--release']], ['./demo/verify', []],
    ['./demo/verify-provider', []], ['./scripts/check-skills', []], ['node', ['scripts/check-packages.mjs', root]],
  ];
}
export const execute = (program, args, options) => runChild(program, args, { ...options, timeout: 3_600_000, maxBytes: 32 * 1024 * 1024 });
export async function verifyCommands(root, stage, results, runner = execute) {
  for (const [program, args] of gates(root)) {
    const index = results.length + 1;
    console.log(`Release gate ${index}/6: ${program} ${args.join(' ')}`);
    const started = new Date().toISOString();
    let outcome;
    try { outcome = await runner(program, args, { cwd: root, env: { ...process.env, CARGO_BUILD_JOBS: '2' } }); }
    catch (error) { outcome = { code: null, stdout: '', stderr: error.message, timedOut: false, spawn_failed: true }; }
    const stdout = `evidence/gate-${index}.stdout.log`, stderr = `evidence/gate-${index}.stderr.log`;
    writeFileSync(join(stage, stdout), outcome.stdout ?? '', { flag: 'wx' });
    writeFileSync(join(stage, stderr), outcome.stderr ?? '', { flag: 'wx' });
    results.push({ program, args, cwd: root, started_at: started, finished_at: new Date().toISOString(), exit_code: outcome.code, bounded_abort: outcome.timedOut === true, spawn_failed: outcome.spawn_failed === true, stdout, stderr });
    if (outcome.code !== 0 || outcome.timedOut) throw Error(`release gate failed: ${program}`);
  }
}
export function requireCompleteGate(results) {
  const root = results?.[0]?.cwd;
  if (typeof root !== 'string' || !root.startsWith('/') || !Array.isArray(results) || results.length !== 6) throw Error('incomplete verification gate');
  const expected = gates(root);
  if (results.some((result, index) => result.exit_code !== 0 || result.bounded_abort !== false || result.spawn_failed !== false || result.cwd !== root || JSON.stringify([result.program, result.args]) !== JSON.stringify(expected[index]))) throw Error('incomplete verification gate');
}
