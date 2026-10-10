import {test} from 'node:test';
import assert from 'node:assert/strict';
import {failureObservation,redactedLog} from './failure-observation.mjs';
test('private failure observation preserves phase/exit/errno without exception messages or capabilities',()=>{
  const key='a'.repeat(64),error=Object.assign(Error(`secret ${key}`),{code:'EEXIST'});
  const value=failureObservation({stage:'case_directory',route:'/__fixture/start',case:'case-sqlite-fixed',error,native:{code:1,signal:null,spawn_errno:'ENOENT',stdout_bytes:0,stderr_bytes:12,threads:8}});
  assert.equal(value.stage,'case_directory');assert.equal(value.errno,'EEXIST');assert.equal(value.native.code,1);assert.equal(value.native.threads,8);assert.ok(!JSON.stringify(value).includes(key));assert.ok(!JSON.stringify(value).includes('secret'));
});
test('host log redaction removes full capabilities even across stream chunks and terminal flush',()=>{
  const key='0123456789abcdef'.repeat(4),control='fedcba9876543210'.repeat(4);let output='';const logger=redactedLog(text=>{output+=text;},[key,control]);
  logger.append(Buffer.from('native '+key.slice(0,20)));logger.append(Buffer.from(key.slice(20)+' error '+control.slice(0,50)));logger.append(Buffer.from(control.slice(50)+'\n'));logger.flush();assert.equal(output,'native [redacted] error [redacted]\n');
});
