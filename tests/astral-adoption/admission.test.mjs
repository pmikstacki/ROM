import test from 'node:test';
import assert from 'node:assert/strict';
import { requireBrowserWindow, requireRecoveryResult } from './admission.mjs';
const now = 1791457000000;
const ready = {status:'ready',unique:'a'.repeat(32),ready_deadline_unix_ms:now+330000};
const witness = {planned_browser_ms:240000,required_provider_window_ms:330000};
test('exact remaining budget admits the declared browser phase',()=>assert.equal(requireBrowserWindow(ready,witness,now),330000));
test('expired or insufficient provider window refuses a new browser phase',()=>{
 for(const remaining of [329999,0,-1])assert.throws(()=>requireBrowserWindow({...ready,ready_deadline_unix_ms:now+remaining},witness,now),/remaining/);
});
test('unbounded or changed phase declaration refuses admission',()=>{
 for(const changed of [{planned_browser_ms:Infinity},{planned_browser_ms:240001},{required_provider_window_ms:285000}])assert.throws(()=>requireBrowserWindow(ready,{...witness,...changed},now),/declared/);
 assert.throws(()=>requireBrowserWindow({...ready,status:'fixture-failed'},witness,now),/provider/);
});
test('all four actual cases must pass without skipped or failed tests',()=>{
 const good={stats:{expected:4,unexpected:0,skipped:0,flaky:0}};
 assert.equal(requireRecoveryResult(good),4);
 for(const stats of [{expected:3,unexpected:0,skipped:1,flaky:0},{expected:3,unexpected:1,skipped:0,flaky:0},{expected:4,unexpected:0,skipped:0,flaky:1},{expected:0,unexpected:0,skipped:0,flaky:0}])assert.throws(()=>requireRecoveryResult({stats}),/four/);
});
