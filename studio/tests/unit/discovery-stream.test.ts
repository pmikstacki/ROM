import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";
const descriptor = {
  version: 1,
  resources: [
    {
      kind: "task",
      version: 1,
      fields: [{ name: "done", shape: { type: "bool" } }],
      actions: ["finish"],
      action_inputs: [{ name: "finish", version: 1, input: { type: "unit" } }],
    },
  ],
};
test("accepts authorized descriptor and rejects duplicate or inconsistent metadata", async () => {
  let value: unknown = descriptor;
  const c = createClient({
    base: "/api",
    fetch: async () => Response.json(value),
  });
  assert.equal((await c.discover()).resources[0].fields[0].shape.type, "bool");
  value = {
    ...descriptor,
    resources: [descriptor.resources[0], descriptor.resources[0]],
  };
  await assert.rejects(c.discover(), /duplicate/);
  value = {
    ...descriptor,
    resources: [
      {
        ...descriptor.resources[0],
        action_inputs: [{ name: "hidden", version: 1, input: null }],
      },
    ],
  };
  await assert.rejects(c.discover(), /action/);
  value = {
    ...descriptor,
    resources: [
      {
        ...descriptor.resources[0],
        fields: [{ name: "x", shape: { type: "anything" } }],
      },
    ],
  };
  await assert.rejects(c.discover(), /shape/);
});
function eventResponse(parts: string[]) {
  const encoder = new TextEncoder();
  return new Response(
    new ReadableStream({
      start(controller) {
        for (const part of parts) controller.enqueue(encoder.encode(part));
        controller.close();
      },
    }),
    { headers: { "content-type": "text/event-stream" } },
  );
}
const view =
  '{"key":{"kind":"task","id":"a"},"revision":1,"value":{"done":false}}';
test("bounded POST stream handles split CRLF frames and surfaces terminal errors", async () => {
  const c = createClient({
    base: "/api",
    fetch: async (_, init) => {
      assert.equal(init?.method, "POST");
      return eventResponse([
        ":keepalive\r\n\r\nevent: da",
        "ta\r\ndata: [",
        view,
        "]\r\n\r\nevent: error\ndata: denied\n\n",
      ]);
    },
  });
  const stream = c
    .observe("task", {}, new AbortController().signal)
    [Symbol.asyncIterator]();
  assert.equal((await stream.next()).value?.[0].revision, 1n);
  await assert.rejects(stream.next(), /denied/);
});
test("stream rejects oversized frames and a wrong kind before disclosure", async () => {
  let parts = ["event: data\ndata: " + " ".repeat(100) + "\n\n"];
  const c = createClient({
    base: "/api",
    maxBytes: 80,
    fetch: async () => eventResponse(parts),
  });
  await assert.rejects(
    c
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    /limit/,
  );
  parts = ["event: data\ndata: [" + view.replace("task", "other") + "]\n\n"];
  const other = createClient({
    base: "/api",
    fetch: async () => eventResponse(parts),
  });
  await assert.rejects(
    other
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    /identity/,
  );
});

test("idle stream closes its reader on timeout and cancellation", async () => {
  let canceled = 0;
  const fetcher = async () =>
    new Response(
      new ReadableStream({
        cancel() {
          canceled++;
        },
      }),
      { headers: { "content-type": "text/event-stream" } },
    );
  const client = createClient({ base: "/api", timeoutMs: 10, fetch: fetcher });
  const stream = client
    .observe("task", {}, new AbortController().signal)
    [Symbol.asyncIterator]();
  await assert.rejects(
    Promise.race([
      stream.next(),
      new Promise((_, reject) =>
        setTimeout(() => reject(Error("test deadline")), 100),
      ),
    ]),
    /timeout/,
  );
  assert.equal(canceled, 1);
  const abort = new AbortController();
  const next = client
    .observe("task", {}, abort.signal)
    [Symbol.asyncIterator]()
    .next();
  setTimeout(() => abort.abort(Error("caller aborted")), 1);
  await assert.rejects(next, /aborted/);
  assert.equal(canceled, 2);
});
test("stream flushes UTF8 decoder and treats silent EOF as stale", async () => {
  const invalid = createClient({
    base: "/api",
    fetch: async () =>
      new Response(new Uint8Array([0xf0, 0x9f]), {
        headers: { "content-type": "text/event-stream" },
      }),
  });
  await assert.rejects(
    invalid
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
  );
  const empty = createClient({
    base: "/api",
    fetch: async () => eventResponse([]),
  });
  await assert.rejects(
    empty
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    /closed/,
  );
});

test("invalid stream media cancels its body, and opening errors retain server category", async () => {
  let canceled = 0;
  const invalid = createClient({
    base: "/api",
    fetch: async () =>
      new Response(
        new ReadableStream({
          cancel() {
            canceled++;
          },
        }),
        { headers: { "content-type": "text/html" } },
      ),
  });
  await assert.rejects(
    invalid
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    /content/,
  );
  assert.equal(canceled, 1);
  const expired = createClient({
    base: "/api",
    fetch: async () =>
      Response.json({ error: "identity_expired" }, { status: 410 }),
  });
  await assert.rejects(
    expired
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    (error: unknown) =>
      (error as { category: string }).category === "identity_expired",
  );
});

test("live snapshots obey the requested page limit", async () => {
  const c = createClient({
    base: "/api",
    maxRows: 10,
    fetch: async () =>
      eventResponse([
        `event: data\ndata: [${view},${view.replace('"a"', '"b"')}]\n\n`,
      ]),
  });
  await assert.rejects(
    c
      .observe("task", { limit: 1 }, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    /rows limit/,
  );
});
test("observation admission is finite and cancellation releases its slot", async () => {
  const c = createClient({
    base: "/api",
    maxObservations: 1,
    fetch: async () =>
      new Response(
        new ReadableStream({
          start(controller) {
            controller.enqueue(
              new TextEncoder().encode(`event: data\ndata: [${view}]\n\n`),
            );
          },
        }),
        { headers: { "content-type": "text/event-stream" } },
      ),
  });
  const a = new AbortController();
  const first = c.observe("task", {}, a.signal)[Symbol.asyncIterator]();
  await first.next();
  await assert.rejects(
    c
      .observe("task", {}, new AbortController().signal)
      [Symbol.asyncIterator]()
      .next(),
    /observation limit/,
  );
  a.abort(Error("cancelled"));
  await assert.rejects(first.next(), /cancelled/);
  const b = new AbortController();
  const replacement = c.observe("task", {}, b.signal)[Symbol.asyncIterator]();
  assert.equal((await replacement.next()).value?.length, 1);
  b.abort();
  await assert.rejects(replacement.next());
});

test("discovery retains bounded wrapper paths and rejects shape mismatches", async () => {
  let field: unknown = {
    name: "labels",
    shape: {
      type: "list",
      value: { type: "nullable", value: { type: "string" } },
    },
    codec: { name: "custom-label", version: 1 },
    codec_wrappers: ["list", "nullable"],
  };
  const c = createClient({
    base: "/api",
    fetch: async () =>
      Response.json({
        ...descriptor,
        resources: [{ ...descriptor.resources[0], fields: [field] }],
      }),
  });
  assert.deepEqual((await c.discover()).resources[0].fields[0].codec_wrappers, [
    "list",
    "nullable",
  ]);
  field = {
    name: "labels",
    shape: { type: "string" },
    codec: { name: "custom-label", version: 1 },
    codec_wrappers: ["list"],
  };
  await assert.rejects(c.discover(), /wrapper/);
  field = {
    name: "labels",
    shape: { type: "list", value: { type: "string" } },
    codec_wrappers: ["list"],
  };
  await assert.rejects(c.discover(), /wrapper/);
});
