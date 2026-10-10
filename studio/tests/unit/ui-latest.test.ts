import test from "node:test";
import assert from "node:assert/strict";
import {
  createLatestRequest,
  type LatestRequestState,
} from "../../src/lib/ui/latest-request.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const turn = () => new Promise<void>((resolve) => setImmediate(resolve));
function fixture() {
  const states: LatestRequestState<{ label: string }>[] = [];
  const request = createLatestRequest({
    clone: (value: { label: string }) => ({ ...value }),
    classifyError: () => "Unavailable",
    onState: (state) => states.push(state),
  });
  return { states, request };
}

test("superseding settles the caller even when old work ignores cancellation", async () => {
  const { states, request } = fixture();
  const old = deferred<{ label: string }>();
  let settled = false;
  const first = request
    .run("old", () => old.promise)
    .then(() => {
      settled = true;
    });
  await request.run("new", async () => ({ label: "current" }));
  await turn();
  assert.equal(settled, true);
  old.resolve({ label: "obsolete" });
  await first;
  await turn();
  assert.deepEqual(states.at(-1), {
    phase: "ready",
    identity: "new",
    value: { label: "current" },
    code: null,
  });
});

test("late errors never classify or publish over a newer result", async () => {
  const states: LatestRequestState<string>[] = [];
  let classified = 0;
  const request = createLatestRequest({
    clone: (v: string) => v,
    classifyError: () => {
      classified++;
      return "Unavailable";
    },
    onState: (s) => states.push(s),
  });
  const old = deferred<string>();
  const first = request.run("old", () => old.promise);
  await request.run("new", async () => "new");
  old.reject(new Error("private server text"));
  await first;
  await turn();
  assert.equal(classified, 0);
  assert.equal(states.at(-1)?.value, "new");
});

test("an abort listener may start a newer request without the interrupted run overwriting it", async () => {
  const { states, request } = fixture();
  let inner: Promise<void> | undefined;
  const first = request.run("old", (signal) => {
    signal.addEventListener("abort", () => {
      inner = request.run("listener", async () => ({ label: "listener" }));
    });
    return new Promise(() => {});
  });
  let middleCalled = false;
  await request.run("middle", async () => {
    middleCalled = true;
    return { label: "middle" };
  });
  await inner;
  await first;
  assert.equal(middleCalled, false);
  assert.equal(states.at(-1)?.identity, "listener");
});

test("pending notification can clear work before its task starts", async () => {
  let called = false;
  const states: LatestRequestState<string>[] = [];
  const request = createLatestRequest({
    clone: (v: string) => v,
    classifyError: () => "Unavailable",
    onState: (s) => {
      states.push(s);
      if (s.phase === "pending") request.clear();
    },
  });
  await request.run("one", async () => {
    called = true;
    return "one";
  });
  assert.equal(called, false);
  assert.deepEqual(states.at(-1), {
    phase: "idle",
    identity: null,
    value: null,
    code: null,
  });
});

test("clone and classifier callbacks cannot publish after clearing ownership", async () => {
  const states: LatestRequestState<string>[] = [];
  let clearDuringClone = true;
  const request = createLatestRequest({
    clone: (v: string) => {
      if (clearDuringClone) request.clear();
      return v;
    },
    classifyError: () => {
      request.clear();
      return "Unavailable";
    },
    onState: (s) => states.push(s),
  });
  await request.run("clone", async () => "one");
  assert.equal(states.at(-1)?.phase, "idle");
  clearDuringClone = false;
  await request.run("error", async () => {
    throw new Error("secret");
  });
  assert.equal(states.at(-1)?.phase, "idle");
});

test("ready data is cloned and errors expose only the host code", async () => {
  const { states, request } = fixture();
  const value = { label: "original" };
  await request.run("value", async () => value);
  value.label = "mutated";
  assert.equal(states.at(-1)?.value?.label, "original");
  await request.run("failure", async () => {
    throw new Error("secret credential");
  });
  assert.deepEqual(states.at(-1), {
    phase: "error",
    identity: "failure",
    value: null,
    code: "Unavailable",
  });
});

test("dispose settles pending work and never publishes again", async () => {
  const { states, request } = fixture();
  const old = deferred<{ label: string }>();
  const pending = request.run("old", () => old.promise);
  request.dispose();
  const count = states.length;
  await pending;
  await request.run("after", async () => ({ label: "ignored" }));
  request.clear();
  old.resolve({ label: "old" });
  await turn();
  assert.equal(states.length, count);
});

test("a throwing notification rejects the wrapper without classifying host defects", async () => {
  let called = false;
  let classified = false;
  const problem = new Error("host callback failed");
  const request = createLatestRequest({
    clone: (v: string) => v,
    classifyError: () => {
      classified = true;
      return "Unavailable";
    },
    onState: () => {
      throw problem;
    },
  });
  await assert.rejects(
    request.run("one", async () => {
      called = true;
      return "one";
    }),
    (error) => error === problem,
  );
  assert.equal(called, false);
  assert.equal(classified, false);
});

test("a synchronous task rejection becomes a sanitized current error", async () => {
  const { states, request } = fixture();
  await request.run("one", () => {
    throw new Error("private text");
  });
  assert.equal(states.at(-1)?.phase, "error");
  assert.equal(states.at(-1)?.code, "Unavailable");
});
