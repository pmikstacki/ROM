import test from 'node:test';
import assert from 'node:assert/strict';
import { browserProfile, requireSessionCookie } from './browser-profile.mjs';
import { loginAstral } from './browser-login.mjs';

const http = 'http://127.0.0.1:43902';
const tls = 'https://rom-astral.test:43904';
const cookie = profile => ({ name: 'rom_session', domain: profile.hostname,
  httpOnly: true, secure: profile.secure, sameSite: 'Lax', value: 'opaque-session' });

test('allocated HTTP and TLS lanes have distinct cookie contracts', () => {
  assert.deepEqual(browserProfile(http), { origin: http, hostname: '127.0.0.1', secure: false });
  assert.deepEqual(browserProfile(tls), { origin: tls, hostname: 'rom-astral.test', secure: true });
});

test('unallocated origins and URL decorations are rejected before browser navigation', () => {
  for (const origin of [undefined, null, {}, '', http + '/', http + '/auth', http + '?q=1',
    http + '#fragment', 'http://user@127.0.0.1:43902', 'http://127.0.0.1:43903',
    'https://rom-astral.test:43905', 'https://example.com', 'https://127.0.0.1:43904']) {
    assert.throws(() => browserProfile(origin), /allocated browser origin/);
  }
});

test('HTTP cookies cannot satisfy the TLS lane and TLS cookies cannot satisfy HTTP', () => {
  for (const origin of [http, tls]) {
    const profile = browserProfile(origin);
    assert.doesNotThrow(() => requireSessionCookie([cookie(profile)], profile));
    assert.throws(() => requireSessionCookie([{ ...cookie(profile), secure: !profile.secure }], profile), /session cookie/);
  }
});

test('session cookie must have the exact host, HttpOnly flag and bounded opaque value', () => {
  const profile = browserProfile(tls);
  for (const patch of [{ domain: '.rom-astral.test' }, { domain: 'example.com' },
    { httpOnly: false }, { httpOnly: 'true' }, { value: undefined }, { value: 12 },
    { value: '' }, { value: 'has space' }, { value: 'x'.repeat(513) }]) {
    assert.throws(() => requireSessionCookie([{ ...cookie(profile), ...patch }], profile), /session cookie/);
  }
});

test('missing or duplicate sessions fail while unrelated cookies are ignored', () => {
  const profile = browserProfile(tls);
  assert.throws(() => requireSessionCookie([], profile), /session cookie/);
  assert.throws(() => requireSessionCookie([cookie(profile), cookie(profile)], profile), /session cookie/);
  assert.doesNotThrow(() => requireSessionCookie([{ name: 'provider_session' }, cookie(profile)], profile));
});

test('a caller cannot forge the profile to bypass the allocated cookie contract', () => {
  const profile = { ...browserProfile(tls), secure: false };
  assert.throws(() => requireSessionCookie([cookie(profile)], profile), /browser profile/);
});

test('login rejects an unallocated origin before accessing the browser', async () => {
  let accessed = false;
  const page = new Proxy({}, { get() { accessed = true; throw Error('browser accessed'); } });
  await assert.rejects(loginAstral(page, {
    password: 'private-test-input', expectedUser: 'astral-fixture-user', origin: 'https://example.com',
  }), /allocated browser origin/);
  assert.equal(accessed, false);
});

test('both lanes require the deployed Host SameSite=Lax cookie contract', () => {
  for (const origin of [http, tls]) {
    const profile = browserProfile(origin);
    for (const sameSite of [undefined, 'None', 'Strict', 'lax']) {
      assert.throws(() => requireSessionCookie([{ ...cookie(profile), sameSite }], profile), /session cookie/);
    }
  }
});
