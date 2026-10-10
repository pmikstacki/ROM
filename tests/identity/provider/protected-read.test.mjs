import test from 'node:test';
import assert from 'node:assert/strict';
import { protectedReadPath, readProtectedResource } from './protected-read.mjs';

test('the browser read uses actual Fetch Response status and projects only safe result fields', async t => {
  t.mock.method(globalThis, 'fetch', async (url, request) => {
    assert.equal(url, '/rom-studio/api/read');
    assert.equal(request.method, 'POST');
    assert.equal(request.headers['x-rom-csrf'], 'synthetic-csrf');
    assert.deepEqual(JSON.parse(request.body), { kind: 'fixture-documents', id: 'private' });
    return new Response(JSON.stringify({ value: { content: 'Synthetic protected fixture document' } }), { status: 200 });
  });
  assert.deepEqual(await readProtectedResource({ token: 'synthetic-csrf', path: protectedReadPath }), { status: 200, protected_value: true });
});
test('missing CSRF is omitted and an actual denial never invents protected content', async t => {
  t.mock.method(globalThis, 'fetch', async (_url, request) => {
    assert.equal(Object.hasOwn(request.headers, 'x-rom-csrf'), false);
    return new Response(JSON.stringify({ error: 'Denied' }), { status: 403 });
  });
  assert.deepEqual(await readProtectedResource({ token: null, path: protectedReadPath }), { status: 403, protected_value: false });
});
test('a non-JSON unauthenticated response retains its actual status', async t => {
  t.mock.method(globalThis, 'fetch', async () => new Response('', { status: 401 }));
  assert.deepEqual(await readProtectedResource({ token: 'synthetic-stale-csrf', path: protectedReadPath }), { status: 401, protected_value: false });
});
