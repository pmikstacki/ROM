import test from 'node:test';
import assert from 'node:assert/strict';
import { forwardedHeaders, admitProxyRequest } from './proxy-contract.mjs';
test('owned proxies replace forwarding authority without changing browser Origin or CSRF', () => {
 const headers=forwardedHeaders({host:'evil.invalid','x-forwarded-host':'evil.invalid','x-forwarded-proto':'http','x-forwarded-for':'203.0.113.1',forwarded:'host=evil.invalid',origin:'https://wrong.invalid','x-rom-csrf':'synthetic','content-type':'application/json'},'provider');
 assert.equal(headers.host,'127.0.0.1:44392');assert.equal(headers['x-forwarded-proto'],'https');assert.equal(headers['x-forwarded-host'],'127.0.0.1:44392');assert.equal(headers['x-forwarded-for'],undefined);assert.equal(headers.forwarded,undefined);assert.equal(headers.origin,'https://wrong.invalid');assert.equal(headers['x-rom-csrf'],'synthetic');
 assert.equal(forwardedHeaders({},'host').host,'127.0.0.1:44389');assert.throws(()=>forwardedHeaders({},'unapproved'));
});
test('private provider proxy admits only approved method and exact token/JWKS paths', () => {
 assert.doesNotThrow(()=>admitProxyRequest('private','POST','/application/o/token/'));
 assert.doesNotThrow(()=>admitProxyRequest('private','GET','/application/o/rom-synthetic-identity/jwks/'));
 for(const [method,path] of [['GET','/application/o/token/'],['POST','/api/v3/admin/'],['GET','/application/o/rom-synthetic-identity/jwks/?issuer=other'],['GET','http://example.invalid/'],['GET','//example.invalid/']])assert.throws(()=>admitProxyRequest('private',method,path));
 assert.doesNotThrow(()=>admitProxyRequest('host','POST','/rom-studio/read'));assert.throws(()=>admitProxyRequest('host','GET','/api/'));
});
