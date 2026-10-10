import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyGioClosure } from './gio-verification.mjs';
const name='00000000000000000000000000000000-test';
function fixture() { return { expected:{records:[{name,fields:{References:[''],NarHash:['sha256:'+'0'.repeat(52)],NarSize:['8']}}]},actual:{['/nix/store/'+name]:{narHash:'sha256-'+Buffer.alloc(32).toString('base64'),narSize:8,references:[],signatures:['cache.nixos.org-1:signature']}} }; }
test('closure verification compares hash, size, complete references and exact path set', () => {
 const {expected,actual}=fixture();assert.equal(verifyGioClosure(expected,actual).length,1);
 for(const field of ['hash','size','reference','signature','path']){const {expected,actual}=fixture(),value=actual['/nix/store/'+name];if(field==='hash')value.narHash='sha256-'+Buffer.alloc(32,1).toString('base64');if(field==='size')value.narSize++;if(field==='reference')value.references=['/nix/store/unadmitted'];if(field==='signature')value.signatures=[];if(field==='path')actual['/nix/store/extra']={...value};assert.throws(()=>verifyGioClosure(expected,actual));}
});
