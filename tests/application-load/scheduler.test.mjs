import test from "node:test";
import assert from "node:assert/strict";
import { runSchedule } from "./scheduler.mjs";
function* plans(count) {
  for (let index = 0; index < count; index++)
    yield {
      index,
      planned: 0,
      measured: true,
      requests: [{ measurement: "read" }],
    };
}
test("bounded scheduled traffic separates client refusal from server errors", async () => {
  let active = 0,
    maximum = 0,
    calls = 0;
  const result = await runSchedule(
    plans(100),
    async () => {
      active++;
      maximum = Math.max(maximum, active);
      calls++;
      await new Promise((resolve) => setTimeout(resolve, 2));
      active--;
      return "success";
    },
    { active: 2, pending: 3, scheduled: 100 },
    1000,
  );
  assert.equal(maximum, 2);
  assert.equal(calls, 5);
  assert.equal(result.admission.rejected_scheduling, 95);
  assert.equal(result.measurements.read.count, 5);
  assert.equal(result.admission.active, 0);
});
test("execution failure is retained as a sample and does not leak payloads", async () => {
  const result = await runSchedule(
    plans(1),
    async () => {
      throw Error("secret value");
    },
    { active: 1, pending: 1, scheduled: 1 },
    1000,
  );
  assert.equal(result.measurements.read.outcomes.failure, 1);
  assert.ok(!JSON.stringify(result).includes("secret"));
});
test("deadline abort is propagated to admitted executor and drains it", async () => {
  const result = await runSchedule(
    plans(1),
    async (_, signal) =>
      new Promise((resolve) =>
        signal.addEventListener("abort", () => resolve("timeout"), {
          once: true,
        }),
      ),
    { active: 1, pending: 1, scheduled: 1 },
    10,
  );
  assert.equal(result.measurements.read.outcomes.timeout, 1);
  assert.equal(result.deadline_reached, true);
  assert.equal(result.admission.active, 0);
});
