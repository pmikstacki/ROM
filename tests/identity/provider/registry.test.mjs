import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { verifyDigest, selectPlatform, requireImageManifest } from './registry.mjs';
const digest = bytes => `sha256:${createHash('sha256').update(bytes).digest('hex')}`;

test('image metadata retains digest identity and rejects byte drift', () => {
  const bytes = Buffer.from('official manifest');
  assert.doesNotThrow(() => verifyDigest(bytes, digest(bytes)));
  assert.throws(() => verifyDigest(Buffer.from('changed'), digest(bytes)), /image digest/);
});

test('platform selection rejects missing and ambiguous descriptors', () => {
  const entry = { digest: digest('manifest'), platform: { os: 'linux', architecture: 'amd64' } };
  assert.equal(selectPlatform({ manifests: [entry] }, 'linux/amd64'), entry.digest);
  assert.throws(() => selectPlatform({ manifests: [entry] }, 'linux/arm64'), /image platform/);
  assert.throws(() => selectPlatform({ manifests: [entry, entry] }, 'linux/amd64'), /image platform/);
});

test('manifest layer inventory rejects foreign URLs and oversized acquisition budgets', () => {
  const manifest = { config: { digest: digest('config') }, layers: [{ digest: digest('layer'), size: 100 }] };
  assert.equal(requireImageManifest(manifest), 100);
  assert.throws(() => requireImageManifest({ ...manifest, layers: [{ ...manifest.layers[0], urls: ['https://outside.invalid/'] }] }), /image manifest/);
  assert.throws(() => requireImageManifest({ ...manifest, layers: [{ ...manifest.layers[0], size: 7 * 1024 ** 3 }] }), /image manifest/);
});

test('blob redirects allow only the role CDN and never forward registry authorization', async () => {
  const { fetchPublicBlob } = await import('./registry.mjs');
  const calls = [];
  const request = async (url, options) => {
    calls.push({ url: String(url), options });
    return calls.length === 1 ? new Response(null, { status: 307, headers: { location: 'https://pkg-containers.githubusercontent.com/blob?opaque=fixture' } }) : new Response('config');
  };
  assert.equal((await fetchPublicBlob('authentik', 'https://ghcr.io/v2/goauthentik/server/blobs/sha256:fixture', { Authorization: 'Bearer fixture' }, request)).toString(), 'config');
  assert.equal(calls[0].options.headers.Authorization, 'Bearer fixture');
  assert.deepEqual(calls[1].options.headers, {});
  assert.equal(calls[1].options.redirect, 'manual');
});

test('blob redirect policy rejects insecure, unrelated, credentialed, cross-role and repeated redirects', async () => {
  const { fetchPublicBlob } = await import('./registry.mjs');
  for (const location of ['http://pkg-containers.githubusercontent.com/blob', 'https://pkg-containers.githubusercontent.com.evil.invalid/blob', 'https://user@pkg-containers.githubusercontent.com/blob', 'https://pkg-containers.githubusercontent.com:444/blob', 'https://production.cloudfront.docker.com/blob']) {
    let calls = 0;
    await assert.rejects(fetchPublicBlob('authentik', 'https://ghcr.io/v2/goauthentik/server/blobs/fixture', {}, async () => { calls++; return new Response(null, { status: 307, headers: { location } }); }), /blob redirect/);
    assert.equal(calls, 1);
  }
  await assert.rejects(fetchPublicBlob('authentik', 'https://ghcr.io/v2/goauthentik/server/blobs/fixture', {}, async () => new Response(null, { status: 307, headers: { location: 'https://pkg-containers.githubusercontent.com/blob' } })), /blob redirect/);
});
