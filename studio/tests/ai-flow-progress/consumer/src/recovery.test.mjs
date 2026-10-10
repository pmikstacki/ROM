import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createFlowRecovery, progressText, validateRunView } from './recovery.mjs';
const principal = { authority: 'external-ai-consumer', subject: 'author', kind: 'human' };
function fixture(post) {
  let saved = null, sequence = 0;
  const store = { async read() { return saved; }, async compareExchange(_slot, expected, next) { if ((saved?.version ?? null) !== expected) return false; saved = next; return true; } };
  const options = { store, post, principal, domain: 'publication', run: 'browser-run', slot: 'publication:browser-run', newVersion: () => `version-${++sequence}` };
  return { options, stored: () => saved };
}
const view = revision => ({ run_id: 'browser-run', revision: String(revision), state: 'ToolsPending', failure: null, output: null, cancel_requested: false, milestones: [], counters: { generation_attempts: 1, tool_calls: 0, ticks: 3 }, read_progress: { status: 'AwaitingRecovery', ordinal: 1 } });
test('unknown acknowledgement reload retries the exact original operation identity and revision', async () => {
  const requests = []; let lost = true;
  const f = fixture(async (_route, body) => { requests.push(body); if (lost) { lost = false; throw Error('connection lost after admission'); } return view(12); });
  const first = createFlowRecovery(f.options); await first.restore(); await first.begin('resume', view(9), 'recover-original');
  await assert.rejects(first.retry()); assert.equal(first.state.knowledge, 'unknown');
  const reopened = createFlowRecovery(f.options); await reopened.restore(); await reopened.retry();
  assert.deepEqual(requests, [ { run_id: 'browser-run', expected_revision: '9', idempotency: 'recover-original' }, { run_id: 'browser-run', expected_revision: '9', idempotency: 'recover-original' } ]);
  assert.equal(reopened.state.knowledge, 'confirmed');
  assert.equal(f.stored(), null);
});
test('pending uncertain recovery rejects replacement input and refusal preserves unknown outcome', async () => {
  const f = fixture(async () => { const error = Error('denied'); error.category = 'denied'; throw error; });
  const lane = createFlowRecovery(f.options); await lane.restore(); await lane.begin('resume', view(9), 'original');
  await assert.rejects(lane.retry()); await assert.rejects(lane.begin('cancel', view(10), 'replacement'), /pending/);
  assert.equal(lane.state.knowledge, 'unknown'); assert.equal(lane.state.view, null);
});
test('owner change and late response cannot disclose the earlier projection', async () => {
  let resolveResponse; const f = fixture(() => new Promise(resolve => { resolveResponse = resolve; }));
  const lane = createFlowRecovery(f.options); await lane.restore(); await lane.begin('resume', view(9), 'original');
  const pending = lane.retry(); await new Promise(resolve => setImmediate(resolve)); lane.quarantine(); resolveResponse(view(12));
  await assert.rejects(pending, /binding/); assert.equal(lane.state.view, null); assert.equal(lane.state.phase, 'quarantined');
  assert.ok(f.stored());
});
test('advisory progress labels do not declare activity or successful side effects', () => {
  assert.equal(progressText({ status: 'Queued', ordinal: 1 }), 'Read queued');
  assert.equal(progressText({ status: 'Active', ordinal: 1 }), 'Read active; caller timeout does not end physical work');
  assert.equal(progressText({ status: 'AwaitingRecovery', ordinal: 2 }), 'Read awaiting authorized recovery');
  assert.equal(progressText({ status: 'ActivityUnknown', ordinal: 2 }), 'Read activity unknown');
});
test('wire validation rejects unsafe revisions, secret-bearing progress and malformed statuses', () => {
  assert.equal(validateRunView(view('9007199254740993')).revision, '9007199254740993');
  assert.throws(() => validateRunView({ ...view(1), revision: 9007199254740993 }), /revision/);
  assert.throws(() => validateRunView({ ...view(1), read_progress: { status: 'Active', ordinal: 1, arguments: 'secret' } }), /progress/);
  assert.throws(() => validateRunView({ ...view(1), read_progress: { status: 'Done', ordinal: 1 } }), /progress/);
});
test('two-tab begin collision preserves the first durable intent', async () => {
  const f = fixture(async () => view(10)); const one = createFlowRecovery(f.options), two = createFlowRecovery(f.options);
  await one.restore(); await two.restore(); await one.begin('resume', view(9), 'first');
  await assert.rejects(two.begin('resume', view(9), 'second'), /concurrent/);
  assert.equal(JSON.parse(f.stored().payload).idempotency, 'first');
});
test('public wire safe integer revisions normalize exactly and Waiting keeps its deadline', () => {
  assert.equal(validateRunView({ ...view(3), revision: 3 }).revision, '3');
  const waiting = { Waiting: { retry_at_unix_ms: 1781000000000 } };
  assert.deepEqual(validateRunView({ ...view(3), state: waiting }).state, waiting);
  assert.throws(() => validateRunView({ ...view(3), state: { Waiting: { retry_at_unix_ms: 1, secret: true } } }), /view/);
});
