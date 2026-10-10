import test from "node:test";
import assert from "node:assert/strict";
import { runSchedule } from "./scheduler.mjs";
import { SchedulerDiagnostics } from "./scheduler-diagnostics.mjs";

const plans = (count, category = "read", steps = 1) =>
  Array.from({ length: count }, (_, index) => ({
    index,
    planned: 0,
    measured: true,
    category,
    requests: Array.from({ length: steps }, () => ({ measurement: "read" })),
  }));

test("due offers preserve the finite burst policy and classify occupied-slot refusals", async () => {
  for (const [active, pending, rejected] of [
    [2, 3, 95],
    [32, 64, 4],
  ]) {
    const calls = [];
    const result = await runSchedule(
      plans(100),
      async (request) => {
        calls.push(request.measurement);
        return "success";
      },
      { active, pending, scheduled: 100 },
      1000,
    );
    assert.equal(calls.length, 100 - rejected);
    assert.equal(result.admission.rejected_scheduling, rejected);
    assert.equal(result.diagnostics.refused_idle_slots, 0);
    assert.equal(result.diagnostics.refused_occupied_slots, rejected);
    assert.equal(result.diagnostics.refused_category.read, rejected);
    assert.equal(result.diagnostics.due_batch_max, 100);
    assert.equal(result.diagnostics.offered_lateness.count, 100);
    assert.equal(result.diagnostics.queue_wait.count, 100 - rejected);
    assert.equal(result.diagnostics.pending_offer_times, 0);
  }
});

test("non-success preserves immediate continuation suppression", async () => {
  for (const outcome of ["overloaded", "denied", "unknown", "failure"]) {
    let calls = 0;
    const result = await runSchedule(
      plans(1, "blob", 3),
      async () => {
        calls++;
        return outcome;
      },
      { active: 1, pending: 1, scheduled: 1 },
      1000,
    );
    assert.equal(calls, 1);
    assert.equal(result.diagnostics.continuations_skipped_non_success, 2);
    assert.equal(result.diagnostics.requests_cancelled_before_execution, 0);
  }
});

test("diagnostics never read payloads or copy free-text categories", async () => {
  const offered = plans(1, "secret-label");
  Object.defineProperty(offered[0].requests[0], "body", {
    get() {
      throw Error("payload read");
    },
  });
  const result = await runSchedule(
    offered,
    async () => "success",
    { active: 1, pending: 1, scheduled: 1 },
    1000,
  );
  assert.ok(!JSON.stringify(result.diagnostics).includes("secret-label"));
  assert.ok(Buffer.byteLength(JSON.stringify(result.diagnostics)) <= 4096);
  assert.equal(result.diagnostics.overflowed, false);
});

test("occupied refusals, offer lateness and queue residence are separate", () => {
  const d = new SchedulerDiagnostics(2);
  d.offer(0, "mutation", 10, 15, true, 0);
  d.offer(1, "blob", 10, 17, false, 2);
  d.offer(2, "mutation", 10, 18, false, 1);
  d.dispatch(0, 22);
  d.dueBatch(3);
  const x = d.snapshot();
  assert.equal(x.refused_occupied_slots, 1);
  assert.equal(x.refused_idle_slots, 1);
  assert.equal(x.refused_category.blob, 1);
  assert.equal(x.refused_category.mutation, 1);
  assert.equal(x.offered_lateness.sum_ms, 20);
  assert.equal(x.queue_wait.sum_ms, 7);
  assert.equal(x.queue_wait.count, 1);
  assert.equal(x.pending_offer_times, 0);
  assert.equal(x.overflowed, false);
  assert.equal(
    x.offered_lateness.buckets.reduce((a, b) => a + b),
    3,
  );
});

test("invalid accounting metadata latches overflow without valid samples", () => {
  const valid = [0, "read", 0, 1, false, 0];
  for (const [position, value] of [
    [0, NaN],
    [0, -1],
    [0, 12000],
    [2, NaN],
    [2, -1],
    [3, Infinity],
    [3, 1200001],
    [4, 1],
    [5, NaN],
    [5, -1],
    [5, 33],
  ]) {
    const d = new SchedulerDiagnostics(32),
      args = [...valid];
    args[position] = value;
    d.offer(...args);
    assert.equal(d.snapshot().overflowed, true);
    assert.equal(d.snapshot().offered_lateness.count, 0);
    assert.equal(
      d.snapshot().refused_occupied_slots + d.snapshot().refused_idle_slots,
      0,
    );
  }
  const early = new SchedulerDiagnostics(1);
  early.offer(0, "read", 2, 1, true, 0);
  assert.equal(early.snapshot().overflowed, true);
  assert.equal(early.snapshot().pending_offer_times, 0);
  for (const limit of [NaN, 0, 33, 1.5])
    assert.equal(new SchedulerDiagnostics(limit).snapshot().overflowed, true);
  for (const dispatched of [NaN, Infinity, -1, 4, 1200001]) {
    const d = new SchedulerDiagnostics(1);
    d.offer(0, "read", 0, 5, true, 0);
    d.dispatch(0, dispatched);
    assert.equal(d.snapshot().overflowed, true);
    assert.equal(d.snapshot().queue_wait.count, 0);
    assert.equal(d.snapshot().pending_offer_times, 1);
  }
});

test("timestamp storage, counters and snapshots have fixed bounds", () => {
  const d = new SchedulerDiagnostics(32);
  for (let i = 0; i < 65; i++) d.offer(i, "arbitrary-" + i, 0, 0, true, 0);
  const x = d.snapshot();
  assert.equal(x.pending_offer_times, 64);
  assert.equal(x.overflowed, true);
  assert.deepEqual(Object.keys(x.refused_category), [
    "read",
    "mutation",
    "replay",
    "blob",
    "other",
  ]);
  assert.ok(Buffer.byteLength(JSON.stringify(x)) <= 4096);
  d.nonSuccess(14000);
  assert.equal(d.snapshot().continuations_skipped_non_success, 13280);
  x.queue_wait.buckets[0] = 999;
  assert.equal(d.snapshot().queue_wait.buckets[0], 0);
  d.cancelled(NaN);
  assert.equal(d.snapshot().requests_cancelled_before_execution, 0);
});

test("deadline cancellation retains queued timestamps and suppresses remaining steps", async () => {
  let calls = 0;
  const result = await runSchedule(
    plans(3, "read", 3),
    async (_request, signal) => {
      calls++;
      if (!signal.aborted)
        await new Promise((resolve) =>
          signal.addEventListener("abort", resolve, { once: true }),
        );
      return "success";
    },
    { active: 1, pending: 3, scheduled: 3 },
    5,
  );
  assert.equal(calls, 1);
  assert.equal(result.deadline_reached, true);
  assert.equal(result.admission.active, 0);
  assert.equal(result.admission.pending, 2);
  assert.equal(result.diagnostics.pending_offer_times, 2);
  assert.equal(result.diagnostics.requests_cancelled_before_execution, 2);
  assert.equal(result.diagnostics.queue_wait.count, 1);
  assert.equal(result.diagnostics.overflowed, false);
});
