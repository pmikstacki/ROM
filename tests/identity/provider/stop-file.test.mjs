import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, renameSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createOwnedStopFile, readOwnedStopFile, requestOwnedStop } from './stop-file.mjs';
function fixture() { const root=mkdtempSync(join(tmpdir(),'rom-provider-stop-'));mkdirSync(`${root}/private`,{mode:0o700});return{root,path:`${root}/private/stop`}; }
test('a fresh exclusive private stop file retains identity through the bounded stop request',()=>{
 const {root,path}=fixture(),identity=createOwnedStopFile(root,path);assert.equal(readOwnedStopFile(root,identity),false);requestOwnedStop(root,identity);assert.equal(readOwnedStopFile(root,identity),true);assert.throws(()=>createOwnedStopFile(root,path));
});
test('replacement and symbolic links cannot reuse an admitted stop identity',()=>{
 for(const symbolic of[false,true]){const{root,path}=fixture(),identity=createOwnedStopFile(root,path);renameSync(path,`${path}-retained`);if(symbolic)symlinkSync(`${path}-retained`,path);else writeFileSync(path,'stop\n',{flag:'wx',mode:0o600});assert.throws(()=>readOwnedStopFile(root,identity));assert.throws(()=>requestOwnedStop(root,identity));}
});
test('oversized, malformed and outside-root files cannot request an owned stop',()=>{
 const{root,path}=fixture(),identity=createOwnedStopFile(root,path);for(const bytes of['arbitrary','no\n']){writeFileSync(path,bytes);assert.throws(()=>readOwnedStopFile(root,identity));}assert.throws(()=>createOwnedStopFile(root,`${root}-sibling/stop`));assert.throws(()=>readOwnedStopFile(root,{...identity,path:'/tmp'}));
});
