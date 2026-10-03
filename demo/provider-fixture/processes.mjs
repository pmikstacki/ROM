// Shared finite child-process ownership for development acceptance tests.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

export async function bounded(promise, ms = 5000) {
  let timeout;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timeout = setTimeout(() => reject(new Error('fixture child deadline')), ms);
    })]);
  } finally { clearTimeout(timeout); }
}

export function startProcess(executable, args) {
  const child = spawn(executable, args, { stdio: ['ignore', 'pipe', 'pipe'] });
  const done = once(child, 'close');
  // Keep early spawn errors observed until the owner awaits teardown.
  done.catch(() => {});
  let stdout = '';
  let stderr = '';
  let exceeded = false;
  for (const [stream, isError] of [[child.stdout, false], [child.stderr, true]]) {
    stream.on('data', bytes => {
      if (Buffer.byteLength(stdout) + Buffer.byteLength(stderr) + bytes.length > 64 * 1024) {
        exceeded = true;
        child.kill('SIGKILL');
      } else if (isError) stderr += bytes.toString();
      else stdout += bytes.toString();
    });
  }
  return { child, done, output: () => stdout + stderr, stdout: () => stdout,
    stderr: () => stderr, exceeded: () => exceeded };
}

export function startNode(file, args) {
  return startProcess(process.execPath, [new URL(file, import.meta.url).pathname, ...args]);
}

export async function stop(process) {
  if (process.child.exitCode === null && process.child.signalCode === null) process.child.kill('SIGTERM');
  try { await bounded(process.done); }
  catch {
    process.child.kill('SIGKILL');
    await bounded(process.done);
  }
}

export async function stopAll(processes) {
  const results = await Promise.allSettled(processes.map(process => stop(process)));
  if (results.some(result => result.status === 'rejected'))
    throw new Error('child cleanup failed');
}

export async function ready(file, process) {
  for (let i = 0; i < 300; i++) {
    try { return JSON.parse(await readFile(file, 'utf8')); } catch { /* Wait for complete publication. */ }
    if (process.child.exitCode !== null || process.child.signalCode !== null) throw new Error('provider exited before readiness');
    await delay(10);
  }
  throw new Error('provider readiness deadline');
}
