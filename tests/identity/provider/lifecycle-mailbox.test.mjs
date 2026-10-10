import test from 'node:test';import assert from 'node:assert/strict';
import{admitLifecycleRequest}from'./lifecycle-mailbox.mjs';
test('lifecycle requests are closed, sequential, and limited to server pause/resume',()=>{assert.deepEqual(admitLifecycleRequest({sequence:1,action:'pause-server'},1),{sequence:1,action:'pause-server'});for(const value of[{sequence:1,action:'restart'},{sequence:1,action:'pause-server',id:'anything'},{sequence:2,action:'pause-server'},{sequence:1,action:'pause-worker'}])assert.throws(()=>admitLifecycleRequest(value,1));assert.throws(()=>admitLifecycleRequest({sequence:5,action:'pause-server'},5));});

import{mkdtempSync,mkdirSync,writeFileSync}from'node:fs';
import{tmpdir}from'node:os';
import{join}from'node:path';
import{LifecycleMailbox}from'./lifecycle-mailbox.mjs';
test('overlapping polls cannot pause an owned provider twice',async()=>{
 const approvedRoot=mkdtempSync(join(tmpdir(),'rom-lifecycle-unit-')),directory=join(approvedRoot,'mailbox');mkdirSync(directory,{mode:0o700});
 writeFileSync(directory+'/request-1.json',JSON.stringify({sequence:1,action:'pause-server'}),{flag:'wx',mode:0o600});
 const mailbox=new LifecycleMailbox(directory,{approvedRoot});let release,calls=0;const pending=new Promise(resolve=>{release=resolve;});
 const provider={async pause(){calls++;await pending;return{synthetic_unit:true};}};
 const first=mailbox.poll(provider,'owned-server');const second=mailbox.poll(provider,'owned-server');second.catch(()=>{});release();await first;
 await assert.rejects(second,/already pending/);assert.equal(calls,1);
});

test('injected lifecycle roots preserve private ownership and containment checks',async()=>{
 const{chmodSync,symlinkSync}=await import('node:fs');
 const approvedRoot=mkdtempSync(join(tmpdir(),'rom-lifecycle-root-')),directory=join(approvedRoot,'mailbox');mkdirSync(directory,{mode:0o700});
 assert.throws(()=>new LifecycleMailbox(directory),/owned lifecycle directory|ENOENT/);
 const outside=mkdtempSync(join(tmpdir(),'rom-lifecycle-outside-'));assert.throws(()=>new LifecycleMailbox(outside,{approvedRoot}),/owned lifecycle directory/);
 chmodSync(approvedRoot,0o755);assert.throws(()=>new LifecycleMailbox(directory,{approvedRoot}),/owned lifecycle directory/);chmodSync(approvedRoot,0o700);
 const alias=join(approvedRoot,'alias');symlinkSync(directory,alias);assert.throws(()=>new LifecycleMailbox(alias,{approvedRoot}),/owned lifecycle directory/);
});
