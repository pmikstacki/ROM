import test from 'node:test';
import assert from 'node:assert/strict';
import {identityBrowserCommand} from './identity-browser-profile.mjs';
const crypto={path:'/private/patched.sh'};
test('Chromium keeps the ordinary owned Node browser entry',()=>{
 assert.deepEqual(identityBrowserCommand('chromium','/node','/entry','/config',null),{executable:'/node',args:['/entry','/config']});
});
test('WebKit requires patched crypto and starts in a private mount namespace',()=>{
 assert.throws(()=>identityBrowserCommand('webkit','/node','/entry','/config',null),/patched crypto/);
 const result=identityBrowserCommand('webkit','/node','/entry','/config',crypto);
 assert.match(result.executable,/\/unshare$/);
 assert.deepEqual(result.args,['--mount','--propagation','private','--fork','/node','/entry','/config']);
});
test('unknown engines and malformed executable references are rejected',()=>{
 for(const engine of ['firefox',null])assert.throws(()=>identityBrowserCommand(engine,'/node','/entry','/config',crypto));
 assert.throws(()=>identityBrowserCommand('webkit','/node','/entry','/config',{path:''}));
});
