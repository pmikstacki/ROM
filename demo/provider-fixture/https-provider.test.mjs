import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash, createPublicKey, randomBytes, verify } from 'node:crypto';
import { startHttpsProvider } from './https-provider.mjs';
const issuer = 'https://preview.example/rom-studio-provider';
const redirectUri = 'https://preview.example/rom-studio/auth/callback/local';
const clientId = 'studio';
test('trusted preview rejects ambiguous issuer and callback mounts', async () => {
  for (const [i, r] of [
    ['http://preview.example/idp', redirectUri], [issuer + '/', redirectUri],
    [issuer + '?x=1', redirectUri], ['https://user@preview.example/idp', redirectUri],
    [issuer, 'https://other.example/rom-studio/auth/callback/local'],
    [issuer, 'https://preview.example/rom-studio/callback/local'],
    ['https://preview.example/a/../idp', redirectUri],
  ]) await assert.rejects(startHttpsProvider({ issuer: i, redirectUri: r, clientId, clientSecret: 'secret' }));
});
test('HTTPS issuer profile preserves proxy paths, PKCE and signed public claims', { timeout: 15000 }, async () => {
  const secret = randomBytes(32).toString('base64url');
  const server = await startHttpsProvider({ issuer, redirectUri, clientId, clientSecret: secret });
  const cookies = new Map();
  const request = async (publicUrl, options = {}) => {
    const url = new URL(publicUrl);
    assert.equal(url.origin, new URL(issuer).origin);
    const { backchannel = false, ...fetchOptions } = options;
    const response = await fetch(`http://127.0.0.1:${server.port}${url.pathname}${url.search}`, {
      ...fetchOptions, redirect: 'manual', signal: AbortSignal.timeout(3000),
      headers: { ...(backchannel ? {} : { 'x-forwarded-proto': 'https', 'x-forwarded-host': url.host }), cookie: [...cookies].map(([k,v]) => `${k}=${v}`).join('; '), ...options.headers },
    });
    for (const c of response.headers.getSetCookie()) { const pair=c.split(';',1)[0]; const at=pair.indexOf('='); cookies.set(pair.slice(0,at),pair.slice(at+1)); }
    return response;
  };
  try {
    const discovery = await (await request(`${issuer}/.well-known/openid-configuration`)).json();
    assert.equal(discovery.issuer, issuer);
    assert.equal(discovery.authorization_endpoint, `${issuer}/auth`);
    const verifier = randomBytes(32).toString('base64url');
    const nonce = randomBytes(24).toString('base64url');
    let next = `${issuer}/auth?${new URLSearchParams({client_id:clientId,redirect_uri:redirectUri,response_type:'code',scope:'openid',nonce,state:'bounded-state',code_challenge:createHash('sha256').update(verifier).digest('base64url'),code_challenge_method:'S256'})}`;
    let code;
    for(let n=0;n<16;n++) {
      const response = await request(next);
      if(response.status >=300 && response.status<400) {
        const target = new URL(response.headers.get('location'), next);
        await response.body?.cancel();
        if(target.href.startsWith(redirectUri+'?')) { assert.equal(target.searchParams.get('state'),'bounded-state'); code=target.searchParams.get('code'); break; }
        next=target.href; continue;
      }
      assert.equal(response.status,200);
      const html=await response.text();
      assert.ok(html.includes('not production authentication'));
      const action=/<form method="post" action="([^"]+)">/.exec(html)?.[1];
      assert.ok(action?.startsWith('/rom-studio-provider/interaction/'));
      const submitted=await request(new URL(action,issuer).href,{method:'POST',headers:{'content-type':'application/x-www-form-urlencoded',origin:new URL(issuer).origin},body:new URLSearchParams(action.endsWith('/login')?{account:'alice'}:{consent:'accept'})});
      assert.equal(submitted.status,303);
      next=new URL(submitted.headers.get('location'),issuer).href;
      await submitted.body?.cancel();
    }
    assert.ok(code);
    const token=await request(`${issuer}/token`,{backchannel:true,method:'POST',headers:{authorization:`Basic ${Buffer.from(`${clientId}:${secret}`).toString('base64')}`,'content-type':'application/x-www-form-urlencoded'},body:new URLSearchParams({grant_type:'authorization_code',code,code_verifier:verifier,redirect_uri:redirectUri})});
    assert.equal(token.status,200);
    const body=await token.json();
    const [header,payload,signature]=body.id_token.split('.');
    const keys=await (await request(`${issuer}/jwks`,{backchannel:true})).json();
    const kid=JSON.parse(Buffer.from(header,'base64url')).kid;
    assert.ok(verify('RSA-SHA256',Buffer.from(`${header}.${payload}`),createPublicKey({key:keys.keys.find(k=>k.kid===kid),format:'jwk'}),Buffer.from(signature,'base64url')));
    const claims=JSON.parse(Buffer.from(payload,'base64url'));
    assert.equal(claims.iss,issuer); assert.equal(claims.nonce,nonce); assert.equal(claims.sub,'alice'); assert.equal(claims.aud,clientId);
    const outside=await fetch(`http://127.0.0.1:${server.port}/auth`,{signal:AbortSignal.timeout(3000)});
    assert.equal(outside.status,404);
  } finally { await server.close(); }
});
