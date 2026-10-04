// Real native HTTPS profile admission. TLS proxy and browser login are separate checks.
import { startHttpsProvider } from '../../../demo/provider-fixture/https-provider.mjs';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { spawn } from 'node:child_process';
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
for (const backend of ['sqlite','redb']) {
  const directory=await mkdtemp('/var/tmp/rom-studio-https-profile-');
  const socket=createServer().listen(0,'127.0.0.1');await once(socket,'listening');const port=socket.address().port;await new Promise(done=>socket.close(done));
  const issuer='https://studio.example/identity';
  const provider=await startHttpsProvider({issuer,redirectUri:'https://studio.example/rom-studio/auth/callback/local',clientId:'studio',clientSecret:'controlled-preview-profile-secret'});
  const secret=`${directory}/secret`,profile=`${directory}/profile.json`;
  await writeFile(secret,'controlled-preview-profile-secret',{mode:0o600});
  await writeFile(profile,JSON.stringify({public_origin:'https://studio.example',issuer,authorization_endpoint:`${issuer}/auth`,token_endpoint:`${issuer}/token`,jwks_endpoint:`${issuer}/jwks`,client_secret_file:secret,backchannel_token_endpoint:`http://127.0.0.1:${provider.port}/identity/token`,backchannel_jwks_endpoint:`http://127.0.0.1:${provider.port}/identity/jwks`}),{mode:0o600});
  const child=spawn('/var/lib/nixos-containers/rom-dev/var/tmp/rom-release-measured-verification-target/debug/rom-demo',['studio-profile',backend,`${directory}/database`,String(port),resolve('dist'),profile],{stdio:['ignore','pipe','pipe']});
  let output='',errors='';child.stderr.on('data',bytes=>errors+=bytes);
  try {
    await new Promise((done,reject)=>{const timer=setTimeout(()=>reject(Error('profile readiness deadline')),10000);child.once('exit',()=>{clearTimeout(timer);reject(Error('profile exited before ready'));});child.stdout.on('data',bytes=>{output+=bytes;if(output.includes('\n')){clearTimeout(timer);done();}});});
    const ready=JSON.parse(output.split('\n')[0]);assert.equal(ready.origin,'https://studio.example');assert.equal(ready.fixture_controls,false);
    const response=await fetch(`http://127.0.0.1:${port}/rom-studio/auth/providers`);assert.equal(response.status,200);assert.deepEqual(await response.json(),{providers:[{id:'local',label:'Local demo provider'}],primary:null});
    const exited=once(child,'exit');child.kill('SIGTERM');assert.deepEqual(await exited,[0,null]);
    await writeFile(`${directory}/result.json`,JSON.stringify({backend,ready,exit:0,scope:'Real native profile/startup/provider listing. TLS proxy and human browser login remain separate acceptance.'},null,2));
    console.log(JSON.stringify({backend,directory,profile_startup:'passed'}));
  } finally {if(child.exitCode===null&&child.signalCode===null)child.kill('SIGKILL');await provider.close();await writeFile(`${directory}/host.log`,output+errors);}
}
