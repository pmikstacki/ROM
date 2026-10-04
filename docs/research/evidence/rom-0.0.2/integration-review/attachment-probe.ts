import assert from 'node:assert/strict';
import { createAttachments } from '../../../../../studio/src/lib/attachments/controller.ts';
import { RemoteError } from '../../../../../studio/src/lib/client/request.ts';
const cap = { version: 1 as const, resource_kind: 'files', stores: ['one'],
  limits: { blob_bytes: 16, chunk_bytes: 8, chunks: 2 }, operations: ['reserve', 'upload', 'download', 'detach'] as const };
const view = { key: { kind: 'files', id: 'file' }, revision: 2n, value: { state: 'ready' } };
for (const [status, category, unknown] of [[500, 'internal', true], [0, 'invalid_response', true],
  [429, 'overloaded', true], [400, 'not_committed', false], [401, 'denied', false]] as const) {
  let calls = 0;
  const submitted: unknown[] = [], uploaded: string[] = [];
  const client = { generation: 0, async reserveBlob(request: unknown) {
    submitted.push(request); if (calls++ === 0) throw new RemoteError(category, status); return view;
  }, async uploadBlob(_id: string, file: Blob) { uploaded.push(await file.text()); return view; },
  async detachBlob() { return view; }, async downloadBlob() { return new Uint8Array(); }, async query() { return [view]; } };
  const controller = createAttachments(client, { ...cap, operations: [...cap.operations] }, () => 'frozen-key', async () => 'a'.repeat(64));
  await controller.upload('file', 'one', new Blob(['frozen-file']));
  assert.equal(controller.state.phase, unknown ? 'unknown' : 'error');
  assert.equal(controller.state.pending !== null, unknown);
  assert.equal(calls, 1);
  if (unknown) {
    await controller.retry(); assert.equal(calls, 2); assert.equal(submitted[0], submitted[1]);
    assert.deepEqual(uploaded, ['frozen-file']); assert.equal(controller.state.phase, 'success');
  }
  controller.dispose(); assert.equal(controller.state.pending, null);
  console.log(JSON.stringify({ status, category, uncertain_retained: unknown, submissions_before_explicit_retry: 1, corrected_invariant: true }));
}
