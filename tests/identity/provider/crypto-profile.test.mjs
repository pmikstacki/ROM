import {test} from 'node:test';
import assert from 'node:assert/strict';
import {selectCryptoLibrary, cryptoPackages} from './crypto-profile.mjs';

test('crypto projection retains exact regular bytes behind an admitted SONAME link',()=>{
 const bytes=Buffer.from('immutable ELF fixture');
 const members=new Map([['usr/lib/x86_64-linux-gnu/libgnutls.so.30',{type:'2',link:'usr/lib/x86_64-linux-gnu/libgnutls.so.30.40.3'}],['usr/lib/x86_64-linux-gnu/libgnutls.so.30.40.3',{type:'0',bytes}]]);
 assert.equal(selectCryptoLibrary(members,cryptoPackages[0]).bytes,bytes);
 assert.equal(selectCryptoLibrary(members,cryptoPackages[0]).filename,'libgnutls.so.30');
});
test('crypto projection rejects absent, chained, escaping and unrelated SONAME targets',()=>{
 for(const link of ['usr/lib/x86_64-linux-gnu/libother.so','../../escape','usr/lib/x86_64-linux-gnu/libgnutls.so.30.40.3']){
  const members=new Map([['usr/lib/x86_64-linux-gnu/libgnutls.so.30',{type:'2',link}], [link,{type:'2',link:'next'}]]);
  assert.throws(()=>selectCryptoLibrary(members,cryptoPackages[0]));
 }
 assert.throws(()=>selectCryptoLibrary(new Map(),cryptoPackages[0]));
});
test('crypto projection rejects empty or oversized bytes and an unapproved profile',()=>{
 const name='usr/lib/x86_64-linux-gnu/libtasn1.so.6';
 for(const bytes of [Buffer.alloc(0),Buffer.alloc(8*1024**2+1)])assert.throws(()=>selectCryptoLibrary(new Map([[name,{type:'0',bytes}]]),cryptoPackages[1]));
 assert.throws(()=>selectCryptoLibrary(new Map(),{soname:'libunknown.so'}));
});
