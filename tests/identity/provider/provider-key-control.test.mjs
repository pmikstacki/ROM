import test from'node:test';import assert from'node:assert/strict';import{admitSigningKey,admitJwksKeys}from'./provider-key-control.mjs';
test('actual key and JWKS projections require bounded public identifiers',()=>{
 assert.deepEqual(admitSigningKey({pk:'25d4d014-37e7-4f4f-8c3f-d05efe8b3431',key_type:'rsa',private_key_available:true}),{id:'25d4d014-37e7-4f4f-8c3f-d05efe8b3431',type:'rsa'});
 assert.deepEqual(admitJwksKeys({keys:[{kid:'key-1',kty:'RSA',n:'public-modulus'}]}),['key-1']);
 for(const value of[{keys:[]},{keys:[{kid:'private key'}]},{keys:[{kid:'a'},{kid:'a'}]}])assert.throws(()=>admitJwksKeys(value));
 assert.throws(()=>admitSigningKey({pk:'arbitrary',key_type:'rsa',private_key_available:true}));
});
