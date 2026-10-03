import assert from 'node:assert/strict';
import test from 'node:test';
import { randomBytes } from 'node:crypto';
import { startProvider, issueToken, introspect } from './provider.mjs';

test('actual provider preserves service profile and loses opaque tokens at restart', { timeout: 15000 }, async () => {
  const serviceSecret = randomBytes(32).toString('hex');
  const introspectionSecret = randomBytes(32).toString('hex');
  const first = await startProvider({ serviceSecret, introspectionSecret });
  let second;
  try {
    const token = await issueToken(first.issuer, serviceSecret);
    const verified = await introspect(first.issuer, introspectionSecret, token);
    assert.equal(verified.active, true);
    assert.equal(verified.iss, first.issuer);
    assert.equal(verified.aud, 'rom-api');
    assert.equal(verified.sub, 'rom-service');
    assert.equal(verified.client_id, verified.sub);
    assert.equal(verified.principal_kind, 'service');
    assert.equal(verified.token_type, 'Bearer');
    assert.ok(!Object.hasOwn(verified, 'cnf'));
    assert.equal((await introspect(first.issuer, introspectionSecret, 'never-issued')).active, false);
    await assert.rejects(issueToken(first.issuer, 'wrong-secret'), { message: 'provider request rejected' });
    const port = first.port;
    await first.close();
    second = await startProvider({ serviceSecret, introspectionSecret, port });
    assert.equal(second.issuer, first.issuer);
    assert.equal((await introspect(second.issuer, introspectionSecret, token)).active, false);
    const fresh = await issueToken(second.issuer, serviceSecret);
    assert.equal((await introspect(second.issuer, introspectionSecret, fresh)).active, true);
  } finally {
    await first.close();
    await second?.close();
  }
});
