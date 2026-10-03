import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, existsSync, writeFileSync, readdirSync, symlinkSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { join } from 'node:path';
import { capability, publish } from './publication.mjs';
import { produce } from './produce.mjs';
import { fixture, passed, scratch } from './test-support.mjs';

test('unsupported no-replace utility rejects before any acceptance gate', async () => {
  const f = fixture();
  const bin = join(f.parent, 'bin'); mkdirSync(bin);
  writeFileSync(join(bin, 'mv'), '#!/bin/sh\nexit 9\n', { mode: 0o755 });
  const original = process.env.PATH;
  process.env.PATH = `${bin}:${original}`;
  let calls = 0;
  try {
    await assert.rejects(produce({ ...f, runner: async () => { calls++; return passed(); } }));
    assert.equal(calls, 0); assert.equal(existsSync(f.output), false);
  } finally { process.env.PATH = original; }
});

test('dangling existing output rejects before gates', async () => {
  const f = fixture(); symlinkSync('missing', f.output);
  let calls = 0;
  await assert.rejects(produce({ ...f, runner: async () => { calls++; return passed(); } }), /output exists/);
  assert.equal(calls, 0);
});

test('unsupported output filesystem rejects before gates', async () => {
  const f = fixture();
  const bin = join(f.parent, 'filesystem-bin'); mkdirSync(bin);
  writeFileSync(join(bin, 'findmnt'), '#!/bin/sh\nprintf "overlay\\n"\n', { mode: 0o755 });
  const original = process.env.PATH; process.env.PATH = `${bin}:${original}`;
  let calls = 0;
  try {
    await assert.rejects(produce({ ...f, runner: async () => { calls++; return passed(); } }), /ext4/);
    assert.equal(calls, 0); assert.equal(existsSync(f.output), false);
  } finally { process.env.PATH = original; }
});

test('publication refuses an existing empty directory', () => {
  const parent = scratch('publication');
  capability(parent);
  const stage = join(parent, 'stage'), output = join(parent, 'final');
  mkdirSync(stage); writeFileSync(join(stage, 'complete'), 'original'); mkdirSync(output);
  assert.throws(() => publish(stage, output));
  assert.ok(existsSync(stage)); assert.deepEqual(readdirSync(output), []);
});

test('concurrent no-replace contenders expose exactly one complete directory', async () => {
  const parent = scratch('race');
  const output = join(parent, 'final');
  const stages = ['a', 'b'].map(name => { const path = join(parent, name); mkdirSync(path); writeFileSync(join(path, 'complete'), name); return path; });
  const result = await Promise.all(stages.map(stage => new Promise((resolve, reject) => {
    const child = spawn('mv', ['-T', '--no-copy', '--update=none-fail', stage, output], { stdio: 'ignore' });
    const timer = setTimeout(() => { child.kill('SIGKILL'); reject(Error('publication exceeded bound')); }, 3000);
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.on('close', code => { clearTimeout(timer); resolve(code); });
  })));
  assert.equal(result.filter(code => code === 0).length, 1);
  assert.equal(stages.filter(existsSync).length, 1);
  assert.deepEqual(readdirSync(output), ['complete']);
  assert.ok(['a', 'b'].includes(readFileSync(join(output, 'complete'), 'utf8')));
});
