import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runChild } from './process.mjs';

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
