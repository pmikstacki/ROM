import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,statSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {mountAcknowledged} from './mount-ack.mjs';
test('mount bridge waits for exact exclusive acknowledgement and rejects replacement',()=>{
 const root=mkdtempSync(`${tmpdir()}/rom-mount-ack-`),path=`${root}/ack`;writeFileSync(path,'',{flag:'wx',mode:0o600});const s=statSync(path),identity={path,device:s.dev,inode:s.ino,uid:s.uid};
 assert.equal(mountAcknowledged(identity),false);writeFileSync(path,'ready\n');assert.equal(mountAcknowledged(identity),true);
 assert.throws(()=>mountAcknowledged({...identity,inode:s.ino+1}));writeFileSync(path,'unsafe');assert.throws(()=>mountAcknowledged(identity));
});
