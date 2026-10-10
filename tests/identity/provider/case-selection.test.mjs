import test from'node:test';import assert from'node:assert/strict';import{selectIdentityCases}from'./case-selection.mjs';
test('focused provider repetitions retain closed adapter and engine identity',()=>{
 assert.equal(selectIdentityCases().length,4);assert.deepEqual(selectIdentityCases(['sqlite/chromium']),[{adapter:'sqlite',engine:'chromium'}]);
 for(const value of[[],['sqlite/chromium','sqlite/chromium'],['other/chromium'],['sqlite/other'],'sqlite/chromium',[{adapter:'sqlite',engine:'chromium'}]])assert.throws(()=>selectIdentityCases(value));
});
