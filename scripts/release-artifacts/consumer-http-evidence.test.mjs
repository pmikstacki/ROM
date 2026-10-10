import test from 'node:test';
import assert from 'node:assert/strict';
import { requireRecoveryHttpReport } from './consumer-http-evidence.mjs';

import { recoveryHttpFixture as report } from './consumer-http-fixture.mjs';

test('HTTP recovery evidence binds adapter, committed and replayed counts, tombstone and exact wire bytes', () => {
  for (const adapter of ['sqlite', 'redb']) assert.equal(requireRecoveryHttpReport(report(adapter), adapter).adapter, adapter);
});

test('HTTP replay cannot substitute identities, duplicate commits, lose integers or erase unknown-outcome limits', () => {
  for (const mutate of [
    r => { r.adapter = 'other'; },
    r => { r.exact_first_and_replay = false; },
    r => { r.process_restart = false; },
    r => { r.counts.replayed.counts[2] = 3; },
    r => { r.counts.committed.receipt.identity = 'different'; },
    r => { r.counts.deleted.row.value = {}; },
    r => { r.counts.delete_replayed.row.revision = 4; },
    r => { r.trace.bodies[2] = '{"expected":2}'; },
    r => { r.trace.bodies[1] = r.trace.bodies[2] = '{"expected":1,"value":9007199254740992}'; },
    r => { r.pending_store_limit = 'durable in all circumstances'; },
  ]) {
    const r = report(); mutate(r);
    assert.throws(() => requireRecoveryHttpReport(r, 'sqlite'), /HTTP recovery evidence/);
  }
});
