import test from "node:test";
import assert from "node:assert/strict";
import { runSchedule } from "./scheduler.mjs";

function groups(count, steps = 1, spacing = 0) {
  return Array.from({ length: count }, (_, index) =>
    Object.freeze({
      index,
      planned: index * spacing,
      measured: true,
      category: "read",
      requests: Object.freeze(
        Array.from({ length: steps }, (_, step) =>
          Object.freeze({ measurement: "read", index, step }),
        ),
      ),
    }),
  );
}
async function heldBurst(outcome, steps = 1) {
  const calls = [],
    releases = [];
  let released = false,
    ready;
  const started = new Promise((resolve) => {
    ready = resolve;
  });
  const offered = groups(100, steps);
  const running = runSchedule(
    offered,
    async (request, signal) => {
      calls.push([request.index, request.step]);
      if (!released)
        await new Promise((resolve) => {
          releases.push(resolve);
          if (signal.aborted) resolve();
          else signal.addEventListener("abort", resolve, { once: true });
          if (calls.length === 32) ready();
        });
      return outcome;
    },
    { active: 32, pending: 64, scheduled: 100 },
    1000,
  );
  let initial;
  try {
    await started;
    initial = calls.slice();
  } finally {
    released = true;
    for (const resolve of releases) resolve();
  }
  const result = await running;
  return { calls, initial, result, offered };
}

test("held100 due groups use32 active and64 pending before refusing4", async () => {
  const { calls, initial, result, offered } = await heldBurst("success");
  assert.deepEqual(
    initial,
    Array.from({ length: 32 }, (_, index) => [index, 0]),
  );
  assert.deepEqual(
    calls,
    Array.from({ length: 96 }, (_, index) => [index, 0]),
  );
  assert.equal(result.admission.scheduled, 100);
  assert.equal(result.admission.rejected_scheduling, 4);
  assert.equal(result.admission.maximum_active, 32);
  assert.equal(result.admission.maximum_pending, 64);
  assert.equal(result.admission.active, 0);
  assert.equal(result.admission.pending, 0);
  assert.equal(result.diagnostics.refused_idle_slots, 0);
  assert.equal(result.diagnostics.refused_occupied_slots, 4);
  assert.equal(result.diagnostics.pending_offer_times, 0);
  assert.ok(offered.every((group) => group.planned === 0));
});

test("FIFO dispatch preserves immediate non-success continuation break", async () => {
  const { calls, result } = await heldBurst("denied", 3);
  assert.deepEqual(
    calls,
    Array.from({ length: 96 }, (_, index) => [index, 0]),
  );
  assert.equal(result.admission.rejected_scheduling, 4);
  assert.equal(result.measurements.read.outcomes.denied, 96);
  assert.equal(result.diagnostics.continuations_skipped_non_success, 192);
  assert.equal(result.diagnostics.requests_cancelled_before_execution, 0);
});

test("deadline cancellation starts no queued groups and retains pending accounting", async () => {
  const calls = [];
  const result = await runSchedule(
    groups(3, 3),
    async (request, signal) => {
      calls.push([request.index, request.step]);
      if (!signal.aborted)
        await new Promise((resolve) =>
          signal.addEventListener("abort", resolve, { once: true }),
        );
      return "success";
    },
    { active: 1, pending: 3, scheduled: 3 },
    5,
  );
  assert.deepEqual(calls, [[0, 0]]);
  assert.equal(result.deadline_reached, true);
  assert.equal(result.admission.active, 0);
  assert.equal(result.admission.pending, 2);
  assert.equal(result.diagnostics.pending_offer_times, 2);
  assert.equal(result.diagnostics.requests_cancelled_before_execution, 2);
});

test("literal10ms planned timestamps remain immutable and FIFO", async () => {
  const offered = groups(4, 1, 10),
    calls = [];
  const result = await runSchedule(
    offered,
    async (request) => {
      calls.push(request.index);
      return "success";
    },
    { active: 32, pending: 64, scheduled: 4 },
    1000,
  );
  assert.deepEqual(
    offered.map((group) => group.planned),
    [0, 10, 20, 30],
  );
  assert.deepEqual(calls, [0, 1, 2, 3]);
  assert.equal(result.admission.rejected_scheduling, 0);
  assert.equal(result.measurements.read.count, 4);
  assert.ok(result.measurements.read.generator_delay.p50 >= 0);
});
