import test from "node:test";
import assert from "node:assert/strict";
import { createObservation } from "../../src/observe.ts";

test("public observation entry supports explicitly public host data", async () => {
  let release!: () => void;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  const observation = createObservation({
    scope: { kind: "public", key: "public-history" },
    source: async function* () {
      yield ["one"];
      await held;
    },
    clone: (row: string) => row,
    measure: (rows) => new TextEncoder().encode(rows.join(",")).byteLength,
    limits: { maxRows: 10, maxBytes: 100 },
    retry: {
      maxAttempts: 1,
      maxElapsedMs: 1000,
      delayMs: 0,
      clock: () => performance.now(),
    },
    classifyError: () => ({ kind: "fatal", code: "Unavailable" }),
    onAuthorityLost: () => {},
  });
  const ready = new Promise<void>((resolve) => {
    observation.subscribe((state) => {
      if (state.phase === "fresh") resolve();
    });
  });
  observation.start();
  await ready;
  assert.deepEqual(observation.state.rows, ["one"]);
  observation.dispose();
  release();
});
