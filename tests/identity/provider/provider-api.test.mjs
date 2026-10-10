import test from'node:test';import assert from'node:assert/strict';import{admitProviderApiRequest}from'./provider-api.mjs';
test('synthetic provider roles admit only their closed endpoints and methods',()=>{
 assert.doesNotThrow(()=>admitProviderApiRequest('account','/api/v3/core/users/','POST'));
 assert.doesNotThrow(()=>admitProviderApiRequest('account','/api/v3/core/users/123/set_password/','POST'));
 assert.doesNotThrow(()=>admitProviderApiRequest('key','/api/v3/providers/oauth2/1/','PATCH'));
 for(const args of[['key','/api/v3/core/users/','POST'],['account','/api/v3/providers/oauth2/1/','PATCH'],['account','/api/v3/core/users/0/set_password/','POST'],['account','/api/v3/core/users/123/set_password/','DELETE'],['account','http://other.invalid/','POST'],['arbitrary','/api/v3/core/users/','POST']])assert.throws(()=>admitProviderApiRequest(...args));
});
