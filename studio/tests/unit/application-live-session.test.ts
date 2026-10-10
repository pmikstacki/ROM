import test from "node:test";
import assert from "node:assert/strict";
import { createAppSession } from "../../src/lib/application/app-session.ts";
import type { PendingIntentStore, StoredIntent } from "../../src/recovery.ts";

function memoryStore(): PendingIntentStore {
  const values = new Map<string, StoredIntent>();
  return {
    async read(key) {
      return structuredClone(values.get(key) ?? null);
    },
    async compareExchange(key, expected, value) {
      if ((values.get(key)?.version ?? null) !== expected) return false;
      if (value) values.set(key, structuredClone(value));
      else values.delete(key);
      return true;
    },
  };
}
async function until(predicate: () => boolean) {
  for (let attempt = 0; attempt < 200 && !predicate(); attempt++)
    await new Promise<void>((resolve) => setTimeout(resolve, 5));
  assert.ok(
    predicate(),
    "expected lifecycle transition within the finite test budget",
  );
}
function fixture() {
  const row = {
    key: { kind: "task", id: "one" },
    revision: 1,
    value: { done: false },
  };
  const streams: ReadableStreamDefaultController<Uint8Array>[] = [];
  let failRenewed = false,
    user = "alice";
  let checks = 0,
    denied = false,
    resume: (() => void) | undefined;
  const encoder = new TextEncoder();
  let abortHook = () => {};
  let version = 0;
  const session = createAppSession({
    base: "http://fixture/",
    profile: {
      authority: "fixture",
      now: () => 1,
      recovery: {
        namespace: "live-session",
        maxBytes: 65536,
        intentStore: memoryStore(),
        editorStore: memoryStore(),
        slot: (owner, target, record) =>
          `${owner.subject}/${target.kind}/${target.id}/${record}`,
        newVersion: () => String(++version),
        newCommandKey: () => "command",
        retryEpoch: () => 1n,
      },
    },
    fetch: async (url, init) => {
      const route = String(url).split("/").at(-1);
      if (route === "session") {
        if (++checks === 2)
          await new Promise<void>((resolve) => {
            resume = resolve;
          });
        return Response.json(
          denied
            ? { authenticated: false }
            : {
                authenticated: true,
                generation: "same",
                csrf_token: "csrf",
                user_id: user,
                expires_at: 100,
              },
        );
      }
      if (route === "live")
        return new Response(
          new ReadableStream<Uint8Array>({
            start(controller) {
              streams.push(controller);
              if (failRenewed && streams.length > 1) {
                controller.enqueue(
                  encoder.encode("event: error\ndata: identity_expired\n\n"),
                );
                controller.close();
                return;
              }
              controller.enqueue(
                encoder.encode(
                  `event: data\ndata: ${JSON.stringify([row])}\n\n`,
                ),
              );
              init?.signal?.addEventListener(
                "abort",
                () => {
                  abortHook();
                  try {
                    controller.close();
                  } catch {}
                },
                { once: true },
              );
            },
          }),
          { headers: { "content-type": "text/event-stream" } },
        );
      return Response.json(
        route === "discover"
          ? {
              version: 1,
              resources: [
                {
                  kind: "task",
                  version: 1,
                  fields: [],
                  actions: [],
                  action_inputs: [],
                },
              ],
            }
          : route === "query"
            ? [row]
            : row,
      );
    },
  });
  return {
    session,
    streams,
    onAbort(hook: () => void) {
      abortHook = hook;
    },
    get checks() {
      return checks;
    },
    release(allow = true, nextUser = "alice") {
      denied = !allow;
      user = nextUser;
      resume?.();
    },
    failRenewedStream() {
      failRenewed = true;
    },
    expire() {
      streams[0].enqueue(
        encoder.encode("event: error\ndata: identity_expired\n\n"),
      );
      streams[0].close();
    },
  };
}

test("managed live query resumes after same-owner refresh and fencing", async () => {
  const f = fixture();
  try {
    await f.session.refresh();
    void f.session.controller.observe();
    await until(
      () =>
        f.streams.length === 1 && f.session.controller.state.rows.length === 1,
    );
    f.expire();
    await until(() => f.checks === 2);
    f.release();
    await until(
      () => f.streams.length === 2 && f.session.controller.state.live,
    );
    assert.equal(f.session.controller.state.rows[0]?.key.id, "one");
    assert.equal(f.session.controller.state.session?.status, "active");
  } finally {
    f.release();
    f.session.destroy();
  }
});

for (const cancellation of [
  "stop",
  "navigation",
  "denied",
  "other-owner",
  "destroy",
] as const)
  test(`managed live renewal does not resume after ${cancellation}`, async () => {
    const f = fixture();
    try {
      await f.session.refresh();
      void f.session.controller.observe();
      await until(() => f.streams.length === 1);
      f.expire();
      await until(() => f.checks === 2);
      if (cancellation === "stop") f.session.controller.stopLive();
      if (cancellation === "navigation")
        await f.session.controller.selectKind("task", { limit: 1 });
      if (cancellation === "destroy") f.session.destroy();
      f.release(
        cancellation !== "denied",
        cancellation === "other-owner" ? "bob" : "alice",
      );
      await until(
        () =>
          cancellation === "destroy" ||
          f.session.controller.state.session?.status !== "renewing",
      );
      await new Promise<void>((resolve) => setTimeout(resolve, 20));
      assert.equal(f.streams.length, 1);
      assert.equal(f.session.controller.state.live, false);
      if (cancellation === "denied")
        assert.equal(f.session.controller.state.rows.length, 0);
    } finally {
      f.release();
      f.session.destroy();
    }
  });

test("managed live recovery stops after a renewed stream expires without a snapshot", async () => {
  const f = fixture();
  try {
    await f.session.refresh();
    void f.session.controller.observe();
    await until(() => f.streams.length === 1);
    f.failRenewedStream();
    f.expire();
    await until(() => f.checks === 2);
    f.release();
    await until(
      () => f.streams.length === 2 && !f.session.controller.state.live,
    );
    assert.equal(f.checks, 2);
    assert.equal(f.session.controller.state.rows.length, 0);
    assert.match(f.session.controller.state.error, /Live query stopped/);
  } finally {
    f.release();
    f.session.destroy();
  }
});

for (const action of ["pause", "stop", "navigation"] as const)
  test(`reentrant ${action} during active publication cancels stale resume`, async () => {
    const f = fixture();
    let unsubscribe = () => {};
    try {
      await f.session.refresh();
      void f.session.controller.observe();
      await until(() => f.streams.length === 1);
      f.expire();
      await until(() => f.checks === 2);
      let armed = false,
        acted = false;
      unsubscribe = f.session.controller.subscribe((state) => {
        if (!armed || acted || state.session?.status !== "active") return;
        acted = true;
        if (action === "pause") f.session.controller.pauseSession("transient");
        if (action === "stop") f.session.controller.stopLive();
        if (action === "navigation")
          void f.session.controller.selectKind("task", { limit: 1 });
      });
      armed = true;
      f.release();
      await until(() => acted);
      await new Promise<void>((resolve) => setTimeout(resolve, 20));
      assert.equal(
        f.streams.length,
        1,
        "stale active callback must not open a stream",
      );
      assert.equal(f.session.controller.state.live, false);
      if (action === "pause")
        assert.equal(f.session.controller.state.session?.status, "transient");
    } finally {
      unsubscribe();
      f.release();
      f.session.destroy();
    }
  });

for (const action of ["pause", "stop", "navigation", "destroy"] as const)
  test(`reentrant transport abort ${action} prevents replacement observation`, async () => {
    const f = fixture();
    try {
      await f.session.refresh();
      void f.session.controller.observe();
      await until(() => f.streams.length === 1);
      f.onAbort(() => {
        if (action === "pause") f.session.controller.pauseSession("transient");
        if (action === "stop") f.session.controller.stopLive();
        if (action === "navigation")
          void f.session.controller.selectKind("task", { limit: 1 });
        if (action === "destroy") f.session.destroy();
      });
      void f.session.controller.observe();
      await new Promise<void>((resolve) => setTimeout(resolve, 20));
      assert.equal(f.streams.length, 1);
      assert.equal(f.session.controller.state.live, false);
      if (action === "pause")
        assert.equal(f.session.controller.state.session?.status, "transient");
    } finally {
      f.onAbort(() => {});
      f.release();
      f.session.destroy();
    }
  });
