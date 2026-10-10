import test from 'node:test';
import assert from 'node:assert/strict';
import { NativeTlsBrowserObservation } from './native-tls-browser-observation.mjs';
test('failure metadata excludes credential-bearing URLs and exception text', () => {
 const o=new NativeTlsBrowserObservation();o.enter('navigation');o.location('https://127.0.0.1:44392/if/flow/default-authentication-flow/?code=SECRET#TOKEN');o.response('https://evil.example/private?password=SECRET',200);
 const r=o.failure({name:'TimeoutError',message:'PASSWORD',stack:'TOKEN'}, {identification:1,password:0,frames:2});
 assert.equal(r.error_name,'TimeoutError');assert.deepEqual(r.locations,[{origin:'https://127.0.0.1:44392',pathname:'/if/flow/default-authentication-flow/'}]);assert.doesNotMatch(JSON.stringify(r),/SECRET|TOKEN|PASSWORD|evil|private/);
});
test('closed stages, transitions and selector counts reject malformed values',()=>{
 const o=new NativeTlsBrowserObservation();assert.throws(()=>o.enter('credential=SECRET'));o.enter('identification-wait');o.enter('password-wait');assert.deepEqual(o.failure({name:'SecretError'},{}).stages,['launch','identification-wait','password-wait']);assert.equal(o.failure({name:'SecretError'},{}).error_name,'Other');assert.throws(()=>o.failure({}, {password:33}));assert.throws(()=>o.failure({}, {secret:1}));
});
test('metadata entry and byte bounds do not grow with attacker-controlled input',()=>{
 const o=new NativeTlsBrowserObservation();for(let i=0;i<10000;i++){o.location('https://127.0.0.1:44389/'+ 'x'.repeat(20000));o.response('https://127.0.0.1:44392/other?secret='+i,500);}const r=o.failure({name:'Error',message:'x'.repeat(100000)},{});assert.ok(Buffer.byteLength(JSON.stringify(r))<=4096);assert.ok(r.locations.length<=16);assert.ok(Object.values(r.response_status_counts).every(n=>n<=128));
});
