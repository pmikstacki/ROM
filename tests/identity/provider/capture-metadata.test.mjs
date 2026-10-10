import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requireCaptureAttempt, requestFieldNames } from './capture-metadata.mjs';

test('capture attempt names cannot escape private evidence paths', () => {
  assert.equal(requireCaptureAttempt('attempt13'), 'attempt13');
  for (const value of ['../token', 'attempt0', 'attempt33', 'attempt1/result', '']) {
    assert.throws(() => requireCaptureAttempt(value));
  }
});

test('request projections retain known field names without retaining credentials', () => {
  assert.deepEqual(requestFieldNames('application/json', '{"password":"private-value","username":"synthetic","unexpected-private-value":"secret"}'), ['password', 'username']);
  assert.deepEqual(requestFieldNames('multipart/form-data; boundary=private-value', 'private-value'), []);
  assert.deepEqual(requestFieldNames('application/json', 'broken-private-value'), []);
});
