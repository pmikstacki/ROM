import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { OwnedLauncher, sameProcessIdentity } from './launch.mjs';

test('actual detached child records its birth, group and session before owned termination', async () => {
  const launcher = new OwnedLauncher();
  const child = launcher.launch(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { timeoutMs: 2000, graceMs: 30, drainMs: 30, outputBytes: 1024 });
  assert.equal(child.identity.pid, child.identity.group);
  assert.equal(child.identity.pid, child.identity.session);
  assert.match(child.identity.started, /^\d+$/);
  assert.match(child.identity.boot_id, /^[a-f0-9-]+$/);
  assert.throws(() => launcher.stop(process.pid), /unowned process/);
  launcher.stop(child.identity.pid);
  const report = await child.closed;
  assert.equal(report.reason, 'shutdown'); assert.equal(report.drained, true);
  assert.equal(report.signal, 'SIGTERM');
});

test('invalid admission limits refuse execution before a child can write its marker', async () => {
  const marker = join(mkdtempSync(join(tmpdir(), 'rom-identity-launch-')), 'unlaunched');
  const launcher = new OwnedLauncher();
  assert.throws(() => launcher.launch(process.execPath, ['-e', `require('node:fs').writeFileSync(${JSON.stringify(marker)}, 'started')`],
    { timeoutMs: 0, graceMs: 10, drainMs: 10, outputBytes: 1024 }), /limits/);
  await new Promise(resolve => setTimeout(resolve, 100));
  assert.equal(existsSync(marker), false);
});

test('PID reuse, different group, and different boot cannot match an owned process', () => {
  const original = { pid: 123, group: 123, session: 123, started: '456', boot_id: 'abc' };
  assert.equal(sameProcessIdentity(original, { ...original }), true);
  for (const field of ['pid', 'group', 'session', 'started', 'boot_id']) {
    assert.equal(sameProcessIdentity(original, { ...original, [field]: 'changed' }), false);
  }
  assert.equal(sameProcessIdentity(original, null), false);
});

test('signal guard refuses a reused child identity and records failed drain without sending a signal', async () => {
  const signals = []; let started = '1';
  const launcher = new OwnedLauncher({ readIdentity: pid => ({ pid, group: pid, session: pid, started, boot_id: 'fixture' }),
    sendSignal: (pid, signal) => signals.push([pid, signal]) });
  const child = launcher.launch(process.execPath, ['-e', 'setTimeout(() => {}, 30)'], { timeoutMs: 1000, graceMs: 10, drainMs: 10, outputBytes: 1024 });
  started = '2';
  launcher.stop(child.identity.pid);
  const report = await child.closed;
  assert.equal(report.drained, false); assert.deepEqual(signals, []);
  // The inert finite child exits on its own; retain ownership until its close.
  await child.physicalClose;
});

test('failed group ownership never executes the requested command', async () => {
  const marker = join(mkdtempSync(join(tmpdir(), 'rom-identity-unowned-')), 'unlaunched');
  const launcher = new OwnedLauncher({ readIdentity: () => null });
  assert.throws(() => launcher.launch(process.execPath, ['-e', `require('node:fs').writeFileSync(${JSON.stringify(marker)}, 'started')`],
    { timeoutMs: 1000, graceMs: 10, drainMs: 10, outputBytes: 1024 }), /owned process identity/);
  await new Promise(resolve => setTimeout(resolve, 100));
  assert.equal(existsSync(marker), false);
});

test('failed resource admission refuses execution before approval', async () => {
 const marker=join(mkdtempSync(join(tmpdir(),'rom-identity-resource-')),'unlaunched');const launcher=new OwnedLauncher();
 assert.throws(()=>launcher.launch(process.execPath,['-e',`require('node:fs').writeFileSync(${JSON.stringify(marker)},'started')`],{timeoutMs:1000,admitIdentity:()=>{throw Error('resource admission failed');}}),/resource admission/);
 await new Promise(resolve=>setTimeout(resolve,100));assert.equal(existsSync(marker),false);
});
