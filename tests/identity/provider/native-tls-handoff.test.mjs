import test from 'node:test';
import assert from 'node:assert/strict';
import { NativeTlsHandoff } from './native-tls-handoff.mjs';

test('native code handoff refuses rotation while the browser is still live', () => {
  const handoff = new NativeTlsHandoff('token');
  handoff.authorizationHeld();
  assert.throws(() => handoff.arm());
  handoff.browserDrained();
  assert.equal(handoff.arm(), 'rotate-before-callback');
  assert.throws(() => handoff.arm());
});

test('JWKS rotation waits for an original token response and forces a new connection', () => {
  const handoff = new NativeTlsHandoff('jwks');
  handoff.authorizationHeld(); handoff.browserDrained();
  assert.equal(handoff.arm(), 'wait-for-token');
  assert.throws(() => handoff.beforeJwks());
  assert.throws(() => handoff.tokenForwarded(401));
  assert.equal(handoff.tokenForwarded(200), 'rotate-and-close-native-connection');
  assert.throws(() => handoff.beforeJwks());
  handoff.nativeConnectionClosed();
  assert.doesNotThrow(() => handoff.beforeJwks());
  assert.throws(() => handoff.tokenForwarded(200));
});

test('a held original callback permits form-navigation abort, but other submit failures do not', async () => {
  const { finishNativeAuthorizationSubmission } = await import('./native-tls-handoff.mjs');
  await assert.doesNotReject(finishNativeAuthorizationSubmission(Promise.reject(Error('navigation-aborted')), Promise.resolve(), () => true));
  await assert.rejects(finishNativeAuthorizationSubmission(Promise.reject(Error('form-failed')), Promise.resolve(), () => false));
  await assert.rejects(finishNativeAuthorizationSubmission(Promise.resolve(), Promise.reject(Error('handoff-deadline')), () => true));
});
