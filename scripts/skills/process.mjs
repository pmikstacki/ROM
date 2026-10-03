// Trusted Unix examples own one process group, including compiler/test descendants.
import { spawn } from 'node:child_process';

export function runChild(program, args, options = {}) {
  if (process.platform !== 'linux') throw Error('skill examples require Linux process-group ownership');
  return new Promise((resolve, reject) => {
    const child = spawn(program, args, { cwd: options.cwd, env: options.env ?? process.env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', receivedBytes = 0, timedOut = false, settled = false;
    const timers = [];
    const signal = value => {
      if (!child.pid) return;
      try { process.kill(-child.pid, value); }
      catch (error) { if (error.code !== 'ESRCH') throw error; }
    };
    const settle = (code, error) => {
      if (settled) return;
      settled = true;
      for (const timer of timers) clearTimeout(timer);
      try { signal('SIGKILL'); } catch (failure) { error ??= failure; }
      child.stdout.destroy();
      child.stderr.destroy();
      child.unref();
      if (error) reject(error); else resolve({ code, stdout, stderr, timedOut });
    };
    const stop = () => {
      if (timedOut || settled) return;
      timedOut = true;
      try { signal('SIGTERM'); } catch (error) { settle(null, error); return; }
      timers.push(setTimeout(() => {
        try { signal('SIGKILL'); } catch (error) { settle(null, error); }
      }, 500));
      timers.push(setTimeout(() => settle(null), 2500));
    };
    timers.push(setTimeout(stop, options.timeout ?? 180000));
    const collect = (kind, bytes) => {
      receivedBytes += bytes.length;
      if (receivedBytes > (options.maxBytes ?? 1_048_576)) { stop(); return; }
      if (kind === 'stdout') stdout += bytes.toString(); else stderr += bytes.toString();
    };
    child.stdout.on('data', bytes => collect('stdout', bytes));
    child.stderr.on('data', bytes => collect('stderr', bytes));
    child.on('error', error => settle(null, error));
    child.on('close', code => settle(code));
  });
}
