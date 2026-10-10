import {test} from 'node:test';
import assert from 'node:assert/strict';
import {expiryDeadline} from './token-expiry.mjs';
test('expiry oracle waits past the original provider exp with a finite deadline',()=>{
 assert.equal(expiryDeadline({iat:100,exp:140,received_at_ms:100100},120000),142000);
 for(const timing of [{iat:100,exp:180,received_at_ms:100100},{iat:100,exp:140,received_at_ms:140000},{iat:100,exp:140,received_at_ms:100100,token:'secret'},{iat:100,exp:140,received_at_ms:Infinity}])assert.throws(()=>expiryDeadline(timing,120000));
 assert.throws(()=>expiryDeadline({iat:100,exp:140,received_at_ms:100100},143000));
});
