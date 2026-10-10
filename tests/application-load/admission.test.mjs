import test from "node:test";
import assert from "node:assert/strict";
import { Admission } from "./admission.mjs";

test("active and pending limits remain finite with visible rejected scheduling", () => {
  const a = new Admission({ active: 2, pending: 1, scheduled: 4 });
  assert.equal(a.offer({ index: 0, planned: 0 }), true);
  const first = a.take();
  assert.equal(a.offer({ index: 1, planned: 10 }), true);
  const second = a.take();
  assert.equal(a.offer({ index: 2, planned: 20 }), true);
  assert.equal(a.offer({ index: 3, planned: 30 }), false);
  assert.equal(a.take(), null);
  a.finish(first.index);
  assert.deepEqual(a.take(), { index: 2, planned: 20 });
  assert.equal(a.stats().rejected_scheduling, 1);
  assert.equal(a.stats().maximum_active, 2);
  assert.equal(a.stats().maximum_pending, 1);
  a.finish(second.index);
  assert.throws(() => a.finish(second.index));
});

test("operation identities cannot be reused and finite budgets reject invalid configuration", () => {
  const a = new Admission({ active: 1, pending: 1, scheduled: 1 });
  a.offer({ index: 0, planned: 0 });
  assert.throws(() => a.offer({ index: 0, planned: 0 }));
  assert.throws(() => a.offer({ index: 1, planned: 1 }));
  for (const c of [{ active: 33 }, { pending: 65 }, { scheduled: 12001 }]) {
    assert.throws(
      () => new Admission({ active: 1, pending: 1, scheduled: 1, ...c }),
    );
  }
});
