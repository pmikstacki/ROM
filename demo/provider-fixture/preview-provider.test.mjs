import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';
test('service loads external bounded credentials and closes on SIGTERM', { timeout: 10000 }, async () => {
  const dir = await mkdtemp(join(tmpdir(), 'rom-preview-profile-'));
  let child;
  try {
    const secret = join(dir, 'secret');
    const config = join(dir, 'profile.json');
    await writeFile(secret, 'private-test-client-secret\n', { mode: 0o600 });
    await writeFile(config, JSON.stringify({ issuer:'https://preview.example/rom-studio-provider', redirect_uri:'https://preview.example/rom-studio/auth/callback/local',client_id:'studio',client_secret_file:secret,port:0 }));
    child = spawn(process.execPath, [fileURLToPath(new URL('./preview-provider.mjs', import.meta.url)), config], {stdio:['ignore','pipe','pipe']});
    let output = '';
    child.stdout.setEncoding('utf8');
    const ready = new Promise((resolve,reject) => {
      child.stdout.on('data', data => { output += data; if(output.includes('\n')) resolve(JSON.parse(output.split('\n')[0])); });
      child.on('error',reject);
      child.on('exit',code=>reject(Error(`unexpected service exit ${code}`)));
    });
    const value = await ready;
    assert.equal(value.demonstration,true); assert.equal(value.issuer,'https://preview.example/rom-studio-provider');
    assert.ok(value.port >0); assert.ok(!output.includes('private-test-client-secret'));
    const exited = once(child,'exit'); child.kill('SIGTERM');
    assert.equal((await exited)[0],0);
  } finally { child?.kill('SIGKILL'); await rm(dir,{recursive:true,force:true}); }
});
