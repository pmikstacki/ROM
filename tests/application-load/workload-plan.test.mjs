import test from "node:test";
import assert from "node:assert/strict";
import { groups } from "./workload-plan.mjs";
test("finite mixed traffic has exact request and durable work budgets", () => {
  const counts = { read: 0, mutation: 0, replay: 0, blob: 0 };
  let requests = 0,
    notifications = 0;
  const plans = [...groups()];
  assert.equal(plans.length, 12000);
  for (const g of plans) {
    counts[g.category]++;
    requests += g.requests.length;
    if (g.category === "mutation" && g.requests[0].body.operation.input.input)
      notifications++;
  }
  assert.deepEqual(counts, {
    read: 9040,
    mutation: 1500,
    replay: 820,
    blob: 640,
  });
  assert.equal(requests, 13280);
  assert.equal(notifications, 500);
  const warm = plans.slice(0, 3000);
  const measured = plans.slice(3000);
  for (const category of Object.keys(counts)) {
    assert.ok(warm.some((g) => g.category === category));
    assert.ok(measured.some((g) => g.category === category));
  }
});
test("replay fingerprints match original mutations even when arrival races them", () => {
  const originals = new Map();
  const replays = [];
  for (const g of groups()) {
    if (g.category === "mutation")
      originals.set(g.requests[0].body.idempotency, g.requests[0].body);
    if (g.category === "replay") replays.push(g.requests[0].body);
  }
  for (const replay of replays)
    assert.deepEqual(replay, originals.get(replay.idempotency));
});
test("query pages and attachment transfers retain explicit bounded wire shapes", () => {
  for (const g of groups()) {
    if (g.category === "read" && g.requests[0].path.endsWith("/query"))
      assert.equal(g.requests[0].body.query.limit, 50);
    if (g.category === "blob") {
      assert.equal(g.requests[0].body.bytes, 65536);
      assert.equal(g.requests[1].binary_bytes, 65536);
      assert.equal(g.requests[2].expected_bytes, 65536);
    }
  }
  assert.throws(() => [...groups({ unbounded: true })], /closed workload/);
});
