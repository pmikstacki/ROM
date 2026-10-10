// Linux process ownership. Never signal a bare PID supplied by a caller.
import { spawn } from 'node:child_process';
import { readFileSync, accessSync, constants } from 'node:fs';
import { isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ProcessRegistry } from './supervision.mjs';

export function readProcessIdentity(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 1) return null;
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    const fields = stat.slice(stat.lastIndexOf(')') + 2).trim().split(/\s+/);
    if (!/^\d+$/.test(fields[19])) return null;
    return Object.freeze({ pid, group: Number(fields[2]), session: Number(fields[3]), started: fields[19],
      boot_id: readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim() });
  } catch { return null; }
}

export function sameProcessIdentity(expected, actual) {
  return actual !== null && ['pid', 'group', 'session', 'started', 'boot_id'].every(key => expected[key] === actual[key]);
}

export class OwnedLauncher {
  #identities = new Map(); #read; #send; #registry;
  constructor({ readIdentity = readProcessIdentity, sendSignal = (group, signal) => process.kill(-group, signal), ...limits } = {}) {
    this.#read = readIdentity; this.#send = sendSignal;
    this.#registry = new ProcessRegistry({ ...limits, terminate: (pid, signal) => {
      const expected = this.#identities.get(pid);
      const actual = this.#read(pid);
      if (!expected || expected.group !== pid || expected.session !== pid || !sameProcessIdentity(expected, actual)) throw Error('owned process identity changed');
      this.#send(pid, signal);
    } });
  }
  launch(executable, args, options) {
    this.#registry.admit(options);
    if (options.admitIdentity !== undefined && typeof options.admitIdentity !== 'function') throw Error('invalid resource admission');
    if (!isAbsolute(executable) || !Array.isArray(args) || args.length > 64 || args.some(arg => typeof arg !== 'string' || arg.length > 4096)) throw Error('invalid owned command');
    accessSync(executable, constants.X_OK);
    // The finite wrapper waits for approval. The target cannot execute before ownership is established.
    const child = spawn(process.execPath, [fileURLToPath(new URL('./child-runner.mjs', import.meta.url))], {
      detached: true, stdio: ['ignore', 'pipe', 'pipe', 'ipc'], cwd: options.cwd,
      env: { PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC' },
    });
    // Attach error/close tracking synchronously, before the event loop advances.
    let close; const physicalClose = new Promise(resolve => { close = resolve; });
    child.once('close', (code, signal) => close({ code, signal }));
    child.on('error', () => {}); // Registry reports process-error; never expose an upstream error string.
    const identity = this.#read(child.pid);
    if (!identity || identity.group !== child.pid || identity.session !== child.pid) {
      child.disconnect(); // The wrapper exits without starting a target. No unowned PID is signalled.
      throw Error('cannot establish owned process identity');
    }
    this.#identities.set(child.pid, Object.freeze({ ...identity }));
    const closed = this.#registry.register(child, options);
    try {
      options.admitIdentity?.(this.#identities.get(child.pid));
      if (!sameProcessIdentity(identity, this.#read(child.pid))) throw Error('resource admission identity changed');
    } catch (error) {
      child.disconnect(); // A rejected wrapper exits before it can launch a command.
      throw error;
    }
    let started; const targetStarted = new Promise(resolve => { started = resolve; });
    child.on('message', message => {
      if (message?.type === 'target_identity') {
        const current = readProcessIdentity(message.identity?.pid);
        if (current && current.group === identity.group && current.session === identity.session && sameProcessIdentity(message.identity, current)) started(Object.freeze({ ...current }));
      }
    });
    child.once('close', () => started(null));
    child.send({ executable, args, env: options.env ?? { PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC' } }, error => {
      if (error) this.#registry.stop(child.pid);
    });
    return { child, identity: this.#identities.get(child.pid), closed, physicalClose, targetStarted };
  }
  stop(pid) { this.#registry.stop(pid); }
  shutdown() { return this.#registry.shutdown(); }
  drain() { return this.#registry.drain(); }
}
