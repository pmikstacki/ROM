import test from 'node:test';
import assert from 'node:assert/strict';
import { nativeTlsEnvironment, selectNativeTlsCase, assertNativeTlsOutcome } from './native-tls-profile.mjs';

test('native trust is child scoped and cannot inherit proxy or verification bypasses', () => {
  const env = nativeTlsEnvironment({ ca: '/private/ca.pem', emptyCaDirectory: '/private/empty', home: '/private/home', temporary: '/private/tmp' });
  assert.equal(env.SSL_CERT_FILE, '/private/ca.pem');
  assert.equal(env.SSL_CERT_DIR, '/private/empty');
  for (const key of ['HTTPS_PROXY', 'HTTP_PROXY', 'ALL_PROXY', 'NODE_TLS_REJECT_UNAUTHORIZED', 'RUSTLS_NATIVE_CERTS']) assert.equal(key in env, false);
});

test('native negative cases require handoff before the selected Rust acquisition route', () => {
  for (const route of ['token', 'jwks']) for (const certificate of ['wrong-ca', 'wrong-name', 'expired']) {
    const selected = selectNativeTlsCase({ route, certificate, adapter: 'sqlite' });
    assert.equal(selected.rotate_after, route === 'token' ? 'authorization-code-held' : 'token-forwarded');
    assert.equal(selected.fresh_host, true);
    assert.equal(selected.provider_backchannel, false);
  }
  assert.throws(() => selectNativeTlsCase({ route: 'authorization', certificate: 'expired', adapter: 'sqlite' }));
  assert.throws(() => selectNativeTlsCase({ route: 'token', certificate: 'trusted', adapter: 'mysql' }));
});

test('browser TLS rejection or no actual native request is not native evidence', () => {
  const selected = selectNativeTlsCase({ route: 'jwks', certificate: 'expired', adapter: 'redb' });
  const observation = { authorization_code_held: true, token_forwarded: 1, tls_errors: 1, native_callback_status: 401, authenticated: false, protected_read_status: 401, protected_value: false, native_tcp_connections: 2, native_tls_handshakes: 1, token_http_requests: 1, jwks_http_requests: 0, target_route: 'jwks', child_trust_fenced: true };
  assert.doesNotThrow(() => assertNativeTlsOutcome(selected, observation));
  assert.throws(() => assertNativeTlsOutcome(selected, { ...observation, native_tcp_connections: 0 }));
  assert.throws(() => assertNativeTlsOutcome(selected, { ...observation, token_forwarded: 0 }));
  assert.throws(() => assertNativeTlsOutcome(selected, { ...observation, protected_value: true }));
  assert.throws(() => assertNativeTlsOutcome(selected, { ...observation, target_route: 'token' }));
});

test('positive evidence requires actual native token and JWKS traffic and authorized read', () => {
  const selected = selectNativeTlsCase({ route: 'token', certificate: 'trusted', adapter: 'sqlite' });
  const observation = { authorization_code_held: true, token_forwarded: 1, tls_errors: 0, native_callback_status: 303, authenticated: true, protected_read_status: 200, protected_value: true, native_tcp_connections: 2, native_tls_handshakes: 2, token_http_requests: 1, jwks_http_requests: 1, target_route: 'token', child_trust_fenced: true };
  assert.doesNotThrow(() => assertNativeTlsOutcome(selected, observation));
  assert.throws(() => assertNativeTlsOutcome(selected, { ...observation, jwks_http_requests: 0 }));
});

test('a raw TCP connection alone cannot establish completed native TLS', () => {
  const selected = selectNativeTlsCase({ route: 'token', certificate: 'trusted', adapter: 'sqlite' });
  const rawOnly = { authorization_code_held: true, token_forwarded: 1, tls_errors: 0, native_callback_status: 303, authenticated: true, protected_read_status: 200, protected_value: true, native_tls_connected: 1, native_tcp_connections: 1, native_tls_handshakes: 0, token_http_requests: 1, jwks_http_requests: 1, target_route: 'token', child_trust_fenced: true };
  assert.throws(() => assertNativeTlsOutcome(selected, rawOnly));
});

import * as nativeTlsProfile from './native-tls-profile.mjs';
test('provider admission rejects one millisecond below the declared complete case budget', () => {
  assert.equal(typeof nativeTlsProfile.requireNativeTlsProviderWindow, 'function');
  const now = 1791492279895;
  assert.throws(() => nativeTlsProfile.requireNativeTlsProviderWindow(now + 209999, now), /provider readiness window/);
  assert.doesNotThrow(() => nativeTlsProfile.requireNativeTlsProviderWindow(now + 210000, now));
  assert.equal(nativeTlsProfile.NATIVE_TLS_PROVIDER_REMAINING_MS, 210000);
});

test('provider admission rejects invalid deadline and injected clock values', () => {
  assert.equal(typeof nativeTlsProfile.requireNativeTlsProviderWindow, 'function');
  const now = 1791492279895;
  for (const deadline of [undefined, null, '210000', NaN, Infinity, -1, now + 209999.5, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => nativeTlsProfile.requireNativeTlsProviderWindow(deadline, now), /provider readiness window/);
  }
  for (const clock of [null, '0', NaN, Infinity, -1, now + 0.5, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => nativeTlsProfile.requireNativeTlsProviderWindow(now + 210000, clock), /provider readiness window/);
  }
});

import { readFileSync } from 'node:fs';
test('preparation metadata binds its provider deadline to the actual shared admission constant', () => {
  const preparationSource = readFileSync(new URL('./native-tls-prepare.mjs', import.meta.url), 'utf8');
  assert.match(preparationSource, /import \{[^}]*\bNATIVE_TLS_PROVIDER_REMAINING_MS\b[^}]*\} from '\.\/native-tls-profile\.mjs'/);
  assert.match(preparationSource, /provider_remaining_ms: NATIVE_TLS_PROVIDER_REMAINING_MS\b/);
  assert.doesNotMatch(preparationSource, /provider_remaining_ms:\s*\d/);
});
