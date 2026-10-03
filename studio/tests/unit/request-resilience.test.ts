import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";
const reply = (body: string) =>
  new Response(body, { headers: { "content-type": "application/json" } });
const view = '{"key":{"kind":"task","id":"a"},"revision":7,"value":{}}';
const finite = <T>(p: Promise<T>) =>
  Promise.race([
    p,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(Error("test completion bound")), 100),
    ),
  ]);
test("POST deadline rejects even when transport ignores abort", async () => {
  let release!: (r: Response) => void;
  const c = createClient({
    base: "/api",
    timeoutMs: 5,
    fetch: async () => new Promise((r) => (release = r)),
  });
  const p = c.read("task", "a");
  try {
    await assert.rejects(finite(p), /timeout/);
  } finally {
    release(reply(view));
  }
});
test("already aborted request does not invoke transport", async () => {
  let calls = 0;
  const a = new AbortController();
  a.abort(Error("caller cancelled"));
  const c = createClient({
    base: "/api",
    fetch: async () => {
      calls++;
      return reply(view);
    },
  });
  await assert.rejects(c.read("task", "a", a.signal), /cancelled/);
  assert.equal(calls, 0);
});
test("oversized body rejection does not await transport cancellation", async () => {
  let release!: () => void;
  const stream = new ReadableStream({
    start(c) {
      c.enqueue(new Uint8Array(101));
    },
    cancel() {
      return new Promise<void>((r) => (release = r));
    },
  });
  const c = createClient({
    base: "/api",
    maxBytes: 100,
    fetch: async () =>
      new Response(stream, { headers: { "content-type": "application/json" } }),
  });
  try {
    await assert.rejects(finite(c.read("task", "a")), /bytes limit/);
  } finally {
    release?.();
  }
});
test("query reply obeys explicit page limit", async () => {
  const c = createClient({
    base: "/api",
    maxRows: 10,
    fetch: async () => reply(`[${view},${view.replace('"a"', '"b"')}]`),
  });
  await assert.rejects(c.query("task", { limit: 1 }), /rows limit/);
});
test("mutation reply must match revision and deletion semantics", async () => {
  for (const body of [view.replace('"revision":7', '"revision":3'), view]) {
    const c = createClient({ base: "/api", fetch: async () => reply(body) });
    const p = c.prepare({
      kind: "task",
      id: "a",
      expected: 7n,
      idempotency: "k",
      operation: { type: "delete" },
    });
    await assert.rejects(c.submit(p), /mutation/);
    assert.equal(p.state, "unknown");
  }
});
test("legal no-op revision and deletion are accepted", async () => {
  const c = createClient({
    base: "/api",
    fetch: async () => reply(view.replace('"value":{}', '"value":null')),
  });
  const p = c.prepare({
    kind: "task",
    id: "a",
    expected: 7n,
    idempotency: "k",
    operation: { type: "delete" },
  });
  assert.equal((await c.submit(p)).revision, 7n);
});
test("response reader and parser use the same byte policy", async () => {
  const c = createClient({
    base: "/api",
    maxBytes: 2 * 1024 * 1024,
    fetch: async () =>
      reply(
        view.replace(
          '"value":{}',
          '"value":{"large":"' + "a".repeat(1100000) + '"}',
        ),
      ),
  });
  assert.equal((await c.read("task", "a")).revision, 7n);
});
test("timer values outside execution range are rejected before transport", () => {
  assert.throws(
    () => createClient({ base: "/api", timeoutMs: Number.MAX_SAFE_INTEGER }),
    /timeoutMs/,
  );
});
test("floating wire tokens cannot become integer metadata", async () => {
  const c = createClient({
    base: "/api",
    fetch: async () => reply(view.replace('"revision":7', '"revision":7.0')),
  });
  await assert.rejects(c.read("task", "a"), /integer/);
  const d = createClient({
    base: "/api",
    fetch: async () => reply('{"version":1.0,"resources":[]}'),
  });
  await assert.rejects(d.discover(), /integer|version/);
});

test("request timeout cancels an opened idle response body", async () => {
  let cancellations = 0;
  const c = createClient({
    base: "/api",
    timeoutMs: 5,
    fetch: async () =>
      new Response(
        new ReadableStream({
          cancel() {
            cancellations++;
          },
        }),
        { headers: { "content-type": "application/json" } },
      ),
  });
  await assert.rejects(finite(c.read("task", "a")), /timeout/);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(cancellations, 1);
});
