import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash, randomBytes } from 'node:crypto';
import { startHumanProvider } from './human-provider.mjs';

const callback = 'http://127.0.0.1:43219/rom-studio/auth/callback/local';
const clientId = 'rom-studio-human';

function browser(issuer) {
  const cookies = new Map();
  return async (url, options = {}) => {
    assert.equal(new URL(url).origin, new URL(issuer).origin);
    const response = await fetch(url, {
      redirect: 'manual', signal: AbortSignal.timeout(3000), ...options,
      headers: { cookie: [...cookies].map(([key, value]) => `${key}=${value}`).join('; '), ...options.headers },
    });
    for (const cookie of response.headers.getSetCookie()) {
      const pair = cookie.split(';', 1)[0];
      const split = pair.indexOf('=');
      cookies.set(pair.slice(0, split), pair.slice(split + 1));
    }
    return response;
  };
}

async function authorize(provider, { subject = 'alice', verifier = randomBytes(32).toString('base64url'), challenge = true } = {}) {
  const state = randomBytes(24).toString('base64url');
  const nonce = randomBytes(24).toString('base64url');
  const query = new URLSearchParams({
    client_id: clientId, redirect_uri: callback, response_type: 'code', scope: 'openid profile', state, nonce,
    ...(challenge ? { code_challenge: createHash('sha256').update(verifier).digest('base64url'), code_challenge_method: 'S256' } : {}),
  });
  const request = browser(provider.issuer);
  let url = `${provider.issuer}/auth?${query}`;
  for (let step = 0; step < 16; step++) {
    const response = await request(url);
    if (response.status >= 300 && response.status < 400) {
      const location = new URL(response.headers.get('location'), url);
      await response.body?.cancel();
      if (location.href.startsWith(callback)) return { result: location, verifier, state, nonce };
      url = location.href;
      continue;
    }
    assert.equal(response.status, 200);
    assert.ok((response.headers.get("content-security-policy") ?? "").includes(`form-action 'self' ${new URL(callback).origin};`), "CSP must allow the configured callback origin");
    const html = await response.text();
    assert.ok(html.length < 16 * 1024);
    const form = /<form method="post" action="([^"]+)">/.exec(html);
    assert.ok(form, 'actual interaction must contain a form');
    const action = new URL(form[1], provider.issuer);
    const login = action.pathname.endsWith('/login');
    const submitted = await request(action, {
      method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams(login ? { account: subject } : { consent: 'accept' }),
    });
    if (submitted.status === 403) return { denied: true };
    assert.ok(submitted.status >= 300 && submitted.status < 400);
    url = new URL(submitted.headers.get('location'), action).href;
    await submitted.body?.cancel();
  }
  throw Error('fixture authorization exceeded redirect bound');
}

async function exchange(provider, secret, code, verifier) {
  const response = await fetch(`${provider.issuer}/token`, {
    method: 'POST', redirect: 'error', signal: AbortSignal.timeout(3000),
    headers: {
      authorization: `Basic ${Buffer.from(`${new URLSearchParams([['',clientId]]).toString().slice(1)}:${new URLSearchParams([['',secret]]).toString().slice(1)}`).toString('base64')}`,
      'content-type': 'application/x-www-form-urlencoded',
    },
    body: new URLSearchParams({ grant_type: 'authorization_code', code, code_verifier: verifier, redirect_uri: callback }),
  });
  return { status: response.status, value: await response.json() };
}

test('actual human provider performs code/PKCE login and one-time exchange', { timeout: 15000 }, async () => {
  const secret = randomBytes(32).toString('base64url');
  const provider = await startHumanProvider({ clientId, clientSecret: secret, redirectUri: callback });
  try {
    const discovery = await (await fetch(`${provider.issuer}/.well-known/openid-configuration`)).json();
    assert.equal(discovery.issuer, provider.issuer);
    assert.ok(discovery.code_challenge_methods_supported.includes('S256'));
    const result = await authorize(provider);
    assert.equal(result.result.searchParams.get('state'), result.state);
    assert.ok(result.result.searchParams.get('code'));
    const issued = await exchange(provider, secret, result.result.searchParams.get('code'), result.verifier);
    assert.equal(issued.status, 200);
    assert.equal(issued.value.token_type, 'Bearer');
    assert.equal(typeof issued.value.id_token, 'string');
    // Protocol fixture assertions only; Rust acceptance must verify the actual signature.
    const claims = JSON.parse(Buffer.from(issued.value.id_token.split('.')[1], 'base64url'));
    assert.equal(claims.iss, provider.issuer);
    assert.equal(claims.sub, 'alice');
    assert.equal(claims.aud, clientId);
    assert.equal(claims.nonce, result.nonce);
    const replay = await exchange(provider, secret, result.result.searchParams.get('code'), result.verifier);
    assert.equal(replay.status, 400);
    assert.equal(replay.value.error, 'invalid_grant');
  } finally { await provider.close(); }
});

test('actual human provider rejects missing PKCE, wrong verifier and undeclared account', { timeout: 15000 }, async () => {
  const secret = randomBytes(32).toString('base64url');
  const provider = await startHumanProvider({ clientId, clientSecret: secret, redirectUri: callback });
  try {
    const missing = await authorize(provider, { challenge: false });
    assert.equal(missing.result.searchParams.get('error'), 'invalid_request');
    const correct = await authorize(provider);
    const wrong = await exchange(provider, secret, correct.result.searchParams.get('code'), randomBytes(32).toString('base64url'));
    assert.equal(wrong.status, 400);
    assert.equal(wrong.value.error, 'invalid_grant');
    assert.equal((await authorize(provider, { subject: 'never-provisioned' })).denied, true);
  } finally { await provider.close(); }
});

test('human fixture refuses non-loopback callbacks and empty credentials', async () => {
  await assert.rejects(startHumanProvider({ clientId, clientSecret: 'secret', redirectUri: 'https://attacker.example/callback' }));
  await assert.rejects(startHumanProvider({ clientId, clientSecret: '', redirectUri: callback }));
});

test('actual provider accepts form-encoded special-character client secret', { timeout: 15000 }, async () => {
  const secret = 'public-test: secret% +!';
  const provider = await startHumanProvider({ clientId, clientSecret: secret, redirectUri: callback });
  try {
    const result = await authorize(provider);
    const issued = await exchange(provider, secret, result.result.searchParams.get('code'), result.verifier);
    assert.equal(issued.status, 200);
    assert.equal(typeof issued.value.id_token, 'string');
  } finally { await provider.close(); }
});
