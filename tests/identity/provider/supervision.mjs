// Register only children launched in separately owned process groups.
// This module does not launch a provider, native build, daemon or browser.
export class ProcessRegistry {
  #terminate; #max; #outputLimit; #bytes = 0; #entries = new Map(); #closed = false;
  constructor({ terminate = (pid, signal) => process.kill(-pid, signal), maxProcesses = 6, outputBytes = 32 * 1024 * 1024 } = {}) {
    if (typeof terminate !== 'function' || !Number.isSafeInteger(maxProcesses) || maxProcesses < 1 || maxProcesses > 32 ||
        !Number.isSafeInteger(outputBytes) || outputBytes < 1 || outputBytes > 32 * 1024 * 1024) throw Error('invalid supervision limits');
    this.#terminate = terminate; this.#max = maxProcesses; this.#outputLimit = outputBytes;
  }
  admit({ timeoutMs, graceMs = 5000, drainMs = 5000, outputBytes = 32 * 1024 * 1024 }) {
    if (this.#closed || this.#entries.size >= 32 || [...this.#entries.values()].filter(entry => !entry.done).length >= this.#max) throw Error('invalid process identity');
    if (![timeoutMs, graceMs, drainMs, outputBytes].every(n => Number.isSafeInteger(n) && n > 0) || timeoutMs > 1_200_000 || graceMs > 5000 || drainMs > 5000 || outputBytes > 32 * 1024 * 1024) throw Error('invalid supervision limits');
  }
  register(child, { timeoutMs, graceMs = 5000, drainMs = 5000, outputBytes = 32 * 1024 * 1024 }) {
    this.admit({ timeoutMs, graceMs, drainMs, outputBytes });
    if (this.#closed || !Number.isSafeInteger(child?.pid) || child.pid <= 1 || this.#entries.has(child.pid) ||
        this.#entries.size >= 32 || [...this.#entries.values()].filter(entry => !entry.done).length >= this.#max ||
        typeof child.once !== 'function' || typeof child.stdout?.on !== 'function' || typeof child.stderr?.on !== 'function') throw Error('invalid process identity');
    let complete; const promise = new Promise(resolve => { complete = resolve; });
    const entry = { promise, stop: undefined, done: false }; this.#entries.set(child.pid, entry);
    let reason = 'completed'; let bytes = 0; let done = false; let force; let drain;
    const deadline = setTimeout(() => stop('deadline'), timeoutMs);
    const signal = name => { try { this.#terminate(child.pid, name); } catch { /* The close or finite drain still decides the outcome. */ } };
    const finish = (exitCode, exitSignal, drained) => {
      if (done) return; done = true; entry.done = true;
      clearTimeout(deadline); clearTimeout(force); clearTimeout(drain);
      child.stdout.removeListener('data', output); child.stderr.removeListener('data', output);
      complete({ pid: child.pid, exit_code: exitCode, signal: exitSignal, reason, drained });
    };
    const stop = category => {
      if (done || reason !== 'completed') return;
      reason = category; clearTimeout(deadline); signal('SIGTERM');
      if (done) return;
      force = setTimeout(() => {
        signal('SIGKILL');
        if (!done) drain = setTimeout(() => finish(null, null, false), drainMs);
      }, graceMs);
    };
    const output = chunk => {
      const count = Buffer.byteLength(chunk); bytes += count; this.#bytes += count;
      if (this.#bytes > this.#outputLimit) {
        this.#closed = true;
        for (const owned of this.#entries.values()) owned.stop('output-limit');
      } else if (bytes > outputBytes) stop('output-limit');
    };
    child.stdout.on('data', output); child.stderr.on('data', output);
    child.once('close', (code, exitSignal) => finish(code, exitSignal, true));
    child.once('error', () => stop('process-error'));
    entry.stop = (category = 'shutdown') => stop(category);
    return promise;
  }
  async drain() { return Promise.all([...this.#entries.values()].map(entry => entry.promise)); }
  stop(pid) {
    const entry = this.#entries.get(pid);
    if (!entry) throw Error('unowned process');
    entry.stop();
  }
  async shutdown() {
    this.#closed = true;
    for (const entry of this.#entries.values()) entry.stop();
    return this.drain();
  }
}
