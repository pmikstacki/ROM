// Finite owned group leader. No requested executable starts before the parent approves.
import { spawn } from 'node:child_process';
import { readProcessIdentity } from './launch.mjs';

const deadline = setTimeout(() => { process.exitCode = 78; process.disconnect?.(); }, 1000);
process.once('disconnect', () => { clearTimeout(deadline); });
process.once('message', request => {
  clearTimeout(deadline);
  if (!request || typeof request.executable !== 'string' || !Array.isArray(request.args) || !request.env) {
    process.exitCode = 78; process.disconnect(); return;
  }
  const child = spawn(request.executable, request.args, { stdio: ['ignore', 'inherit', 'inherit'], env: request.env });
  child.once('spawn', () => {
    const identity = readProcessIdentity(child.pid);
    if (identity) process.send({ type: 'target_identity', identity });
  });
  child.once('error', () => { process.exitCode = 127; process.disconnect(); });
  child.once('close', (code, signal) => {
    process.exitCode = code ?? (signal ? 128 : 127);
    if (process.connected) process.disconnect();
  });
});
