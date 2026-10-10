import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { auditSourceInputs, finalObservation } from './source-fence.mjs';
const hash=b=>createHash('sha256').update(b).digest('hex');
test('changed browser configuration or imported helper refuses acceptance',()=>{
 const root=mkdtempSync('/var/tmp/rom-astral-source-fence-');
 try {
  const files=[root+'/config',root+'/helper'];
  for(const file of files)writeFileSync(file,'original');
  const witness={source_inputs:files.map(path=>({path,sha256:hash(readFileSync(path))}))};
  auditSourceInputs(witness);
  for(const file of files){writeFileSync(file,'changed');assert.throws(()=>auditSourceInputs(witness),/changed/);writeFileSync(file,'original');}
 }finally{rmSync(root,{recursive:true,force:true});}
});
test('missing source closure refuses acceptance',()=>assert.throws(()=>auditSourceInputs({}),/closure/));
test('failed final observation retains primary failure and records failure category',()=>{
 const record={status:'failed',error:'original browser failure'};
 assert.equal(finalObservation(record,'artifact_inventory',()=>{throw Error('later observation failure');}),undefined);
 assert.equal(record.error,'original browser failure');
 assert.deepEqual(record.final_failures,[{stage:'artifact_inventory',category:'Error'}]);
});
test('failed final observation cannot retain a successful status',()=>{
 const record={status:'passed'};
 finalObservation(record,'socket',()=>{throw Error('socket unavailable');});
 assert.equal(record.status,'failed');
});
