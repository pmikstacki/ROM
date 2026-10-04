import assert from 'node:assert/strict';
import { createClient } from '../../../../../studio/src/lib/client/client.ts';

const cap = { version: 1, resource_kind: 'blobs', stores: ['folder'], limits: { blob_bytes: 1024, chunk_bytes: 16, chunks: 64 }, operations: ['reserve', 'upload', 'download', 'detach'] };
const json = (value: unknown) => new Response(JSON.stringify(value), { headers: { 'content-type': 'application/json' } });

for (const id of ['.', '..']) {
  const paths: string[] = [];
  const client = createClient({ base: '/rom-studio/api', fetch: async url => {
    const path = new URL(String(url), 'https://studio.example').pathname; paths.push(path);
    return path.endsWith('capabilities') ? json(cap) : new Response(new Uint8Array([1]), { headers: { 'content-type': 'application/octet-stream' } });
  } });
  await client.blobCapabilities();
  assert.equal((await client.downloadBlob(id)).length, 1);
  console.log(JSON.stringify({ probe: 'download-path', id, actual: paths[1], attachment_endpoint: paths[1].startsWith('/rom-studio/blobs/attachment/') }));
}

for (const kind of ['length', 'content-type']) {
  let cancelled = 0, signal: AbortSignal | undefined;
  const client = createClient({ base: '/api', fetch: async (url, init) => {
    if (String(url).endsWith('capabilities')) return json(cap);
    signal = init?.signal ?? undefined;
    return new Response(new ReadableStream<Uint8Array>({ cancel() { cancelled++; } }), {
      headers: { 'content-type': kind === 'length' ? 'application/octet-stream' : 'text/html', ...(kind === 'length' ? { 'content-length': '1025' } : {}) },
    });
  } });
  await client.blobCapabilities();
  await assert.rejects(client.downloadBlob('a'), kind === 'length' ? /limit/ : /content type/);
  await Promise.resolve();
  console.log(JSON.stringify({ probe: 'early-binary-rejection', kind, cancelled, aborted: signal?.aborted }));
}

const windows: number[] = [];
for (let steps = 0; steps < 40; steps++) {
  let calls = 0;
  const client = createClient({ base: '/api', fetch: async () => { calls++; return json(cap); } });
  const pending = client.blobCapabilities().then(() => 'accepted', () => 'rejected');
  let tick = Promise.resolve();
  for (let step = 0; step < steps; step++) tick = tick.then(() => {});
  const invalidation = tick.then(() => client.invalidateSession());
  const admitted = await pending;
  await invalidation;
  await client.reserveBlob({ id: 'a', store: 'folder', digest: 'a'.repeat(64), bytes: 1n, idempotency: 'key' }).catch(() => {});
  if (admitted === 'accepted' && calls > 1) windows.push(steps);
}
console.log(JSON.stringify({ probe: 'capability-generation-window', steps: windows }));
