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
