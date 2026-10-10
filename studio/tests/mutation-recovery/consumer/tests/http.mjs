// Real HTTP and process restart acceptance through installed public package entries.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, existsSync, openSync, fsyncSync, closeSync } from 'node:fs';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { createClient, stringifyWire } from 'rom-studio/client';
import { createMutationRecovery } from 'rom-studio/recovery';
import { startProxy } from '../../runtime/proxy.mjs';
const evidence = process.env.ROM_RECOVERY_EVIDENCE;
const proxy = await startProxy({ binary:process.env.ROM_RECOVERY_HOST, adapter:process.env.ROM_RECOVERY_ADAPTER, database:process.env.ROM_RECOVERY_DATABASE, port:Number(process.env.ROM_RECOVERY_PORT), evidence });
const file = join(evidence,'http-pending-intent.json');
function read() { return existsSync(file) ? JSON.parse(readFileSync(file,'utf8')) : null; }
const store = { async read() { return read(); }, async compareExchange(_slot,version,next) { if ((read()?.version ?? null) !== version) return false; writeFileSync(file,JSON.stringify(next)); const data=openSync(file,'r');try{fsyncSync(data);}finally{closeSync(data);} const directory=openSync(evidence,'r');try{fsyncSync(directory);}finally{closeSync(directory);} return true; } };
let generation = 0;
const principal = {authority:'recovery-fixture',kind:'human',subject:'alice'};
function client(subject='alice') { const current = generation; return createClient({ base:proxy.base+'/api', csrf:()=>`fixture-csrf-${current}`, fetch:(url,init)=>{const headers=new Headers(init.headers);headers.set('authorization',`Bearer fixture-${subject==='alice'?'owner':'other'}-${current}`);return fetch(url,{...init,headers});} }); }
const options = current => ({namespace:'http-fixture',slot:'one',target:{kind:'recovery-notes',id:'http-one'},binding:{client:current,principal},store,newVersion:randomUUID});
async function control(path,body={}) { const response=await fetch(proxy.base+'/__test/'+path,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});assert.equal(response.status,200);return response.json(); }
const probe=(key,operation='save')=>control('inspect',{id:'http-one',idempotency:key,operation});
const counts={};
try {
  const seedClient=client(); await seedClient.submit(seedClient.prepare({kind:'recovery-notes',id:'http-one',expected:null,idempotency:'http-seed',operation:{type:'create',input:{owner:'alice',title:'seed',count:9007199254740993n,optional:null,flag:false}}}));
  const beforeDispatch=await probe('http-save-A');
  const refused=createMutationRecovery({...options(client()),store:{async read(){return null;},async compareExchange(){return false;}}});
  await assert.rejects(refused.stage({type:'delete'}));assert.deepEqual((await probe('http-save-A')).counts,beforeDispatch.counts);assert.equal((await control('trace',{id:'http-one'})).bodies.length,1);
  const firstClient=client(), first=createMutationRecovery(options(firstClient));
  await first.stage({type:'action',input:{name:'save',input:{title:'A',count:9007199254740993n}}});await first.begin({expected:1n,idempotency:'http-save-A',retryEpoch:0n});
  await control('drop',{id:'http-one',key:'http-save-A'});await assert.rejects(first.retry());await control('resume');assert.equal(first.state.commitKnowledge,'unknown');
  counts.committed=await probe('http-save-A');assert.equal(counts.committed.row.revision,2);assert.equal(counts.committed.events_for_id,2);assert.deepEqual(counts.committed.counts,[1,2,2,1]);
  assert.equal(counts.committed.receipt.identity,counts.committed.expected_identity);assert.match(counts.committed.wire,/9007199254740993/);
  await first.stage({type:'action',input:{name:'save',input:{title:'B',count:9007199254740993n}}});await first.stage({type:'action',input:{name:'save',input:{title:'C',count:9007199254740993n}}});first.dispose();
  await control('restart');generation=(await control('session')).generation;
  const refreshed=client(), restored=createMutationRecovery(options(refreshed));await restored.restore();const recoveredView=await restored.retry();assert.equal(recoveredView.value.count,9007199254740993n);
  counts.replayed=await probe('http-save-A');assert.deepEqual(counts.replayed.counts,counts.committed.counts);assert.equal(restored.state.draft.input.input.title,'C');
  let trace=await control('trace',{id:'http-one'});assert.equal(trace.bodies[1],trace.bodies[2]);assert.match(trace.bodies[2],/9007199254740993/);assert.match(trace.bodies[2],/"expected":1/);
  // Same identity with changed bytes must not mutate storage or disclose a new result.
  await assert.rejects(refreshed.submit(refreshed.prepare({kind:'recovery-notes',id:'http-one',expected:1n,idempotency:'http-save-A',operation:{type:'action',input:{name:'save',input:{title:'different',count:9007199254740993n}}}})),error=>error.category==='identity_mismatch');
  assert.deepEqual((await probe('http-save-A')).counts,counts.committed.counts);
  // A stale host-chosen expected revision gets a real conflict; no automatic rebase follows.
  await restored.begin({expected:1n,idempotency:'http-save-C'});await assert.rejects(restored.retry(),error=>error.category==='conflict');assert.equal(restored.state.phase,'conflict');
  assert.deepEqual((await probe('http-save-A')).counts,counts.committed.counts);
  await restored.discard({acknowledgePossibleCommit:true});
  // Deletion has the same lost-ack ambiguity and authorized tombstone receipt replay.
  await restored.stage({type:'delete'});await restored.begin({expected:2n,idempotency:'http-delete'});await control('drop',{id:'http-one',key:'http-delete'});await assert.rejects(restored.retry());await control('resume');
  const delayedRecord=readFileSync(file,'utf8');
  counts.deleted=await probe('http-delete','delete');assert.equal(counts.deleted.row.revision,3);assert.equal(counts.deleted.row.value,null);assert.deepEqual(counts.deleted.counts,[1,3,3,1]);
  restored.dispose();await control('restart');const deleted=createMutationRecovery(options(client()));await deleted.restore();await deleted.retry();
  counts.delete_replayed=await probe('http-delete','delete');assert.deepEqual(counts.delete_replayed.counts,counts.deleted.counts);
  // Durable principal differs from live transport credentials. Switching principal cannot dispatch.
  await deleted.rebind({client:client('bob'),principal:{...principal,subject:'bob'}});await assert.rejects(deleted.retry());assert.equal(deleted.state.phase,'quarantined');assert.equal(deleted.state.result,null);
  await control('retire');
  let delayed=JSON.parse(delayedRecord); const lateStore={async read(){return delayed;},async compareExchange(_slot,version,next){if((delayed?.version??null)!==version)return false;delayed=next;return true;}};
  const expired=createMutationRecovery({...options(client()),store:lateStore});await expired.restore();await assert.rejects(expired.retry(),error=>error.category==='identity_expired');assert.equal(expired.state.commitKnowledge,'unknown');
  counts.retired=await probe('http-delete','delete');assert.equal(counts.retired.row.revision,3);
  trace=await control('trace',{id:'http-one'});assert.doesNotMatch(readFileSync(file,'utf8'),/fixture-owner|fixture-csrf|host_stamp|generation/);
  writeFileSync(join(evidence,'http-results.json'),JSON.stringify({adapter:process.env.ROM_RECOVERY_ADAPTER,counts,trace,exact_first_and_replay:true,process_restart:true,pending_store_limit:'single-process synchronous file CAS with file/directory fsync; no power-loss certification'},null,2)+'\n');
  console.log(JSON.stringify({completed:true,adapter:process.env.ROM_RECOVERY_ADAPTER,counts}));
} finally { await proxy.close(); }
