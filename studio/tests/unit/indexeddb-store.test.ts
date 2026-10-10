import test from "node:test";
import assert from "node:assert/strict";
import { openIndexedDbIntentStore } from "../../src/recovery.ts";

test("IndexedDB configuration requires an explicit bounded database before opening browser storage", async () => {
  const valid = { name: "explicit-host", maxBytes: 1024, maxSlots: 2, timeoutMs: 1000 };
  for (const change of [
    { name: "" }, { name: "x".repeat(4097) }, { maxBytes: 0 },
    { maxBytes: 4 * 1024 * 1024 + 1 }, { maxBytes: NaN },
    { maxSlots: 0 }, { maxSlots: 4097 }, { maxSlots: 1.5 },
    { timeoutMs: 0 }, { timeoutMs: 60001 },
  ]) await assert.rejects(openIndexedDbIntentStore({ ...valid, ...change }), /configuration/);
});

test("missing IndexedDB capability fails closed instead of replacing durable storage with memory", async () => {
  await assert.rejects(openIndexedDbIntentStore({ name: "host", maxBytes: 1024, maxSlots: 1 }), /IndexedDB unavailable/);
});
