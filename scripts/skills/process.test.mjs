import assert from 'node:assert/strict';
import { test } from 'node:test';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runChild } from './process.mjs';

test('invalid supervision options reject before the process can start', async () => {
  const scratch = mkdtempSync(join(tmpdir(), 'rom-skill-options-'));
  const marker = join(scratch, 'started');
  const program = `require('node:fs').writeFileSync(${JSON.stringify(marker)},'started');`;
  try {
    assert.throws(() => runChild(process.execPath, ['-e', program], { timeoutMs: 1000 }), /unsupported process option: timeoutMs/);
    for (const option of ['timeout', 'maxBytes']) {
      for (const value of [0, -1, 1.5, NaN, Infinity, '1000']) {
        assert.throws(() => runChild(process.execPath, ['-e', program], { [option]: value }), /positive safe integer/);
      }
    }
    assert.throws(() => runChild(process.execPath, ['-e', program], { timeout: 2 ** 31 }), /supported timer range/);
    const accepted = await runChild(process.execPath, ['-e', "process.stdout.write('accepted')"], { timeout: undefined, maxBytes: undefined });
    assert.equal(accepted.code, 0);
    assert.equal(accepted.stdout, 'accepted');
    assert.equal(existsSync(marker), false);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test('deadline kills a descendant holding inherited stdio and returns within its bound', async () => {
  const scratch = mkdtempSync(join(tmpdir(), 'rom-skill-process-'));
  const pidFile = join(scratch, 'pid');
  const child = `process.on('SIGTERM',()=>{});setInterval(()=>{},1000);`;
  const parent = `const {spawn}=require('node:child_process');const {writeFileSync}=require('node:fs');const child=spawn(process.execPath,['-e',${JSON.stringify(child)}],{stdio:'inherit'});writeFileSync(${JSON.stringify(pidFile)},String(child.pid));process.on('SIGTERM',()=>{});setInterval(()=>{},1000);`;
  const began = Date.now();
  let pid;
  try {
    const result = await runChild(process.execPath, ['-e', parent], { timeout: 500 });
    assert.equal(result.timedOut, true);
    assert.ok(Date.now() - began < 4000);
    pid = Number(readFileSync(pidFile));
    let state;
    try { state = readFileSync(`/proc/${pid}/stat`, 'utf8').split(' ')[2]; } catch { state = 'gone'; }
    assert.ok(state === 'gone' || state === 'Z', `owned descendant remains live: ${state}`);
  } finally {
    if (pid) try { process.kill(pid, 'SIGKILL'); } catch {}
    rmSync(scratch, { recursive: true, force: true });
  }
});

test('output byte budget terminates the owned process group', async () => {
  const result = await runChild(process.execPath, ['-e', "process.stdout.write('ß'.repeat(10000));setInterval(()=>{},1000);"], { timeout: 5000, maxBytes: 16 });
  assert.equal(result.timedOut, true);
  assert.ok(Buffer.byteLength(result.stdout + result.stderr) <= 16);
});

test('process output preserves UTF-8 split across pipe reads', async () => {
  const program = `const out=process.stdout,err=process.stderr;out.write(Buffer.from([0xe2]));err.write(Buffer.from([0xe2]));setTimeout(()=>{out.write(Buffer.from([0x82]));err.write(Buffer.from([0x82]));},20);setTimeout(()=>{out.end(Buffer.from([0xac]));err.end(Buffer.from([0xac]));},40);`;
  const result = await runChild(process.execPath, ['-e', program], { timeout: 2000 });
  assert.equal(result.code, 0);
  assert.equal(result.stdout, '€');
  assert.equal(result.stderr, '€');
});
