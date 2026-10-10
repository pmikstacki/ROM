import { test } from "node:test";
import assert from "node:assert/strict";
import { createCompositionCommand } from "../../src/lib/ui/components/selection-card.ts";
test("unknown classification holds one command until explicit reset without automatic retry", async () => {
  const states: string[] = [];
  const gate = createCompositionCommand((value) => states.push(value));
  let calls = 0;
  await gate.run(
    () => {
      calls++;
      return "unknown";
    },
    () => true,
  );
  await gate.run(
    () => {
      calls++;
      return "accepted";
    },
    () => true,
  );
  assert.equal(calls, 1);
  assert.equal(states.at(-1), "unknown");
  gate.reset();
  await gate.run(
    () => {
      calls++;
      return "accepted";
    },
    () => true,
  );
  assert.equal(calls, 2);
  assert.equal(states.at(-1), "idle");
});
test("old scope/rejection/disposal cannot publish a stale command result", async () => {
  const states: string[] = [];
  const gate = createCompositionCommand((value) => states.push(value));
  let reject: (error: Error) => void = () => {};
  const pending = gate.run(
    () => new Promise((_, fail) => (reject = fail)),
    () => true,
  );
  gate.reset();
  reject(Error("private"));
  await pending;
  assert.equal(states.at(-1), "idle");
  const disposed = gate.run(
    () => new Promise((_, fail) => (reject = fail)),
    () => true,
  );
  gate.dispose();
  const before = states.length;
  reject(Error("private"));
  await disposed;
  assert.equal(states.length, before);
});
test("throwing then getter cannot establish an outcome", async () => {
  const states: string[] = [];
  const gate = createCompositionCommand((value) => states.push(value));
  await gate.run(
    () =>
      Object.defineProperty({}, "then", {
        get() {
          throw Error("private");
        },
      }) as Promise<"accepted">,
    () => true,
  );
  assert.deepEqual(states, ["pending", "unknown"]);
});
test("callback acknowledgement failure is unknown until explicit host recovery", async () => {
  const states: string[] = [];
  const gate = createCompositionCommand((value) => states.push(value));
  let accepted = 0;
  await gate.run(
    () => {
      accepted++;
      throw Error("lost acknowledgement after acceptance");
    },
    () => true,
  );
  assert.equal(accepted, 1);
  assert.equal(states.at(-1), "unknown");
  await gate.run(
    () => {
      accepted++;
      return "accepted";
    },
    () => true,
  );
  assert.equal(accepted, 1);
});
test("pending publisher reset or dispose prevents stale callback dispatch", async () => {
  for (const action of ["reset", "dispose"] as const) {
    let calls = 0;
    const gate = createCompositionCommand((status) => {
      if (status === "pending") gate[action]();
    });
    await gate.run(
      () => {
        calls++;
        return "accepted";
      },
      () => true,
    );
    assert.equal(calls, 0);
  }
});
