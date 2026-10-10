import test from "node:test";
import assert from "node:assert/strict";
import { Measurements } from "./measurements.mjs";

test("scheduled latency includes generator delay and percentiles retain operation classes", () => {
  const m = new Measurements(4);
  m.record({
    operation: "read",
    planned: 0,
    dispatched: 10,
    completed: 15,
    outcome: "success",
  });
  m.record({
    operation: "read",
    planned: 20,
    dispatched: 20,
    completed: 21,
    outcome: "overloaded",
  });
  m.record({
    operation: "commit",
    planned: 30,
    dispatched: 30,
    completed: 130,
    outcome: "unknown",
  });
  const s = m.summary();
  assert.equal(s.read.scheduled.p50, 1);
  assert.equal(s.read.scheduled.p95, 15);
  assert.equal(s.read.dispatched.p95, 5);
  assert.equal(s.commit.scheduled.p99, 100);
  assert.equal(s.read.outcomes.overloaded, 1);
  assert.equal(s.commit.outcomes.unknown, 1);
  assert.equal(s.query.count, 0);
});

test("samples are finite and unknown labels or invalid timing are rejected", () => {
  const m = new Measurements(1);
  const r = {
    operation: "read",
    planned: 0,
    dispatched: 1,
    completed: 2,
    outcome: "success",
  };
  m.record(r);
  assert.throws(() => m.record(r), /sample budget/);
  for (const changed of [
    { operation: "private-resource-name" },
    { outcome: "secret error body" },
    { dispatched: -1 },
    { completed: 0 },
    { completed: Infinity },
    { actor: "secret" },
  ])
    assert.throws(() => new Measurements(1).record({ ...r, ...changed }));
});
