import test from 'node:test';
import assert from 'node:assert/strict';
import { admitTlsObservation } from './tls-observation.mjs';
test('TLS prerequisite requires the synthetic page and an actual request for trusted positive', () => {
  assert.doesNotThrow(() => admitTlsObservation('trusted', { navigated: true, title: 'ROM isolated TLS trust fixture', requests: 1, navigation_error: null }));
  for (const value of [{ navigated: true, title: 'other', requests: 1 }, { navigated: true, title: 'ROM isolated TLS trust fixture', requests: 0 }, { navigated: false, requests: 0, navigation_error: 'certificate rejected' }]) assert.throws(() => admitTlsObservation('trusted', value));
});
test('negative TLS admission rejects connection failures and successful navigation', () => {
  for (const [expected, code] of [['untrusted-ca', 'AUTHORITY_INVALID'], ['wrong-san', 'COMMON_NAME_INVALID'], ['expired', 'DATE_INVALID']]) {
    assert.doesNotThrow(() => admitTlsObservation(expected, { navigated: false, requests: 0, navigation_error: `net::ERR_CERT_${code}` }));
    for (const error of ['net::ERR_CONNECTION_REFUSED', 'Timeout 10000ms exceeded', 'TLS support is not available', 'SSL backend unavailable', null]) assert.throws(() => admitTlsObservation(expected, { navigated: false, requests: 0, navigation_error: error }));
    assert.throws(() => admitTlsObservation(expected, { navigated: true, requests: 1, navigation_error: 'certificate' }));
  }
  assert.throws(() => admitTlsObservation('unknown', {}));
});
