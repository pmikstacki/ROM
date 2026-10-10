import test from'node:test';import assert from'node:assert/strict';import{projectTokenTiming}from'./token-timing.mjs';
test('token timing projection keeps bounds and hashes without token or claims disclosure',()=>{const token=`e30.${Buffer.from(JSON.stringify({iat:100,exp:400,sub:'private'})).toString('base64url')}.signature`;const v=projectTokenTiming(Buffer.from(JSON.stringify({id_token:token})),100000);assert.equal(v.exp,400);assert.equal(v.iat,100);assert.equal(v.received_at_ms,100000);assert.equal(v.token_sha256.length,64);assert.equal(JSON.stringify(v).includes('private'),false);assert.throws(()=>projectTokenTiming(Buffer.alloc(65537),100000));assert.throws(()=>projectTokenTiming(Buffer.from('{"id_token":"malformed"}'),100000));});
test('original token key identifier is projected without private claims',()=>{
 const token=`${Buffer.from(JSON.stringify({kid:'actual-provider-key-1',alg:'RS256'})).toString('base64url')}.${Buffer.from(JSON.stringify({iat:100,exp:400,sub:'private'})).toString('base64url')}.signature`;
 assert.equal(projectTokenTiming(Buffer.from(JSON.stringify({id_token:token})),100000).kid,'actual-provider-key-1');
});
test('original subject comparison uses hashes without disclosing the subject',()=>{
 const token=subject=>`e30.${Buffer.from(JSON.stringify({iat:100,exp:400,sub:subject})).toString('base64url')}.signature`;
 const a=projectTokenTiming(Buffer.from(JSON.stringify({id_token:token('private-first')})),100000),b=projectTokenTiming(Buffer.from(JSON.stringify({id_token:token('private-second')})),100000);
 assert.equal(a.subject_sha256.length,64);assert.notEqual(a.subject_sha256,b.subject_sha256);assert.equal(JSON.stringify(a).includes('private-first'),false);
});
