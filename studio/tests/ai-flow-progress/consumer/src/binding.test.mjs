import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createBinding,readBoundView} from './binding.mjs';
for (const transition of ['quarantine','dispose']) for (const response of ['authorized','refused']) {
  test(`late ${response} view cannot repopulate projection or error after ${transition}`,async()=>{
    const binding=createBinding();let resolve,reject;const deferred=new Promise((yes,no)=>{resolve=yes;reject=no;});
    let view=null,message='';const pending=readBoundView(binding,()=>deferred,value=>{view=value;},error=>{message=error.message;});
    binding[transition]();view=null;message='Signed out';
    if(response==='authorized')resolve({state:'Completed',output:{private:'earlier-owner-output'}});else reject(Error('earlier-owner-error'));
    await pending;assert.equal(view,null);assert.equal(message,'Signed out');
  });
}
test('a currently authorized view and current refusal remain visible',async()=>{
  const binding=createBinding();let view=null,message='';await readBoundView(binding,async()=>({state:'Queued'}),value=>{view=value;},()=>{});assert.equal(view.state,'Queued');
  await readBoundView(binding,async()=>{throw Error('current refusal');},()=>{},error=>{message=error.message;});assert.equal(message,'current refusal');
});
