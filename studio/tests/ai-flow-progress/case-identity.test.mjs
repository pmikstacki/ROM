import {test} from 'node:test';
import assert from 'node:assert/strict';
import {browserCase} from './consumer/tests/case-identity.mjs';
import {requireCase} from './protocol.mjs';
test('case identity survives worker restart and differs across actual test/engine/retry',()=>{
  const identity={adapter:'sqlite',domain:'publication',mode:'normal',project:'chromium',testId:'fixed-publication-recovery-test',retry:0,repeat:0};
  assert.equal(browserCase(identity),browserCase({...identity}));
  const cases=[identity,{...identity,testId:'fixed-publication-cancel-test'},{...identity,project:'webkit'},{...identity,retry:1},{...identity,domain:'triage'}].map(browserCase);
  assert.equal(new Set(cases).size,5);for(const value of cases)requireCase({case:value,domain:'publication',mode:'normal'});
  assert.throws(()=>browserCase({...identity,project:'unconfigured-engine'}),/case identity/);
});
