import test from 'node:test';
import assert from 'node:assert/strict';
import { awaitLoginDispatch } from './login-dispatch.mjs';

test('a form that becomes visible after navigation is awaited before entry', async () => {
  let visible;
  const delayed = new Promise(resolve => { visible = resolve; });
  const observation = awaitLoginDispatch(delayed, new Promise(() => {}));
  let settled = false;
  observation.then(() => { settled = true; });
  await Promise.resolve();
  assert.equal(settled, false);
  visible();
  assert.deepEqual(await observation, { kind: 'form' });
});
test('an existing provider session may dispatch the callback without a form', async () => {
  const response = { status: 303 };
  assert.deepEqual(await awaitLoginDispatch(new Promise(() => {}), Promise.resolve(response)), { kind: 'callback', response });
});
test('both failed observations reject instead of inventing form or callback progress', async () => {
  await assert.rejects(awaitLoginDispatch(Promise.reject(Error('form deadline')), Promise.reject(Error('callback deadline'))), AggregateError);
});
