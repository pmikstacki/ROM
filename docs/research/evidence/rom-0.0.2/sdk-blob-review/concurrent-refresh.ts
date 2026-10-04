import assert from 'node:assert/strict';
import { createClient } from '../../../../../studio/src/lib/client/client.ts';
const cap = { version: 1, resource_kind: 'blobs', stores: ['folder'], limits: { blob_bytes: 1024, chunk_bytes: 16, chunks: 64 }, operations: ['reserve', 'upload', 'download', 'detach'] };
let resolveFirst: (response: Response) => void = () => {}, calls = 0;
const client = createClient({ base: '/api', fetch: async () => {
  if (++calls === 1) return new Promise<Response>(resolve => { resolveFirst = resolve; });
  return new Response(JSON.stringify({ error: 'denied' }), { status: 403, headers: { 'content-type': 'application/json' } });
} });
const first = client.blobCapabilities();
await assert.rejects(client.blobCapabilities(), /denied/);
resolveFirst(new Response(JSON.stringify(cap), { headers: { 'content-type': 'application/json' } }));
await first;
await assert.rejects(client.reserveBlob({ id: 'a', store: 'folder', digest: 'a'.repeat(64), bytes: 1n, idempotency: 'key' }), /denied/);
console.log(JSON.stringify({ probe: 'older-capability-after-failed-refresh', calls, reserve_reached_transport: calls === 3, generation: client.generation }));
