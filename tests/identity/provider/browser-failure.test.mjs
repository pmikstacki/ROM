import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyBrowserFailure } from './browser-failure.mjs';

test('browser failure evidence is a closed classification without raw messages', () => {
  const secret = 'synthetic-secret-not-for-evidence';
  const result = classifyBrowserFailure(Error(`https://host.invalid/callback?code=${secret}`));
  assert.deepEqual(result, { kind: 'other' });
  assert.equal(JSON.stringify(result).includes(secret), false);
});
test('evaluate failures and lost contexts remain distinct from fetch failures', () => {
  assert.deepEqual(classifyBrowserFailure(Error('page.evaluate: ReferenceError: fetch is not defined')), { kind: 'reference-error' });
  assert.deepEqual(classifyBrowserFailure(Error('page.evaluate: Execution context was destroyed')), { kind: 'context-lost' });
  assert.deepEqual(classifyBrowserFailure(Error('page.evaluate: TypeError: Failed to fetch')), { kind: 'network-failure' });
});
test('unexpected inputs and browser deadlines never fabricate product rejection', () => {
  assert.deepEqual(classifyBrowserFailure(null), { kind: 'other' });
  const error = Error('redacted'); error.name = 'TimeoutError';
  assert.deepEqual(classifyBrowserFailure(error), { kind: 'deadline' });
});
