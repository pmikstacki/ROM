import {test} from 'node:test';
import assert from 'node:assert/strict';
import {awaitPhysicalRead} from './consumer/tests/read-progression.mjs';
const queued=()=>({view:{state:'ToolsPending',read_progress:{status:'Queued'}},physical_started:false,read_calls:0,provider_calls:1});
const active=()=>({view:{state:'ToolsPending',read_progress:{status:'Active'}},physical_started:true,read_calls:1,provider_calls:1});
test('earlier Source work advances one at a time and stops at the first physical read',async()=>{
  const trace=[],values=[queued(),queued(),active()];
  const actual=await awaitPhysicalRead({step:async()=>trace.push('step'),inspect:async()=>{trace.push('inspect');return values.shift();}});
  assert.deepEqual(actual,active());assert.deepEqual(trace,['step','inspect','step','inspect','step','inspect']);assert.equal(values.length,0);
});
test('sixteen acknowledged queued items exhaust the finite budget without an extra mutation',async()=>{
  let steps=0,inspections=0;
  await assert.rejects(awaitPhysicalRead({step:async()=>{steps++;},inspect:async()=>{inspections++;return queued();}}),/physical read did not start within 16 work items/);
  assert.equal(steps,16);assert.equal(inspections,16);
});
test('inconsistent progress, duplicate callback or provider rerun stop before another step',async()=>{
  for(const value of [{...active(),read_calls:2},{...active(),provider_calls:2},{...active(),physical_started:false},{...queued(),physical_started:true},{...queued(),view:{state:'Failed',read_progress:null}}]){
    let steps=0;await assert.rejects(awaitPhysicalRead({step:async()=>{steps++;},inspect:async()=>value}),/unexpected read progression/);assert.equal(steps,1);
  }
});
