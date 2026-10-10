import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { ProcessRegistry } from './supervision.mjs';

function child(pid) {
  return Object.assign(new EventEmitter(), { pid, stdout: new EventEmitter(), stderr: new EventEmitter() });
}
const options = { timeoutMs: 100, graceMs: 5, drainMs: 5, outputBytes: 20 };

test('supervision reports actual close and rejects invalid or duplicate process identities', async () => {
  const registry = new ProcessRegistry({ terminate: () => {}, maxProcesses: 2 });
  const owned = child(100); const result = registry.register(owned, options);
  assert.throws(() => registry.register(owned, options), /process identity/);
  assert.throws(() => registry.register(child(0), options), /process identity/);
  owned.emit('close', 0, null);
  assert.deepEqual(await result, { pid: 100, exit_code: 0, signal: null, reason: 'completed', drained: true });
  assert.deepEqual(await registry.drain(), [await result]);
});

test('deadline terminates the owned group and retains forced exit evidence', async () => {
  const signals = []; const owned = child(101);
  const registry = new ProcessRegistry({ terminate: (pid, signal) => {
    signals.push([pid, signal]); if (signal === 'SIGKILL') owned.emit('close', null, signal);
  } });
  const report = await registry.register(owned, { ...options, timeoutMs: 5 });
  assert.deepEqual(signals, [[101, 'SIGTERM'], [101, 'SIGKILL']]);
  assert.equal(report.reason, 'deadline'); assert.equal(report.drained, true);
});

test('combined stdout and stderr limits terminate without retaining secret output', async () => {
  const owned = child(102); const signals = [];
  const registry = new ProcessRegistry({ terminate: (pid, signal) => { signals.push(signal); owned.emit('close', null, signal); } });
  const result = registry.register(owned, options);
  owned.stdout.emit('data', Buffer.from('authorization:'));
  owned.stderr.emit('data', Buffer.from(' bearer private'));
  const report = await result;
  assert.deepEqual(signals, ['SIGTERM']); assert.equal(report.reason, 'output-limit');
  assert.ok(!JSON.stringify(report).includes('private'));
});

test('shutdown attempts every child and a non-draining process cannot appear successful', async () => {
  const first = child(103); const second = child(104); const signals = [];
  const registry = new ProcessRegistry({ terminate: (pid, signal) => {
    signals.push([pid, signal]); if (pid === 104) second.emit('close', null, signal);
  } });
  registry.register(first, options); registry.register(second, options);
  const reports = await registry.shutdown();
  assert.ok(signals.some(([pid]) => pid === 103)); assert.ok(signals.some(([pid]) => pid === 104));
  assert.equal(reports.find(r => r.pid === 103).drained, false);
  assert.equal(reports.find(r => r.pid === 104).drained, true);
});

test('registry enforces an aggregate output cap across all owned children', async () => {
  const first = child(105); const second = child(106); const owned = new Map([[105, first], [106, second]]);
  const registry = new ProcessRegistry({ outputBytes: 10, terminate: (pid, signal) => owned.get(pid).emit('close', null, signal) });
  registry.register(first, options); const result = registry.register(second, options);
  first.stdout.emit('data', Buffer.from('123456')); second.stderr.emit('data', Buffer.from('78901'));
  assert.equal((await result).reason, 'output-limit');
  await registry.shutdown();
});

test('concurrency cap preserves earlier reports while allowing a replacement child after close', async () => {
  const registry = new ProcessRegistry({ maxProcesses: 1, terminate: () => {} });
  const first = child(107); const second = child(108);
  const firstResult = registry.register(first, options);
  assert.throws(() => registry.register(second, options), /process identity/);
  first.emit('close', 0, null); await firstResult;
  const secondResult = registry.register(second, options); second.emit('close', 0, null); await secondResult;
  assert.equal((await registry.drain()).length, 2);
});
