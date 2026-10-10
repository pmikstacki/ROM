import { test } from "node:test";
import assert from "node:assert/strict";
import { createMutationRecovery } from "../../src/recovery.ts";
import { parseWire } from "../../src/lib/client/codec.ts";
import { fileStore, binding, options, identity, ready } from "./mutation-recovery-support.test.ts";
import type { Operation } from "../../src/lib/client/types.ts";

test("lexical decimal, negative zero, exact limits and field-state values survive restore", async () => {
  const store = fileStore(), bodies: string[] = [];
  const operation = parseWire('{"type":"patch","input":{"big":{"op":"set","value":18446744073709551615},"float":{"op":"set","value":1.0},"negative":{"op":"set","value":-0.0},"null":{"op":"set","value":null},"false":{"op":"set","value":false},"zero":{"op":"set","value":0},"empty":{"op":"set","value":""},"removed":{"op":"remove"}}}') as unknown as Operation;
  const first = createMutationRecovery(options(store, binding(async (_url, init) => { bodies.push(String(init?.body)); throw Error("lost"); })));
  await first.stage(operation); await first.begin({ ...identity, expected: 18446744073709551615n, retryEpoch: 18446744073709551615n }); await assert.rejects(first.retry());
  const second = createMutationRecovery(options(store, binding(async (_url, init) => { bodies.push(String(init?.body)); throw Error("lost again"); })));
  await second.restore(); await assert.rejects(second.retry());
  assert.equal(bodies[0], bodies[1]); assert.match(bodies[1], /"value":1\.0/); assert.match(bodies[1], /"value":-0\.0/);
  assert.match(bodies[1], /"removed":\{"op":"remove"\}/); assert.match(bodies[1], /18446744073709551615/);
});

for (const corrupt of [
  (r: any) => { r.format = "future-v2"; },
  (r: any) => { r.namespace = "another-host"; },
  (r: any) => { r.target.id = "another-id"; },
  (r: any) => { r.csrf_token = "must-not-be-accepted"; },
  (r: any) => { r.accepted.wire = r.accepted.wire.replace('"expected":7', '"expected":7.0'); },
  (r: any) => { r.accepted.knowledge = "committed"; },
]) {
  test("corrupt saved record fails closed without dispatch or local replacement", async () => {
    const backing = fileStore(); const original = createMutationRecovery(options(backing, binding(async () => ready())));
    await original.stage({ type: "delete" }); await original.begin(identity);
    const persisted = await backing.read("slot"); assert.ok(persisted);
    const record = JSON.parse(persisted.payload); corrupt(record);
    let calls = 0;
    const store = { async read() { return { ...persisted, payload: JSON.stringify(record) }; }, async compareExchange() { return true; } };
    const lane = createMutationRecovery(options(store, binding(async () => { calls++; return ready(); })));
    await assert.rejects(lane.restore()); assert.equal(lane.state.phase, "storage_error"); assert.equal(calls, 0);
    await assert.rejects(lane.stage({ type: "delete" }));
  });
}

test("duplicate envelope keys and bounded payloads fail before dispatch", async () => {
  for (const payload of ['{"format":"a","format":"b"}', "x".repeat(200)]) {
    const lane = createMutationRecovery({ ...options({ async read() { return { version: "one", payload }; }, async compareExchange() { return true; } }, binding(async () => { throw Error("must not dispatch"); })), maxBytes: 128 });
    await assert.rejects(lane.restore()); assert.equal(lane.state.phase, "storage_error");
  }
});

test("a generation change during the submitting notification prevents transport dispatch", async () => {
  let calls = 0;
  const current = binding(async () => { calls++; return ready(); });
  const lane = createMutationRecovery(options(fileStore(), current));
  await lane.stage({ type: "delete" }); await lane.begin(identity);
  lane.subscribe(s => { if (s.phase === "submitting") current.client.invalidateSession(); });
  await assert.rejects(lane.retry()); assert.equal(calls, 0); assert.equal(lane.state.phase, "unknown");
});

test("invalid discriminants, inherited inputs and excessive nesting cannot enter a lane", async () => {
  let calls = 0;
  const lane = createMutationRecovery(options(fileStore(), binding(async () => { calls++; return ready(); })));
  const nested: Record<string, unknown> = {}; let cursor = nested;
  for (let i = 0; i < 130; i++) { const next = {}; cursor.next = next; cursor = next; }
  for (const operation of [
    { type: "future", input: {} },
    { type: "replace", input: Object.create({ inherited: "hidden" }) },
    { type: "patch", input: { field: { op: "future" } } },
    { type: "replace", input: nested },
  ]) {
    await assert.rejects(async () => lane.stage(operation as Operation));
    assert.equal(lane.state.hasDraft, false);
  }
  assert.equal(calls, 0);
});
