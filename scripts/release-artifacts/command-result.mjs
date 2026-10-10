// All release subprocesses share finite ownership and immutable result logs.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { runChild } from '../skills/process.mjs';
import { reportCommandEvidenceFailure } from './failure-diagnostics.mjs';
import { COMMAND_TIMEOUT_MS, COMMAND_LOG_BYTES } from './command-limits.mjs';

export const execute = (program, args, options) => runChild(program, args, { ...options, timeout: COMMAND_TIMEOUT_MS, maxBytes: COMMAND_LOG_BYTES });

export async function recordedCommand(program, args, root, stage, stem, runner = execute) {
  const started = new Date().toISOString();
  let outcome;
  try { outcome = await runner(program, args, { cwd: root, env: { ...process.env, CARGO_BUILD_JOBS: '2' } }); }
  catch (error) { outcome = { code: null, stdout: '', stderr: error.message, timedOut: false, spawn_failed: true }; }
  const stdout = `evidence/${stem}.stdout.log`, stderr = `evidence/${stem}.stderr.log`;
  const command = { program, args, cwd: root, started_at: started, finished_at: new Date().toISOString(), exit_code: outcome.code,
    bounded_abort: outcome.timedOut === true, spawn_failed: outcome.spawn_failed === true, stdout, stderr };
  try {
    writeFileSync(join(stage, stdout), outcome.stdout ?? '', { flag: 'wx' });
    writeFileSync(join(stage, stderr), outcome.stderr ?? '', { flag: 'wx' });
  } catch (error) {
    error.commandResult = { ...command, evidence_recorded: false };
    reportCommandEvidenceFailure(error.commandResult, outcome, error);
    throw error;
  }
  return command;
}
