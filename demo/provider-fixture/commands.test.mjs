import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, readFile, writeFile, rm, stat, symlink, chmod } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { bounded, startNode as command, stop, stopAll, ready } from './processes.mjs';
import { readPrivate } from './files.mjs';

test('separate commands issue a private CLI auth file and redact credentials', { timeout: 15000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), 'rom-provider-command-'));
  let provider;
  const children = [];
  try {
    const secret = randomBytes(32).toString('hex');
    const introspectionSecret = randomBytes(32).toString('hex');
    const serviceFile = join(root, 'service');
    const introspectionFile = join(root, 'introspection');
    const configFile = join(root, 'config.json');
    const readyFile = join(root, 'ready.json');
    const authFile = join(root, 'auth');
    await writeFile(serviceFile, secret, { mode: 0o600 });
    await writeFile(introspectionFile, introspectionSecret, { mode: 0o600 });
    await writeFile(configFile, JSON.stringify({ service_secret_file: serviceFile, introspection_secret_file: introspectionFile }), { mode: 0o600 });
    provider = command('server.mjs', [configFile, readyFile]);
    const metadata = await ready(readyFile, provider);
    assert.equal(metadata.version, '9.12.2');
    assert.equal(new URL(metadata.issuer).hostname, '127.0.0.1');
    const issue = command('issue.mjs', [readyFile, serviceFile, authFile]);
    children.push(issue);
    assert.deepEqual(await bounded(issue.done), [0, null]);
    const header = await readFile(authFile, 'utf8');
    assert.ok(header.startsWith('Bearer ') && header.length > 7 && header.length <= 4103);
    assert.equal((await stat(authFile)).mode & 0o077, 0);
    const duplicate = command('issue.mjs', [readyFile, serviceFile, authFile]);
    children.push(duplicate);
    assert.deepEqual(await bounded(duplicate.done), [1, null]);
    assert.ok((await readFile(authFile, 'utf8')) === header, 'existing credential file must survive');
    for (const process of [provider, ...children]) {
      for (const sensitive of [secret, introspectionSecret, header.slice(7), serviceFile, introspectionFile])
        assert.ok(!process.output().includes(sensitive), 'command output must omit credential material and paths');
    }
  } finally {
    try { await stopAll(provider ? [...children, provider] : children); }
    finally { await rm(root, { recursive: true, force: true }); }
  }
});

test('fixture private reads reject unsafe and unbounded files without FIFO blocking', { timeout: 3000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), 'rom-provider-file-'));
  try {
    const file = join(root, 'secret');
    const link = join(root, 'link');
    await writeFile(file, 'small-value', { mode: 0o600 });
    assert.equal(await readPrivate(file, 4096), 'small-value');
    await symlink(file, link);
    await assert.rejects(readPrivate(link, 4096));
    await assert.rejects(readPrivate(root, 4096));
    await chmod(file, 0o644);
    await assert.rejects(readPrivate(file, 4096));
    await chmod(file, 0o600);
    for (const data of ['', 'x'.repeat(4097), Buffer.from([0xff])]) {
      await writeFile(file, data);
      await assert.rejects(readPrivate(file, 4096));
    }
    const fifo = join(root, 'fifo');
    assert.equal(spawnSync('mkfifo', [fifo]).status, 0);
    const fifoReader = command('private-read-child.mjs', [fifo]);
    try { assert.deepEqual(await bounded(fifoReader.done, 1000), [0, null]); }
    finally { await stop(fifoReader); }
  } finally { await rm(root, { recursive: true, force: true }); }
});
