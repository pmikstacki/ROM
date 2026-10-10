import test from'node:test';import assert from'node:assert/strict';import{admitSyntheticAccount}from'./provider-account-control.mjs';
test('unlinked account projection contains synthetic identity and no credential',()=>{
 const value={pk:3,username:'rom-probe-012345678901234567890123',uid:'provider-generated-uid',is_active:true,type:'internal',password:'private-secret'};
 const projected=admitSyntheticAccount(value,value.username);assert.deepEqual(projected,{id:3,username:value.username,uid:value.uid});assert.equal(JSON.stringify(projected).includes('private-secret'),false);
 for(const changed of[{pk:0},{username:'existing-account'},{is_active:false},{type:'service_account'}])assert.throws(()=>admitSyntheticAccount({...value,...changed},value.username));
});
