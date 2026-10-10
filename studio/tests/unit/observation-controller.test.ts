import test from "node:test";
import assert from "node:assert/strict";
import { createObservation } from "../../src/lib/observe/controller.ts";
import type { ObservationSource } from "../../src/lib/observe/types.ts";
const publicScope = { kind: "public" as const, key: "catalog" };
const principalScope = (subject: string, generation = 1) => ({
  kind: "principal" as const,
  principal: { authority: "local", kind: "human" as const, subject },
  generation,
});
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}
async function waitFor(check: () => boolean) {
  for (let i = 0; i < 100; i++) {
    if (check()) return;
    await new Promise((resolve) => setTimeout(resolve, 2));
  }
  assert.fail("observation did not reach expected state");
}
function fixture(
  source: ObservationSource<{ label: string }>,
  scope = publicScope,
) {
  let lost = 0;
  const observation = createObservation({
    scope,
    source,
    clone: (row: { label: string }) => ({ ...row }),
    measure: (rows) =>
      new TextEncoder().encode(JSON.stringify(rows)).byteLength,
    limits: { maxRows: 2, maxBytes: 256 },
    retry: {
      maxAttempts: 2,
      maxElapsedMs: 1000,
      delayMs: 0,
      clock: () => performance.now(),
    },
    classifyError: (error) => ({
      kind: error === "denied" ? "denied" : "transient",
      code: error === "denied" ? "Denied" : "Unavailable",
    }),
    onAuthorityLost: () => {
      lost++;
    },
  });
  return {
    observation,
    get lost() {
      return lost;
    },
  };
}

test("rows are detached and transient failure has a finite attempt ceiling", async () => {
  let attempts = 0;
  const row = { label: "original" };
  const f = fixture(async function* () {
    attempts++;
    yield [row];
    throw new Error("private response");
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "exhausted");
  row.label = "mutated";
  assert.equal(attempts, 2);
  assert.equal(f.observation.state.rows[0].label, "original");
  const snapshot = f.observation.state;
  snapshot.rows[0].label = "client mutation";
  assert.equal(f.observation.state.rows[0].label, "original");
  assert.equal(f.observation.state.stale, true);
  f.observation.dispose();
});

test("denial removes private data before the authority-loss callback", async () => {
  const gate = deferred();
  const f = fixture(async function* () {
    yield [{ label: "private" }];
    await gate.promise;
    throw "denied";
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "fresh");
  gate.resolve();
  await waitFor(() => f.observation.state.phase === "denied");
  assert.deepEqual(f.observation.state.rows, []);
  assert.equal(f.lost, 1);
  f.observation.dispose();
});

test("rebind fences late rows from an ignored cancellation signal", async () => {
  const old = deferred();
  const next = deferred();
  const f = fixture(async function* () {
    yield [{ label: "old" }];
    await old.promise;
    yield [{ label: "late secret" }];
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "fresh");
  f.observation.rebind(principalScope("bob"), async function* () {
    yield [{ label: "new" }];
    await next.promise;
  });
  assert.deepEqual(f.observation.state.rows, []);
  await waitFor(() => f.observation.state.rows[0]?.label === "new");
  old.resolve();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(f.observation.state.rows[0]?.label, "new");
  f.observation.dispose();
  next.resolve();
});

test("hidden observation aborts current source and reconnects only when visible", async () => {
  let calls = 0;
  let aborted = false;
  const f = fixture(async function* (signal) {
    calls++;
    signal.addEventListener("abort", () => {
      aborted = true;
    });
    yield [{ label: "row" }];
    await new Promise<void>((resolve) =>
      signal.addEventListener("abort", () => resolve(), { once: true }),
    );
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "fresh");
  f.observation.setVisible(false);
  assert.equal(aborted, true);
  assert.equal(f.observation.state.stale, true);
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(calls, 1);
  f.observation.setVisible(true);
  await waitFor(() => calls === 2 && f.observation.state.phase === "fresh");
  f.observation.dispose();
});

test("row and byte violations do not publish partial data or retry", async () => {
  for (const rows of [
    [{ label: "a" }, { label: "b" }, { label: "c" }],
    [{ label: "x".repeat(300) }],
  ]) {
    let calls = 0;
    const f = fixture(async function* () {
      calls++;
      yield rows;
    });
    f.observation.start();
    await waitFor(() => f.observation.state.phase === "exhausted");
    assert.deepEqual(f.observation.state.rows, []);
    assert.equal(calls, 1);
    assert.equal(f.observation.state.code, "ObservationLimit");
    f.observation.dispose();
  }
});

test("abort listeners can rebind without an older rebind overwriting the new owner", async () => {
  const gate = deferred();
  const f = fixture(async function* (signal) {
    signal.addEventListener("abort", () =>
      f.observation.rebind(principalScope("carol"), async function* () {
        yield [{ label: "carol" }];
        await gate.promise;
      }),
    );
    yield [{ label: "old" }];
    await gate.promise;
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "fresh");
  f.observation.rebind(principalScope("bob"), async function* () {
    yield [{ label: "bob" }];
    await gate.promise;
  });
  await waitFor(() => f.observation.state.phase === "fresh");
  assert.equal(f.observation.state.rows[0].label, "carol");
  f.observation.dispose();
  gate.resolve();
});

test("a throwing error classifier fails closed instead of leaking an unhandled task rejection", async () => {
  const observation = createObservation({
    scope: publicScope,
    source: async function* () {
      throw new Error("private upstream");
    },
    clone: (v: string) => v,
    measure: () => 0,
    limits: { maxRows: 1, maxBytes: 10 },
    retry: {
      maxAttempts: 1,
      maxElapsedMs: 1000,
      delayMs: 0,
      clock: () => performance.now(),
    },
    classifyError: () => {
      throw new Error("host defect");
    },
    onAuthorityLost: () => {},
  });
  observation.start();
  await waitFor(() => observation.state.phase === "exhausted");
  assert.equal(observation.state.code, "ObservationConfiguration");
  assert.deepEqual(observation.state.rows, []);
  observation.dispose();
});

test("a retry timer clock defect terminates without an uncaught callback", async () => {
  let clocks = 0;
  const observation = createObservation({
    scope: publicScope,
    source: async function* () {
      throw "offline";
    },
    clone: (v: string) => v,
    measure: () => 0,
    limits: { maxRows: 1, maxBytes: 10 },
    retry: {
      maxAttempts: 2,
      maxElapsedMs: 1000,
      delayMs: 0,
      clock: () => {
        if (++clocks >= 3) throw new Error("clock defect");
        return 0;
      },
    },
    classifyError: () => ({ kind: "transient", code: "Unavailable" }),
    onAuthorityLost: () => {},
  });
  observation.start();
  await waitFor(() => observation.state.phase === "exhausted");
  assert.equal(observation.state.code, "ObservationConfiguration");
  observation.dispose();
});

test("throwing denied subscriber cannot suppress authority loss or healthy subscribers", async () => {
  const f = fixture(async function* () {
    throw "denied";
  });
  let notifications = 0;
  f.observation.subscribe((state) => {
    if (state.phase === "denied") throw new Error("broken view");
  });
  f.observation.subscribe((state) => {
    if (state.phase === "denied") notifications++;
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "denied");
  assert.equal(f.lost, 1);
  assert.equal(notifications, 1);
  assert.deepEqual(f.observation.state.rows, []);
  f.observation.dispose();
});

test("throwing connecting subscriber does not strand the source", async () => {
  const gate = deferred();
  let calls = 0;
  const f = fixture(async function* () {
    calls++;
    yield [{ label: "current" }];
    await gate.promise;
  });
  f.observation.subscribe((state) => {
    if (state.phase === "connecting") throw new Error("broken view");
  });
  try {
    assert.doesNotThrow(() => f.observation.start());
    await waitFor(() => f.observation.state.phase === "fresh");
    assert.equal(calls, 1);
  } finally {
    f.observation.dispose();
    gate.resolve();
  }
});

test("nested idle rebind connects the surviving source once", async () => {
  const gate = deferred();
  let calls = 0;
  let aborts = 0;
  const f = fixture(async function* () {
    yield [{ label: "old" }];
    await gate.promise;
  });
  f.observation.start();
  await waitFor(() => f.observation.state.phase === "fresh");
  f.observation.subscribe((state) => {
    if (
      state.phase === "idle" &&
      state.scope.kind === "public" &&
      state.scope.key === "outer"
    ) {
      f.observation.rebind(
        { kind: "public", key: "inner" },
        async function* (signal) {
          calls++;
          signal.addEventListener("abort", () => {
            aborts++;
          });
          yield [{ label: "inner" }];
          await gate.promise;
        },
      );
    }
  });
  try {
    f.observation.rebind({ kind: "public", key: "outer" }, async function* () {
      assert.fail("superseded source must not start");
      yield [];
    });
    await waitFor(() => f.observation.state.phase === "fresh");
    assert.equal(calls, 1);
    assert.equal(aborts, 0);
    assert.equal(f.observation.state.rows[0].label, "inner");
  } finally {
    f.observation.dispose();
    gate.resolve();
  }
});

test("idle notification hide and disposal prevent the superseded connection", async () => {
  for (const action of ["hide", "dispose"] as const) {
    const gate = deferred();
    let calls = 0;
    const f = fixture(async function* () {
      yield [{ label: "old" }];
      await gate.promise;
    });
    f.observation.start();
    await waitFor(() => f.observation.state.phase === "fresh");
    f.observation.subscribe((state) => {
      if (
        state.phase === "idle" &&
        state.scope.kind === "public" &&
        state.scope.key === "next"
      ) {
        if (action === "hide") f.observation.setVisible(false);
        else f.observation.dispose();
      }
    });
    f.observation.rebind({ kind: "public", key: "next" }, async function* () {
      calls++;
      yield [{ label: "next" }];
      await gate.promise;
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    assert.equal(calls, 0);
    assert.deepEqual(f.observation.state.rows, []);
    f.observation.dispose();
    gate.resolve();
  }
});

test("callback defects have sanitized diagnostics and cannot break denial", async () => {
  const diagnostics: string[] = [];
  let brokenCalls = 0;
  const observation = createObservation({
    scope: publicScope,
    source: async function* () {
      throw "denied";
    },
    clone: (v: string) => v,
    measure: () => 0,
    limits: { maxRows: 1, maxBytes: 10 },
    retry: {
      maxAttempts: 1,
      maxElapsedMs: 1000,
      delayMs: 0,
      clock: () => performance.now(),
    },
    classifyError: () => ({ kind: "denied", code: "Denied" }),
    onAuthorityLost: () => {
      throw new Error("private callback content");
    },
    onCallbackError: (code) => {
      diagnostics.push(code);
      throw new Error("diagnostic defect");
    },
  });
  observation.subscribe(() => {
    brokenCalls++;
    throw new Error("view defect");
  });
  observation.start();
  await waitFor(() => observation.state.phase === "denied");
  assert.equal(brokenCalls, 1);
  assert.deepEqual(diagnostics, [
    "ObservationSubscriber",
    "ObservationAuthorityCallback",
  ]);
  assert.deepEqual(observation.state.rows, []);
  observation.dispose();
});

test("async callback rejections cannot escape lifecycle supervision", async () => {
  const diagnostics: string[] = [];
  const gate = deferred();
  let subscriberCalls = 0;
  const observation = createObservation({
    scope: publicScope,
    source: async function* () {
      yield ["row"];
      await gate.promise;
      throw "denied";
    },
    clone: (value: string) => value,
    measure: () => 1,
    limits: { maxRows: 1, maxBytes: 10 },
    retry: {
      maxAttempts: 1,
      maxElapsedMs: 1000,
      delayMs: 0,
      clock: () => performance.now(),
    },
    classifyError: () => ({ kind: "denied", code: "Denied" }),
    onAuthorityLost: async () => {
      throw new Error("private authority callback");
    },
    onCallbackError: async (code) => {
      diagnostics.push(code);
      throw new Error("private diagnostic callback");
    },
  });
  observation.subscribe(async () => {
    subscriberCalls++;
    throw new Error("private subscriber");
  });
  // Removal occurs when the returned Promise rejects, not before it settles.
  await waitFor(() => diagnostics.includes("ObservationSubscriber"));
  observation.start();
  await waitFor(
    () => observation.state.phase === "fresh" && diagnostics.length > 0,
  );
  gate.resolve();
  await waitFor(() => observation.state.phase === "denied");
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(subscriberCalls, 1);
  assert.deepEqual(diagnostics, [
    "ObservationSubscriber",
    "ObservationAuthorityCallback",
  ]);
  observation.dispose();
});
