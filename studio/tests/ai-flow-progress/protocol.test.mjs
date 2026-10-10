import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requireCase, requireOperation, requireOrigin } from './protocol.mjs';
test('fixture operation rejects browser principals and unsafe or changed revision encodings', () => {
  const valid = { run_id: 'browser-run', expected_revision: '9007199254740993', idempotency: 'same-key' };
  assert.deepEqual(requireOperation(valid), valid);
  for (const changed of [ { ...valid, principal: 'service' }, { ...valid, run_id: 'foreign' }, { ...valid, expected_revision: '09' }, { ...valid, expected_revision: 9007199254740993 }, { ...valid, expected_revision: '18446744073709551616' } ]) assert.throws(() => requireOperation(changed));
});
test('cross-origin request and database traversal fail before native forwarding', () => {
  requireOrigin('http://127.0.0.1:43928', 'http://127.0.0.1:43928');
  assert.throws(() => requireOrigin('http://foreign', 'http://127.0.0.1:43928'));
  assert.deepEqual(requireCase({ domain: 'triage', case: 'webkit-1' }), { domain: 'triage', case: 'webkit-1', mode: 'normal' });
  assert.throws(() => requireCase({ domain: 'publication', case: '../../production' }));
});
