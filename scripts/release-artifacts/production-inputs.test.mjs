import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { hash } from '../skills/files.mjs';
import { productionProfile, admitProductionRequirements, retainProductionRequirements } from './production-inputs.mjs';
function inputs() {
  const directory=mkdtempSync(join(tmpdir(),'rom-production-requirements-')), source='a'.repeat(64);
  const requirements=Array.from({length:14},(_,i)=>{
    const id=`R${i+1}`, path=`${id}.json`, review=`${id}-review.json`;
    writeFileSync(join(directory,path),JSON.stringify({executed:true}));writeFileSync(join(directory,review),'independent scoped review');
    const ref=(p,kind)=>({path:p,sha256:hash(join(directory,p)),bytes:readFileSync(join(directory,p)).length,kind});
    return{id,state:'accepted',source_identity:source,evidence:[ref(path,'executed')],measured_scope:'fixture executed scope',limitations:'fixture only',review:ref(review,'review')};
  });
  const record={schema_version:1,source_identity:source,requirements};writeFileSync(join(directory,'record.json'),JSON.stringify(record));
  return{directory,source,record,input:{directory,sha256:hash(join(directory,'record.json'))}};
}
test('production versions select mandatory v3 while historical profiles remain stable',()=>{
  assert.equal(productionProfile('0.0.3'),'rom-studio-v2');assert.equal(productionProfile('0.1.0'),'rom-public-consumers-v3');assert.equal(productionProfile('1.0.0'),'rom-public-consumers-v3');
});
test('production requires actual source-bound reviewed complete ledger, not an optional metadata flag',()=>{
  const f=inputs();assert.throws(()=>admitProductionRequirements(undefined,f.source),/release requirement evidence required/);
  const accepted=admitProductionRequirements(f.input,f.source);assert.equal(accepted.summary.accepted,14);
  assert.throws(()=>admitProductionRequirements(f.input,'b'.repeat(64)),/requirement/);
  f.record.requirements[11].state='pending';writeFileSync(join(f.directory,'record.json'),JSON.stringify(f.record));f.input.sha256=hash(join(f.directory,'record.json'));
  assert.throws(()=>admitProductionRequirements(f.input,f.source),/incomplete release requirements/);
});
test('retained ledger preserves exact evidence and refuses stale bytes',()=>{
  const f=inputs(), admitted=admitProductionRequirements(f.input,f.source), stage=mkdtempSync(join(tmpdir(),'rom-ledger-stage-'));mkdirSync(join(stage,'evidence'));
  const retained=retainProductionRequirements(admitted,stage);assert.equal(retained.record.source_identity,f.source);
  assert.equal(hash(join(stage,'evidence/requirements/R12.json')),f.record.requirements[11].evidence[0].sha256);
  writeFileSync(join(f.directory,'R12.json'),'changed');assert.throws(()=>retainProductionRequirements(admitted,mkdtempSync(join(tmpdir(),'rom-ledger-changed-'))),/requirement|ENOENT/);
});

test('current production versions cannot be admitted as a legacy eight-gate artifact',async()=>{
 const {requireManifestProfile}=await import('./production-inputs.mjs');
 assert.equal(requireManifestProfile({manifest_version:2,source:{package_version:'0.0.3'}}),true);
 assert.equal(requireManifestProfile({manifest_version:3,verification_profile:'rom-public-consumers-v3',source:{package_version:'0.1.0'}}),true);
 for(const version of [1,2])assert.throws(()=>requireManifestProfile({manifest_version:version,source:{package_version:'0.1.0'}}),/production release requires/);
});
