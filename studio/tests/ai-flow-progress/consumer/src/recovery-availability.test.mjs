import {test} from 'node:test';
import assert from 'node:assert/strict';
import {allowsRecovery} from './recovery.mjs';
test('terminal public states with restored grants do not offer recovery',()=>{
  for(const state of ['Completed','Cancelled','Failed'])assert.equal(allowsRecovery({state}),false);
  assert.equal(allowsRecovery(null),false);
  for(const state of ['ToolsPending','AwaitingReconciliation',{Waiting:{retry_at_unix_ms:0}}])assert.equal(allowsRecovery({state}),true);
});
